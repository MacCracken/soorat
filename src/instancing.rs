//! Instanced rendering — draw many copies of a mesh with per-instance transforms.

use wgpu::util::DeviceExt;

/// Per-instance data: model matrix (column-major 4x4) + color tint.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    /// Model matrix (column-major 4x4).
    pub model: [f32; 16],
    /// Color tint (RGBA, multiplied with vertex/texture color).
    pub color: [f32; 4],
}

impl Default for InstanceData {
    fn default() -> Self {
        Self {
            model: crate::math_util::IDENTITY_MAT4,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

impl InstanceData {
    /// Create instance data from a translation.
    pub fn from_translation(x: f32, y: f32, z: f32) -> Self {
        let mut model = crate::math_util::IDENTITY_MAT4;
        model[12] = x;
        model[13] = y;
        model[14] = z;
        Self {
            model,
            ..Default::default()
        }
    }

    /// wgpu vertex buffer layout for instance data (step mode = Instance).
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // model matrix — 4 vec4 columns at locations 7-10
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 32,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 48,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // color tint
                wgpu::VertexAttribute {
                    offset: 64,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Instance buffer — holds per-instance data on the GPU.
pub struct InstanceBuffer {
    pub buffer: wgpu::Buffer,
    pub count: u32,
    capacity: usize,
}

impl InstanceBuffer {
    /// Create an instance buffer from a slice of instance data.
    pub fn new(device: &wgpu::Device, instances: &[InstanceData]) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance_buffer"),
            contents: bytemuck::cast_slice(instances),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            buffer,
            count: instances.len() as u32,
            capacity: instances.len(),
        }
    }

    /// Create an empty instance buffer with pre-allocated capacity.
    pub fn with_capacity(device: &wgpu::Device, capacity: usize) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            size: (capacity * std::mem::size_of::<InstanceData>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            count: 0,
            capacity,
        }
    }

    /// Update instance data. Regrows buffer if needed.
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[InstanceData],
    ) {
        if instances.len() > self.capacity {
            // Exponential growth to avoid reallocating on every single-instance addition.
            self.capacity = (instances.len() * 3 / 2).max(16);
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instance_buffer"),
                size: (self.capacity * std::mem::size_of::<InstanceData>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(instances));
        self.count = instances.len() as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_data_size() {
        // mat4 + vec4 = 64 + 16 = 80
        assert_eq!(std::mem::size_of::<InstanceData>(), 80);
    }

    #[test]
    fn instance_data_default() {
        let d = InstanceData::default();
        assert_eq!(d.model[0], 1.0); // identity
        assert_eq!(d.color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn instance_data_translation() {
        let d = InstanceData::from_translation(5.0, 3.0, 1.0);
        assert_eq!(d.model[12], 5.0);
        assert_eq!(d.model[13], 3.0);
        assert_eq!(d.model[14], 1.0);
    }

    #[test]
    fn instance_data_layout() {
        let layout = InstanceData::layout();
        assert_eq!(layout.array_stride, 80);
        assert_eq!(layout.attributes.len(), 5); // 4 mat4 cols + color
        assert_eq!(layout.step_mode, wgpu::VertexStepMode::Instance);
    }

    #[test]
    fn instance_data_bytemuck() {
        let d = InstanceData::default();
        let bytes = bytemuck::bytes_of(&d);
        assert_eq!(bytes.len(), 80);
    }

    #[test]
    fn instance_data_batch_cast() {
        let instances = vec![
            InstanceData::from_translation(0.0, 0.0, 0.0),
            InstanceData::from_translation(1.0, 0.0, 0.0),
            InstanceData::from_translation(2.0, 0.0, 0.0),
        ];
        let bytes: &[u8] = bytemuck::cast_slice(&instances);
        assert_eq!(bytes.len(), 80 * 3);
    }
}
