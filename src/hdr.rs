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

    #[must_use]
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        tracing::debug!(width, height, "creating hdr framebuffer");
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
    #[must_use]
    pub fn new(
        threshold: f32,
        soft_threshold: f32,
        intensity: f32,
        width: u32,
        height: u32,
    ) -> Self {
        let texel_size = if width == 0 || height == 0 {
            [0.0, 0.0, 0.0, 0.0]
        } else {
            [1.0 / width as f32, 1.0 / height as f32, 0.0, 0.0]
        };
        Self {
            params: [threshold, soft_threshold, intensity, 0.0],
            texel_size,
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
///
/// Uses [`mabda::RenderPipeline`] for pipeline management and
/// [`mabda::create_uniform_buffer`] for uniform buffer creation.
pub struct BloomPipeline {
    threshold_pipeline: mabda::RenderPipeline,
    blur_h_pipeline: mabda::RenderPipeline,
    blur_v_pipeline: mabda::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
}

impl BloomPipeline {
    /// Create a bloom pipeline. Returns `None` if pipeline creation fails.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Option<Self> {
        tracing::debug!(?format, "creating bloom pipeline");
        let shader_src = include_str!("bloom.wgsl");
        let bind_entries = mabda::BindGroupLayoutBuilder::new()
            .texture_2d(wgpu::ShaderStages::FRAGMENT)
            .sampler(wgpu::ShaderStages::FRAGMENT)
            .uniform_buffer(wgpu::ShaderStages::FRAGMENT)
            .into_entries();

        let make_pipeline =
            |label: &str, fs_entry: &str| -> Result<mabda::RenderPipeline, mabda::GpuError> {
                mabda::RenderPipelineBuilder::new(device, shader_src, "vs_main", fs_entry)
                    .label(label)
                    .bind_group(bind_entries.clone())
                    .color_target(format, None)
                    .build()
            };

        let threshold_pipeline = make_pipeline("bloom_threshold", "fs_threshold").ok()?;
        let blur_h_pipeline = make_pipeline("bloom_blur_h", "fs_blur_h").ok()?;
        let blur_v_pipeline = make_pipeline("bloom_blur_v", "fs_blur_v").ok()?;

        let defaults = BloomUniforms::default();
        let uniform_buffer = mabda::create_uniform_buffer(
            device,
            bytemuck::bytes_of(&defaults),
            "bloom_uniform_buffer",
        );

        Some(Self {
            threshold_pipeline,
            blur_h_pipeline,
            blur_v_pipeline,
            uniform_buffer,
        })
    }

    /// Create a bind group for a bloom pass input.
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        input_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> crate::error::Result<wgpu::BindGroup> {
        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_bind_group"),
            layout: self
                .threshold_pipeline
                .bind_group_layout(0)
                .ok_or_else(|| {
                    crate::error::RenderError::Pipeline(
                        "bloom pipeline missing bind group layout 0".into(),
                    )
                })?,
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
        }))
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
        tracing::debug!("rendering bloom passes");
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
            pass.set_pipeline(self.threshold_pipeline.raw());
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
            pass.set_pipeline(self.blur_h_pipeline.raw());
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
            pass.set_pipeline(self.blur_v_pipeline.raw());
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

    #[test]
    fn bloom_uniforms_zero_dimensions() {
        // Division-by-zero regression: zero width/height must not produce Inf texel_size
        let u = BloomUniforms::new(0.5, 0.5, 1.0, 0, 0);
        assert!(!u.texel_size[0].is_infinite(), "texel_size[0] is Inf");
        assert!(!u.texel_size[1].is_infinite(), "texel_size[1] is Inf");
        assert!(!u.texel_size[0].is_nan(), "texel_size[0] is NaN");
        assert!(!u.texel_size[1].is_nan(), "texel_size[1] is NaN");
        assert_eq!(u.texel_size, [0.0, 0.0, 0.0, 0.0]);
    }
}
