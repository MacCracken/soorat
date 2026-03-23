//! Sprite rendering pipeline.

use crate::color::Color;
use crate::error::Result;
use crate::sprite::SpriteBatch;
use crate::vertex::Vertex2D;

/// Orthographic projection matrix for 2D rendering.
/// Maps screen coordinates (0,0 top-left) to clip space.
fn orthographic_projection(width: f32, height: f32) -> [f32; 16] {
    let l = 0.0;
    let r = width;
    let t = 0.0;
    let b = height;
    let n = -1.0;
    let f = 1.0;

    // Column-major 4x4 orthographic matrix (right-handed, zero-to-one depth)
    [
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
        1.0 / (n - f),
        0.0,
        -(r + l) / (r - l),
        -(t + b) / (t - b),
        n / (n - f),
        1.0,
    ]
}

/// Quad indices for a single sprite (two triangles).
const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

/// Expand a sprite batch into vertex and index data for GPU upload.
pub fn batch_to_vertices(batch: &SpriteBatch) -> (Vec<Vertex2D>, Vec<u16>) {
    let sprite_count = batch.sprites.len();
    let mut vertices = Vec::with_capacity(sprite_count * 4);
    let mut indices = Vec::with_capacity(sprite_count * 6);

    for (i, sprite) in batch.sprites.iter().enumerate() {
        let c = sprite.color.to_array();
        let base = (i * 4) as u16;

        // Quad corners: top-left, top-right, bottom-right, bottom-left
        vertices.push(Vertex2D {
            position: [sprite.x, sprite.y],
            tex_coords: [0.0, 0.0],
            color: c,
        });
        vertices.push(Vertex2D {
            position: [sprite.x + sprite.width, sprite.y],
            tex_coords: [1.0, 0.0],
            color: c,
        });
        vertices.push(Vertex2D {
            position: [sprite.x + sprite.width, sprite.y + sprite.height],
            tex_coords: [1.0, 1.0],
            color: c,
        });
        vertices.push(Vertex2D {
            position: [sprite.x, sprite.y + sprite.height],
            tex_coords: [0.0, 1.0],
            color: c,
        });

        for &idx in &QUAD_INDICES {
            indices.push(base + idx);
        }
    }

    (vertices, indices)
}

/// Sprite rendering pipeline — holds the wgpu pipeline, bind group layouts, and buffers.
pub struct SpritePipeline {
    render_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl SpritePipeline {
    /// Create a new sprite pipeline for the given surface format.
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
        });

        // Uniform bind group layout (group 0): projection matrix
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sprite_uniform_layout"),
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

        // Texture bind group layout (group 1): texture + sampler
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sprite_texture_layout"),
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
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite_pipeline_layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex2D::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create uniform buffer with identity projection (updated per frame)
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_uniform_buffer"),
            size: 64, // mat4x4<f32> = 16 * 4 bytes
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite_uniform_bind_group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Ok(Self {
            render_pipeline,
            uniform_buffer,
            uniform_bind_group,
            texture_bind_group_layout,
        })
    }

    /// Get the texture bind group layout for creating texture bind groups.
    pub fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }

    /// Update the projection matrix for the current viewport size.
    pub fn update_projection(&self, queue: &wgpu::Queue, width: f32, height: f32) {
        let proj = orthographic_projection(width, height);
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&proj));
    }

    /// Draw a sprite batch. The batch should already be sorted by z-order.
    ///
    /// `clear_color`: if Some, clears the render target first.
    /// `texture_bind_group`: the bind group for the texture to use.
    pub fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        batch: &SpriteBatch,
        texture_bind_group: &wgpu::BindGroup,
        clear_color: Option<Color>,
    ) {
        if batch.is_empty() && clear_color.is_none() {
            return;
        }

        let (vertices, indices) = batch_to_vertices(batch);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite_index_buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("sprite_encoder"),
        });

        {
            let load_op = match clear_color {
                Some(c) => wgpu::LoadOp::Clear(c.to_wgpu()),
                None => wgpu::LoadOp::Load,
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sprite_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            if !batch.is_empty() {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_bind_group(1, texture_bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
    }
}

use wgpu::util::DeviceExt;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::Sprite;

    #[test]
    fn orthographic_projection_identity_check() {
        let proj = orthographic_projection(800.0, 600.0);
        // Should be 16 floats
        assert_eq!(proj.len(), 16);
        // Top-left (0,0) should map to clip (-1, 1)
        // proj[0] = 2/800, proj[12] = -1 => x=0 maps to -1
        assert!((proj[0] - 2.0 / 800.0).abs() < f32::EPSILON);
        assert!((proj[12] - (-1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn batch_to_vertices_empty() {
        let batch = SpriteBatch::new();
        let (verts, indices) = batch_to_vertices(&batch);
        assert!(verts.is_empty());
        assert!(indices.is_empty());
    }

    #[test]
    fn batch_to_vertices_single_sprite() {
        let mut batch = SpriteBatch::new();
        batch.push(Sprite::new(10.0, 20.0, 32.0, 32.0));
        let (verts, indices) = batch_to_vertices(&batch);
        assert_eq!(verts.len(), 4);
        assert_eq!(indices.len(), 6);
        assert_eq!(indices, vec![0, 1, 2, 2, 3, 0]);
        // Check corners
        assert_eq!(verts[0].position, [10.0, 20.0]);
        assert_eq!(verts[1].position, [42.0, 20.0]);
        assert_eq!(verts[2].position, [42.0, 52.0]);
        assert_eq!(verts[3].position, [10.0, 52.0]);
    }

    #[test]
    fn batch_to_vertices_multiple_sprites() {
        let mut batch = SpriteBatch::new();
        batch.push(Sprite::new(0.0, 0.0, 10.0, 10.0));
        batch.push(Sprite::new(50.0, 50.0, 20.0, 20.0));
        let (verts, indices) = batch_to_vertices(&batch);
        assert_eq!(verts.len(), 8);
        assert_eq!(indices.len(), 12);
        // Second sprite indices should be offset by 4
        assert_eq!(indices[6..], [4, 5, 6, 6, 7, 4]);
    }

    #[test]
    fn batch_to_vertices_preserves_color() {
        let mut batch = SpriteBatch::new();
        batch.push(Sprite::new(0.0, 0.0, 10.0, 10.0).with_color(Color::RED));
        let (verts, _) = batch_to_vertices(&batch);
        for v in &verts {
            assert_eq!(v.color, [1.0, 0.0, 0.0, 1.0]);
        }
    }

    #[test]
    fn batch_to_vertices_tex_coords() {
        let mut batch = SpriteBatch::new();
        batch.push(Sprite::new(0.0, 0.0, 10.0, 10.0));
        let (verts, _) = batch_to_vertices(&batch);
        assert_eq!(verts[0].tex_coords, [0.0, 0.0]);
        assert_eq!(verts[1].tex_coords, [1.0, 0.0]);
        assert_eq!(verts[2].tex_coords, [1.0, 1.0]);
        assert_eq!(verts[3].tex_coords, [0.0, 1.0]);
    }

    #[test]
    fn quad_indices_are_valid() {
        // Two triangles forming a quad
        assert_eq!(QUAD_INDICES, [0, 1, 2, 2, 3, 0]);
    }
}
