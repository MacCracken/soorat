//! Post-processing pipeline — tone mapping, bloom.

use crate::error::Result;

/// Tone mapping mode.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneMapMode {
    Reinhard = 0,
    AcesFilmic = 1,
}

/// Post-process uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PostProcessUniforms {
    /// x = exposure, y = bloom_threshold, z = bloom_intensity, w = tone_map_mode
    pub params: [f32; 4],
}

impl Default for PostProcessUniforms {
    fn default() -> Self {
        Self {
            params: [1.0, 1.0, 0.0, ToneMapMode::Reinhard as u32 as f32],
        }
    }
}

impl PostProcessUniforms {
    #[must_use]
    pub fn new(exposure: f32, tone_map: ToneMapMode) -> Self {
        Self {
            params: [exposure, 1.0, 0.0, tone_map as u32 as f32],
        }
    }

    /// Set the bloom intensity (`params.z`): how strongly a texture bound with
    /// [`PostProcessPipeline::set_bloom`] is added before exposure and tone
    /// mapping. Negative or non-finite values become 0 (no bloom).
    #[must_use]
    pub fn with_bloom_intensity(mut self, intensity: f32) -> Self {
        self.params[2] = if intensity.is_finite() {
            intensity.max(0.0)
        } else {
            0.0
        };
        self
    }
}

/// Post-processing pipeline — renders a full-screen quad with tone mapping,
/// optionally compositing a bloom texture first.
///
/// Uses [`mabda::RenderPipeline`] for pipeline management and
/// [`mabda::create_uniform_buffer`] for uniform buffer creation.
pub struct PostProcessPipeline {
    /// Tone mapping only (`fs_no_bloom`, bind group 0).
    pipeline: mabda::RenderPipeline,
    /// Bloom composite + tone mapping (`fs_main`, bind groups 0 and 1).
    bloom_pipeline: mabda::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: Option<wgpu::BindGroup>,
    /// Group 0 (input + uniforms) built against `bloom_pipeline`'s layout.
    bloom_input_bind_group: Option<wgpu::BindGroup>,
    /// Group 1 (bloom texture), set by [`Self::set_bloom`].
    bloom_bind_group: Option<wgpu::BindGroup>,
}

impl PostProcessPipeline {
    /// Create a post-process pipeline for the given output format.
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Result<Self> {
        tracing::debug!(?output_format, "creating postprocess pipeline");
        let input_entries = || {
            mabda::BindGroupLayoutBuilder::new()
                .texture_2d(wgpu::ShaderStages::FRAGMENT)
                .sampler(wgpu::ShaderStages::FRAGMENT)
                .uniform_buffer(wgpu::ShaderStages::FRAGMENT)
                .into_entries()
        };
        let pipeline = mabda::RenderPipelineBuilder::new(
            device,
            include_str!("postprocess.wgsl"),
            "vs_main",
            "fs_no_bloom",
        )
        .label("postprocess_pipeline")
        .bind_group(input_entries())
        .color_target(output_format, None)
        .build()?;
        let bloom_pipeline = mabda::RenderPipelineBuilder::new(
            device,
            include_str!("postprocess.wgsl"),
            "vs_main",
            "fs_main",
        )
        .label("postprocess_bloom_pipeline")
        .bind_group(input_entries())
        .bind_group(
            mabda::BindGroupLayoutBuilder::new()
                .texture_2d(wgpu::ShaderStages::FRAGMENT)
                .sampler(wgpu::ShaderStages::FRAGMENT)
                .into_entries(),
        )
        .color_target(output_format, None)
        .build()?;

        let defaults = PostProcessUniforms::default();
        let uniform_buffer = mabda::create_uniform_buffer(
            device,
            bytemuck::bytes_of(&defaults),
            "postprocess_uniform_buffer",
        );

        Ok(Self {
            pipeline,
            bloom_pipeline,
            uniform_buffer,
            uniform_bind_group: None,
            bloom_input_bind_group: None,
            bloom_bind_group: None,
        })
    }

