//! Compute shader pipeline — general-purpose GPU compute.

/// A compute pipeline wrapping wgpu::ComputePipeline with buffer management.
pub struct ComputePipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl ComputePipeline {
    /// Create a compute pipeline from WGSL source code.
    ///
    /// `entry_point`: the compute shader entry function name.
    /// `buffer_count`: number of storage buffers in the bind group (bindings 0..n).
    ///
    /// Buffer 0 is created as read-write (`read_only: false`) and buffers 1+
    /// are read-only. This matches the common pattern where a single output
    /// buffer is written by the shader while additional input buffers are
    /// consumed without modification.
    pub fn new(
        device: &wgpu::Device,
        wgsl_source: &str,
        entry_point: &str,
        buffer_count: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_shader"),
            source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
        });

        let entries: Vec<wgpu::BindGroupLayoutEntry> = (0..buffer_count)
            .map(|i| wgpu::BindGroupLayoutEntry {
                binding: i,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: i > 0 },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_layout"),
            entries: &entries,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("compute_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some(entry_point),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    /// Get the bind group layout for creating bind groups.
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Dispatch the compute shader.
    pub fn dispatch(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bind_group: &wgpu::BindGroup,
        workgroups_x: u32,
        workgroups_y: u32,
        workgroups_z: u32,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("compute_encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

/// Helper to create a GPU storage buffer.
pub fn create_storage_buffer(
    device: &wgpu::Device,
    data: &[u8],
    label: &str,
    read_only: bool,
) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;
    let mut usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
    if !read_only {
        usage |= wgpu::BufferUsages::COPY_SRC;
    }
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: data,
        usage,
    })
}

/// Helper to create an empty GPU storage buffer with a given size.
pub fn create_storage_buffer_empty(
    device: &wgpu::Device,
    size: u64,
    label: &str,
    read_only: bool,
) -> wgpu::Buffer {
    let mut usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
    if !read_only {
        usage |= wgpu::BufferUsages::COPY_SRC;
    }
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn compute_pipeline_types() {
        let _size = std::mem::size_of::<super::ComputePipeline>();
    }
}
