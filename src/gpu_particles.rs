//! GPU particle system — compute shader simulation + instanced quad rendering.

use crate::instancing::InstanceData;
use crate::math_util::IDENTITY_MAT4;

/// A GPU particle (matches the WGSL Particle struct layout).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuParticle {
    /// xyz = position, w = remaining life (seconds, ≤0 = dead).
    pub position: [f32; 4],
    /// xyz = velocity, w = size (world units).
    pub velocity: [f32; 4],
    /// RGBA color.
    pub color: [f32; 4],
}

impl Default for GpuParticle {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0, 1.0],
            velocity: [0.0, 0.0, 0.0, 0.1],
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// Simulation parameters uniform.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimParams {
    pub delta_time: f32,
    pub gravity_y: f32,
    pub damping: f32,
    pub particle_count: u32,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            delta_time: 1.0 / 60.0,
            gravity_y: -9.81,
            damping: 0.98,
            particle_count: 0,
        }
    }
}

/// GPU particle system — manages compute simulation + instance buffer for rendering.
pub struct GpuParticleSystem {
    compute_pipeline: wgpu::ComputePipeline,
    particle_buffer: wgpu::Buffer,
    params_buffer: wgpu::Buffer,
    compute_bind_group: wgpu::BindGroup,
    pub particle_count: u32,
    capacity: u32,
}

impl GpuParticleSystem {
    /// Create a particle system with the given capacity.
    pub fn new(device: &wgpu::Device, capacity: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpu_particles_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_particles.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("particle_compute_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("particle_compute_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("particle_compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("cs_update"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle_buffer"),
            size: (capacity as u64) * std::mem::size_of::<GpuParticle>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle_params_buffer"),
            size: std::mem::size_of::<SimParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle_compute_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            compute_pipeline,
            particle_buffer,
            params_buffer,
            compute_bind_group,
            particle_count: 0,
            capacity,
        }
    }

    /// Upload initial particle data.
    pub fn upload(&mut self, queue: &wgpu::Queue, particles: &[GpuParticle]) {
        self.particle_count = (particles.len() as u32).min(self.capacity);
        queue.write_buffer(
            &self.particle_buffer,
            0,
            bytemuck::cast_slice(&particles[..self.particle_count as usize]),
        );
    }

    /// Run the compute simulation step.
    pub fn simulate(&self, device: &wgpu::Device, queue: &wgpu::Queue, delta_time: f32) {
        let params = SimParams {
            delta_time,
            particle_count: self.particle_count,
            ..Default::default()
        };
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("particle_compute_encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("particle_compute_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.compute_pipeline);
            pass.set_bind_group(0, &self.compute_bind_group, &[]);
            let workgroups = self.particle_count.div_ceil(64);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }

    /// Generate instance data from current particle state for rendering.
    /// Call after `simulate()`. Reads particle buffer back to CPU.
    /// For high-performance, use a GPU-only path with indirect draw instead.
    pub fn to_instance_data(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<InstanceData> {
        // For now, use a staging buffer readback
        let size = (self.particle_count as u64) * std::mem::size_of::<GpuParticle>() as u64;
        if size == 0 {
            return Vec::new();
        }

        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle_readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("particle_readback_encoder"),
        });
        encoder.copy_buffer_to_buffer(&self.particle_buffer, 0, &staging, 0, size);
        queue.submit(std::iter::once(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);

        if rx.recv().unwrap().is_err() {
            return Vec::new();
        }

        let data = slice.get_mapped_range();
        let particles: &[GpuParticle] = bytemuck::cast_slice(&data);

        let instances: Vec<InstanceData> = particles
            .iter()
            .filter(|p| p.position[3] > 0.0) // alive
            .map(|p| {
                let mut model = IDENTITY_MAT4;
                model[12] = p.position[0];
                model[13] = p.position[1];
                model[14] = p.position[2];
                let s = p.velocity[3]; // size in w
                model[0] = s;
                model[5] = s;
                model[10] = s;
                InstanceData {
                    model,
                    color: p.color,
                }
            })
            .collect();

        drop(data);
        staging.unmap();

        instances
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_particle_size() {
        assert_eq!(std::mem::size_of::<GpuParticle>(), 48); // 3 * vec4
    }

    #[test]
    fn sim_params_size() {
        assert_eq!(std::mem::size_of::<SimParams>(), 16); // 3 f32 + 1 u32
    }

    #[test]
    fn gpu_particle_default() {
        let p = GpuParticle::default();
        assert_eq!(p.position[3], 1.0); // alive
        assert_eq!(p.color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn sim_params_default() {
        let p = SimParams::default();
        assert!(p.gravity_y < 0.0);
        assert!(p.damping > 0.0 && p.damping < 1.0);
    }

    #[test]
    fn gpu_particle_bytemuck() {
        let p = GpuParticle::default();
        let bytes = bytemuck::bytes_of(&p);
        assert_eq!(bytes.len(), 48);
    }
}