    /// Group 0 (input texture, sampler, uniforms) for `pipeline`'s layout.
    fn input_bind_group(
        &self,
        device: &wgpu::Device,
        pipeline: &mabda::RenderPipeline,
        input_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> Result<wgpu::BindGroup> {
        Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("postprocess_bind_group"),
            layout: pipeline.bind_group_layout(0).ok_or_else(|| {
                crate::error::RenderError::Pipeline(
                    "postprocess pipeline missing bind group layout 0".into(),
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

    /// Set the input texture to post-process (call once or when render target changes).
    pub fn set_input(
        &mut self,
        device: &wgpu::Device,
        input_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> Result<()> {
        let plain = self.input_bind_group(device, &self.pipeline, input_view, sampler)?;
        let bloom = self.input_bind_group(device, &self.bloom_pipeline, input_view, sampler)?;
        self.uniform_bind_group = Some(plain);
        self.bloom_input_bind_group = Some(bloom);
        Ok(())
    }

    /// Composite `bloom_view` (e.g. [`crate::hdr::BloomPipeline`]'s output)
    /// into every following [`Self::render`], scaled by the bloom intensity in
    /// [`PostProcessUniforms`] (see [`PostProcessUniforms::with_bloom_intensity`]).
    pub fn set_bloom(
        &mut self,
        device: &wgpu::Device,
        bloom_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> Result<()> {
        self.bloom_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("postprocess_bloom_bind_group"),
            layout: self.bloom_pipeline.bind_group_layout(1).ok_or_else(|| {
                crate::error::RenderError::Pipeline(
                    "postprocess bloom pipeline missing bind group layout 1".into(),
                )
            })?,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(bloom_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        }));
        Ok(())
    }

    /// Stop compositing bloom: [`Self::render`] goes back to tone mapping only.
    pub fn clear_bloom(&mut self) {
        self.bloom_bind_group = None;
    }

    /// Update post-process parameters.
    #[inline]
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &PostProcessUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    /// Render the post-process pass to the output view. With a bloom texture
    /// set ([`Self::set_bloom`]) it is composited before tone mapping.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
    ) {
        let bloom = self
            .bloom_input_bind_group
            .as_ref()
            .zip(self.bloom_bind_group.as_ref());
        tracing::debug!(bloom = bloom.is_some(), "rendering postprocess pass");
        let Some(bind_group) = &self.uniform_bind_group else {
            return;
        };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("postprocess_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("postprocess_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            if let Some((input, bloom)) = bloom {
                render_pass.set_pipeline(self.bloom_pipeline.raw());
                render_pass.set_bind_group(0, input, &[]);
                render_pass.set_bind_group(1, bloom, &[]);
            } else {
                render_pass.set_pipeline(self.pipeline.raw());
                render_pass.set_bind_group(0, bind_group, &[]);
            }
            render_pass.draw(0..3, 0..1); // full-screen triangle
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postprocess_uniforms_size() {
        assert_eq!(std::mem::size_of::<PostProcessUniforms>(), 16);
    }

    #[test]
    fn postprocess_uniforms_default() {
        let u = PostProcessUniforms::default();
        assert_eq!(u.params[0], 1.0); // exposure
        assert_eq!(u.params[3], 0.0); // Reinhard
    }

    #[test]
    fn postprocess_uniforms_aces() {
        let u = PostProcessUniforms::new(2.0, ToneMapMode::AcesFilmic);
        assert_eq!(u.params[0], 2.0);
        assert_eq!(u.params[3], 1.0); // ACES
    }

    #[test]
    fn postprocess_uniforms_bytemuck() {
        let u = PostProcessUniforms::default();
        let bytes = bytemuck::bytes_of(&u);
        assert_eq!(bytes.len(), 16);
    }

    #[test]
    fn bloom_intensity_builder_sanitizes() {
        let u = PostProcessUniforms::default().with_bloom_intensity(0.75);
        assert_eq!(u.params[2], 0.75);
        assert_eq!(u.params[0], 1.0); // exposure untouched
        assert_eq!(
            PostProcessUniforms::default()
                .with_bloom_intensity(-1.0)
                .params[2],
            0.0
        );
        assert_eq!(
            PostProcessUniforms::default()
                .with_bloom_intensity(f32::NAN)
                .params[2],
            0.0
        );
        assert_eq!(
            PostProcessUniforms::default()
                .with_bloom_intensity(f32::INFINITY)
                .params[2],
            0.0
        );
    }

    #[test]
    fn tone_map_modes() {
        assert_eq!(ToneMapMode::Reinhard as u32, 0);
        assert_eq!(ToneMapMode::AcesFilmic as u32, 1);
    }
}
