//! Shadow mapping for directional lights.

use crate::math_util::{IDENTITY_MAT4, cross, mul_mat4, normalize3};
use crate::mesh_pipeline::{DepthBuffer, Mesh};
use crate::vertex::Vertex3D;

/// Default shadow map resolution.
pub const DEFAULT_SHADOW_MAP_SIZE: u32 = 2048;

/// Shadow map — a depth texture rendered from the light's perspective.
pub struct ShadowMap {
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: u32,
}

impl ShadowMap {
    /// Create a shadow map with the given resolution.
    pub fn new(device: &wgpu::Device, size: u32) -> Self {
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow_map"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DepthBuffer::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Comparison sampler for shadow testing
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        Self {
            depth_texture,
            depth_view,
            sampler,
            size,
        }
    }
}

/// Light-space uniforms for the shadow pass.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShadowUniforms {
    /// Light view-projection matrix.
    pub light_view_proj: [f32; 16],
    /// Model matrix.
    pub model: [f32; 16],
}

impl Default for ShadowUniforms {
    fn default() -> Self {
        Self {
            light_view_proj: IDENTITY_MAT4,
            model: IDENTITY_MAT4,
        }
    }
}

/// Compute an orthographic light-space matrix for a directional light.
/// `direction`: normalized light direction (where light points).
/// `extent`: half-size of the shadow frustum in world units.
/// `near`/`far`: depth range.
pub fn directional_light_matrix(
    direction: [f32; 3],
    extent: f32,
    near: f32,
    far: f32,
) -> [f32; 16] {
    // Build a view matrix looking along the light direction
    let d = normalize3(direction);

    // Choose an up vector that isn't parallel to direction
    let up = if d[1].abs() > 0.99 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };

    let right = normalize3(cross(up, d));
    let actual_up = cross(d, right);

    // View matrix (look-at from origin along direction)
    // Column-major: right, up, forward, translation
    let view = [
        right[0],
        actual_up[0],
        d[0],
        0.0,
        right[1],
        actual_up[1],
        d[1],
        0.0,
        right[2],
        actual_up[2],
        d[2],
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ];

    // Orthographic projection
    let l = -extent;
    let r = extent;
    let b = -extent;
    let t = extent;
    let proj = [
        2.0 / (r - l),
        0.0,
        0.0,
        0.0,
        0.0,
        2.0 / (t - b),
        0.0,
        0.0,
        0.0,
        0.0,
        1.0 / (far - near),
        0.0,
        -(r + l) / (r - l),
        -(t + b) / (t - b),
        -near / (far - near),
        1.0,
    ];

    mul_mat4(proj, view)
}

/// Shadow pass pipeline — renders depth-only from the light's perspective.
pub struct ShadowPipeline {
    render_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
}

impl ShadowPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow_uniform_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex3D::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: None, // depth-only
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Front), // front-face culling reduces shadow acne
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DepthBuffer::FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow_uniform_buffer"),
            size: std::mem::size_of::<ShadowUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_uniform_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            render_pipeline,
            uniform_buffer,
            uniform_bind_group,
        }
    }

    /// Update the shadow pass uniforms.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &ShadowUniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    /// Render a mesh into the shadow map (depth-only pass).
    pub fn render_shadow(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        shadow_map: &ShadowMap,
        meshes: &[&Mesh],
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("shadow_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &shadow_map.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            for mesh in meshes {
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_uniforms_size() {
        assert_eq!(std::mem::size_of::<ShadowUniforms>(), 128);
    }

    #[test]
    fn shadow_uniforms_default() {
        let u = ShadowUniforms::default();
        assert_eq!(u.light_view_proj[0], 1.0);
        assert_eq!(u.light_view_proj[15], 1.0);
    }

    #[test]
    fn shadow_uniforms_bytemuck() {
        let u = ShadowUniforms::default();
        let bytes = bytemuck::bytes_of(&u);
        assert_eq!(bytes.len(), 128);
    }

    #[test]
    fn directional_light_matrix_produces_valid() {
        let m = directional_light_matrix([0.0, -1.0, 0.0], 10.0, 0.1, 50.0);
        assert_eq!(m.len(), 16);
        assert!(m != IDENTITY_MAT4);
    }

    #[test]
    fn directional_light_matrix_different_directions() {
        let m1 = directional_light_matrix([0.0, -1.0, 0.0], 10.0, 0.1, 50.0);
        let m2 = directional_light_matrix([1.0, 0.0, 0.0], 10.0, 0.1, 50.0);
        assert!(
            m1 != m2,
            "Different directions should produce different matrices"
        );
    }

    #[test]
    fn directional_light_matrix_diagonal_direction() {
        // Ensure the near-parallel-to-up case works
        let m = directional_light_matrix([0.0, -0.999, -0.01], 20.0, 1.0, 100.0);
        // Should not be NaN
        for &v in &m {
            assert!(!v.is_nan(), "Matrix contains NaN");
        }
    }

    #[test]
    fn default_shadow_map_size() {
        assert_eq!(DEFAULT_SHADOW_MAP_SIZE, 2048);
    }
}
