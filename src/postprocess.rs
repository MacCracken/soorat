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
    pub fn new(exposure: f32, tone_map: ToneMapMode) -> Self {
        Self {
            params: [exposure, 1.0, 0.0, tone_map as u32 as f32],
        }
    }
}

/// Post-processing pipeline — renders a full-screen quad with tone mapping.
///
/// Uses [`mabda::RenderPipeline`] for pipeline management and
/// [`mabda::create_uniform_buffer`] for uniform buffer creation.
pub struct PostProcessPipeline {
    pipeline: mabda::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: Option<wgpu::BindGroup>,
}

impl PostProcessPipeline {
    /// Create a post-process pipeline for the given output format.
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Result<Self> {
        let pipeline = mabda::RenderPipelineBuilder::new(
            device,
            include_str!("postprocess.wgsl"),
            "vs_main",
            "fs_no_bloom",
        )
        .label("postprocess_pipeline")
        .bind_group(
            mabda::BindGroupLayoutBuilder::new()
                .texture_2d(wgpu::ShaderStages::FRAGMENT)
                .sampler(wgpu::ShaderStages::FRAGMENT)
                .uniform_buffer(wgpu::ShaderStages::FRAGMENT)
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
            uniform_buffer,
            uniform_bind_group: None,
        })
    }

    /// Set the input texture to post-process (call once or when render target changes).
    pub fn set_input(
        &mut self,
        device: &wgpu::Device,
        input_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) {
        self.uniform_bind_group = Some(
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("postprocess_bind_group"),
                layout: self
                    .pipeline
                    .bind_group_layout(0)
                    .expect("postprocess pipeline has bind group 0"),
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
            }),
        );
    }

    /// Update post-process parameters.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &PostProcessUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    /// Render the post-process pass to the output view.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
    ) {
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

            render_pass.set_pipeline(self.pipeline.raw());
            render_pass.set_bind_group(0, bind_group, &[]);
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
    fn tone_map_modes() {
        assert_eq!(ToneMapMode::Reinhard as u32, 0);
        assert_eq!(ToneMapMode::AcesFilmic as u32, 1);
    }
}
