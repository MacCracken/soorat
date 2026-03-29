//! HDR framebuffer and post-processing chain.

/// HDR render target — Rgba16Float for linear HDR rendering.
/// Scene renders to this, then post-processing reads from it.
pub struct HdrFramebuffer {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
}

impl HdrFramebuffer {
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hdr_framebuffer"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("hdr_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            width,
            height,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        *self = Self::new(device, width, height);
    }
}

/// Bloom uniforms for the bloom shader passes.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BloomUniforms {
    /// x = threshold, y = soft_threshold, z = intensity, w = unused
    pub params: [f32; 4],
    /// x = texel_width, y = texel_height
    pub texel_size: [f32; 4],
}

impl BloomUniforms {
    pub fn new(
        threshold: f32,
        soft_threshold: f32,
        intensity: f32,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            params: [threshold, soft_threshold, intensity, 0.0],
            texel_size: [1.0 / width as f32, 1.0 / height as f32, 0.0, 0.0],
        }
    }
}

impl Default for BloomUniforms {
    fn default() -> Self {
        Self::new(1.0, 0.5, 0.3, 1920, 1080)
    }
}

/// Intermediate targets for the bloom pipeline passes.
pub struct BloomTargets<'a> {
    pub hdr_bind_group: &'a wgpu::BindGroup,
    pub bright_view: &'a wgpu::TextureView,
    pub blur_temp_view: &'a wgpu::TextureView,
    pub bloom_output_view: &'a wgpu::TextureView,
    pub bright_bind_group: &'a wgpu::BindGroup,
    pub blur_temp_bind_group: &'a wgpu::BindGroup,
}

/// Bloom pipeline — orchestrates threshold extraction + separable Gaussian blur.
/// Requires 2 intermediate textures (same format as HDR framebuffer).
pub struct BloomPipeline {
    threshold_pipeline: wgpu::RenderPipeline,
    blur_h_pipeline: wgpu::RenderPipeline,
    blur_v_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl BloomPipeline {
    #[must_use]
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bloom.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
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
            label: Some("bloom_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let make_pipeline = |label: &str, entry: &str| -> wgpu::RenderPipeline {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        let threshold_pipeline = make_pipeline("bloom_threshold", "fs_threshold");
        let blur_h_pipeline = make_pipeline("bloom_blur_h", "fs_blur_h");
        let blur_v_pipeline = make_pipeline("bloom_blur_v", "fs_blur_v");

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloom_uniform_buffer"),
            size: std::mem::size_of::<BloomUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            threshold_pipeline,
            blur_h_pipeline,
            blur_v_pipeline,
            uniform_buffer,
            bind_group_layout,
        }
    }

    /// Create a bind group for a bloom pass input.
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        input_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(input_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Update bloom uniforms.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &BloomUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    /// Run the full bloom pipeline: threshold → blur_h → blur_v.
    /// `hdr_input`: the HDR scene texture bind group.
    /// `bright_target`: intermediate texture for threshold output / blur input.
    /// `blur_target`: intermediate texture for blur H output.
    /// `final_target`: output view for the final blurred bloom.
    /// Run the full bloom pipeline: threshold → blur_h → blur_v.
    pub fn render(&self, device: &wgpu::Device, queue: &wgpu::Queue, targets: &BloomTargets<'_>) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("bloom_encoder"),
        });

        // Pass 1: Threshold — HDR → bright_target
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_threshold_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: targets.bright_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.threshold_pipeline);
            pass.set_bind_group(0, targets.hdr_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // Pass 2: Horizontal blur — bright_target → blur_temp
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_blur_h_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: targets.blur_temp_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.blur_h_pipeline);
            pass.set_bind_group(0, targets.bright_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        // Pass 3: Vertical blur — blur_temp → bloom_output
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_blur_v_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: targets.bloom_output_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.blur_v_pipeline);
            pass.set_bind_group(0, targets.blur_temp_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdr_format() {
        assert_eq!(HdrFramebuffer::FORMAT, wgpu::TextureFormat::Rgba16Float);
    }

    #[test]
    fn bloom_uniforms_size() {
        assert_eq!(std::mem::size_of::<BloomUniforms>(), 32);
    }

    #[test]
    fn bloom_uniforms_default() {
        let u = BloomUniforms::default();
        assert_eq!(u.params[0], 1.0); // threshold
        assert_eq!(u.params[2], 0.3); // intensity
    }

    #[test]
    fn bloom_uniforms_texel_size() {
        let u = BloomUniforms::new(1.0, 0.5, 0.3, 1920, 1080);
        assert!((u.texel_size[0] - 1.0 / 1920.0).abs() < 0.0001);
        assert!((u.texel_size[1] - 1.0 / 1080.0).abs() < 0.0001);
    }

    #[test]
    fn bloom_uniforms_bytemuck() {
        let u = BloomUniforms::default();
        let bytes = bytemuck::bytes_of(&u);
        assert_eq!(bytes.len(), 32);
    }
}
