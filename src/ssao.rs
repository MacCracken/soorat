//! Screen-Space Ambient Occlusion (SSAO).

/// SSAO uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsaoUniforms {
    /// x = radius, y = bias, z = intensity, w = sample_count
    pub params: [f32; 4],
    /// Camera projection matrix (for reprojection).
    pub projection: [f32; 16],
    /// Inverse projection matrix (for depth → view-space).
    pub inv_projection: [f32; 16],
}

impl Default for SsaoUniforms {
    fn default() -> Self {
        Self {
            params: [0.5, 0.025, 1.0, 16.0],
            projection: crate::math_util::IDENTITY_MAT4,
            inv_projection: crate::math_util::IDENTITY_MAT4,
        }
    }
}

impl SsaoUniforms {
    pub fn new(radius: f32, bias: f32, intensity: f32, sample_count: u32) -> Self {
        Self {
            params: [radius, bias, intensity, sample_count as f32],
            ..Default::default()
        }
    }
}

/// SSAO pipeline — renders screen-space ambient occlusion to a single-channel texture.
/// Input: depth buffer + normal buffer. Output: occlusion texture (R channel, 0=occluded, 1=open).
///
/// Uses [`mabda::RenderPipeline`] for pipeline management and
/// [`mabda::create_uniform_buffer`] for uniform buffer creation.
pub struct SsaoPipeline {
    pipeline: mabda::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
}

impl SsaoPipeline {
    /// Create an SSAO pipeline. Output format is typically R8Unorm or R16Float.
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> mabda::Result<Self> {
        let pipeline = mabda::RenderPipelineBuilder::new(
            device,
            include_str!("ssao.wgsl"),
            "vs_main",
            "fs_main",
        )
        .label("ssao_pipeline")
        .bind_group(
            mabda::BindGroupLayoutBuilder::new()
                .texture_depth_2d(wgpu::ShaderStages::FRAGMENT)
                .sampler(wgpu::ShaderStages::FRAGMENT)
                .texture_2d(wgpu::ShaderStages::FRAGMENT)
                .sampler(wgpu::ShaderStages::FRAGMENT)
                .uniform_buffer(wgpu::ShaderStages::FRAGMENT)
                .into_entries(),
        )
        .color_target(output_format, None)
        .build()?;

        let defaults = SsaoUniforms::default();
        let uniform_buffer = mabda::create_uniform_buffer(
            device,
            bytemuck::bytes_of(&defaults),
            "ssao_uniform_buffer",
        );

        Ok(Self {
            pipeline,
            uniform_buffer,
        })
    }

    /// Update SSAO uniforms (radius, bias, intensity, sample count, projection matrices).
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &SsaoUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    /// Create a bind group for SSAO inputs.
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        depth_view: &wgpu::TextureView,
        depth_sampler: &wgpu::Sampler,
        normal_view: &wgpu::TextureView,
        normal_sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssao_bind_group"),
            layout: self
                .pipeline
                .bind_group_layout(0)
                .expect("ssao pipeline has bind group 0"),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(depth_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(normal_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Render SSAO to an output texture.
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        output_view: &wgpu::TextureView,
        bind_group: &wgpu::BindGroup,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ssao_encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ssao_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(self.pipeline.raw());
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssao_uniforms_size() {
        // vec4 + 2 * mat4 = 16 + 128 = 144
        assert_eq!(std::mem::size_of::<SsaoUniforms>(), 144);
    }

    #[test]
    fn ssao_uniforms_default() {
        let u = SsaoUniforms::default();
        assert_eq!(u.params[0], 0.5); // radius
        assert_eq!(u.params[3], 16.0); // sample_count
    }

    #[test]
    fn ssao_uniforms_new() {
        let u = SsaoUniforms::new(1.0, 0.01, 2.0, 32);
        assert_eq!(u.params[0], 1.0);
        assert_eq!(u.params[3], 32.0);
    }

    #[test]
    fn ssao_uniforms_bytemuck() {
        let u = SsaoUniforms::default();
        let bytes = bytemuck::bytes_of(&u);
        assert_eq!(bytes.len(), 144);
    }
}
