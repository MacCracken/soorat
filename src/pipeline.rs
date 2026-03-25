//! Sprite rendering pipeline.

//! Also re-exports batch functions from [`crate::batch`] for backward compatibility.

use crate::color::Color;
use crate::error::Result;
use crate::sprite::SpriteBatch;
use crate::vertex::Vertex2D;
use wgpu::util::DeviceExt;

// Re-export batch functions for backward compatibility
pub use crate::batch::{
    MAX_SPRITES_PER_BATCH, batch_to_vertices, batch_to_vertices_into, batch_to_vertices_u32,
    batch_to_vertices_u32_into,
};

/// Orthographic projection matrix for 2D rendering.
/// Maps screen coordinates (0,0 top-left) to clip space.
/// Origin at top-left, Y-axis points down, Z range [0, 1].
fn orthographic_projection(width: f32, height: f32) -> [f32; 16] {
    // Column-major 4x4: origin (0,0) top-left, right-handed, zero-to-one depth
    // Simplified from general ortho with l=0, t=0, n=-1, f=1
    [
        2.0 / width,
        0.0,
        0.0,
        0.0,
        0.0,
        -2.0 / height,
        0.0,
        0.0,
        0.0,
        0.0,
        -0.5,
        0.0,
        -1.0,
        1.0,
        0.5,
        1.0,
    ]
}

/// Persistent GPU buffers for sprite rendering. Reuse across frames to avoid per-frame allocation.
pub struct SpriteBuffers {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    index_capacity: usize,
    // CPU-side staging
    vertices: Vec<Vertex2D>,
    indices: Vec<u16>,
}

impl SpriteBuffers {
    /// Create persistent buffers sized for the given sprite count.
    pub fn new(device: &wgpu::Device, sprite_capacity: usize) -> Self {
        let vert_cap = sprite_capacity * 4;
        let idx_cap = sprite_capacity * 6;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_vertex_buffer_persistent"),
            size: (vert_cap * std::mem::size_of::<Vertex2D>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite_index_buffer_persistent"),
            size: (idx_cap * std::mem::size_of::<u16>()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            vertex_buffer,
            index_buffer,
            vertex_capacity: vert_cap,
            index_capacity: idx_cap,
            vertices: Vec::with_capacity(vert_cap),
            indices: Vec::with_capacity(idx_cap),
        }
    }

    /// Prepare buffers for a batch. Grows GPU buffers if needed, then writes via queue.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, batch: &SpriteBatch) {
        batch_to_vertices_into(batch, &mut self.vertices, &mut self.indices);

        // Regrow GPU buffers if CPU data exceeds capacity
        if self.vertices.len() > self.vertex_capacity {
            self.vertex_capacity = self.vertices.len();
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite_vertex_buffer_persistent"),
                size: (self.vertex_capacity * std::mem::size_of::<Vertex2D>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if self.indices.len() > self.index_capacity {
            self.index_capacity = self.indices.len();
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite_index_buffer_persistent"),
                size: (self.index_capacity * std::mem::size_of::<u16>()) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&self.indices));
    }

    /// Number of indices currently written.
    #[must_use]
    #[inline]
    pub fn index_count(&self) -> u32 {
        self.indices.len() as u32
    }
}

/// Per-frame rendering statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameStats {
    pub draw_calls: u32,
    pub triangles: u32,
    pub sprites: u32,
}

/// Parameters for `SpritePipeline::draw_batched`.
pub struct SpriteBatchDrawParams<'a> {
    pub view: &'a wgpu::TextureView,
    pub batch: &'a SpriteBatch,
    pub texture_cache: &'a crate::texture::TextureCache,
    pub fallback_bind_group: &'a wgpu::BindGroup,
    pub clear_color: Option<Color>,
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
    #[must_use]
    pub fn texture_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_bind_group_layout
    }

    /// Update the projection matrix for the current viewport size.
    pub fn update_projection(&self, queue: &wgpu::Queue, width: f32, height: f32) {
        let proj = orthographic_projection(width, height);
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&proj));
    }

    /// Draw a sprite batch with a single texture. The batch should already be sorted by z-order.
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
    ) -> FrameStats {
        let mut stats = FrameStats::default();

        if batch.is_empty() && clear_color.is_none() {
            return stats;
        }

        let (vertices, indices) = batch_to_vertices(batch);
        stats.sprites = batch.sprites.len() as u32;
        stats.triangles = indices.len() as u32 / 3;

        let (vertex_buffer, index_buffer) = Self::upload_buffers(device, &vertices, &indices);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("sprite_encoder"),
        });

        {
            let mut render_pass = Self::begin_pass(&mut encoder, view, clear_color);

            if !batch.is_empty() {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_bind_group(1, texture_bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
                stats.draw_calls = 1;
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
        stats
    }

    /// Draw using pre-allocated `SpriteBuffers`. Call `buffers.prepare()` first.
    /// Zero per-frame GPU allocations after the first frame.
    pub fn draw_with_buffers(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        buffers: &SpriteBuffers,
        texture_bind_group: &wgpu::BindGroup,
        clear_color: Option<Color>,
    ) -> FrameStats {
        let mut stats = FrameStats::default();
        let index_count = buffers.index_count();

        if index_count == 0 && clear_color.is_none() {
            return stats;
        }

        stats.sprites = index_count / 6;
        stats.triangles = index_count / 3;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("sprite_encoder"),
        });

        {
            let mut render_pass = Self::begin_pass(&mut encoder, view, clear_color);

            if index_count > 0 {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_bind_group(1, texture_bind_group, &[]);
                render_pass.set_vertex_buffer(0, buffers.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(buffers.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..index_count, 0, 0..1);
                stats.draw_calls = 1;
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
        stats
    }

    /// Draw a sprite batch with multiple textures, issuing one draw call per texture group.
    /// Sprites are drawn in z-order; consecutive sprites sharing a texture_id are batched.
    pub fn draw_batched(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        params: &SpriteBatchDrawParams<'_>,
    ) -> FrameStats {
        let mut stats = FrameStats::default();

        if params.batch.is_empty() && params.clear_color.is_none() {
            return stats;
        }

        let (vertices, indices) = batch_to_vertices(params.batch);
        stats.sprites = params.batch.sprites.len() as u32;
        stats.triangles = indices.len() as u32 / 3;

        let (vertex_buffer, index_buffer) = Self::upload_buffers(device, &vertices, &indices);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("sprite_encoder"),
        });

        {
            let mut render_pass = Self::begin_pass(&mut encoder, params.view, params.clear_color);

            if !params.batch.is_empty() {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);

                // Issue draw calls grouped by consecutive texture_id
                let mut run_start: u32 = 0;
                let mut current_tex_id = params.batch.sprites[0].texture_id;

                for (i, sprite) in params.batch.sprites.iter().enumerate() {
                    if sprite.texture_id != current_tex_id {
                        let bind_group = params
                            .texture_cache
                            .get_bind_group(current_tex_id)
                            .unwrap_or(params.fallback_bind_group);
                        render_pass.set_bind_group(1, bind_group, &[]);
                        let idx_start = run_start * 6;
                        let idx_end = (i as u32) * 6;
                        render_pass.draw_indexed(idx_start..idx_end, 0, 0..1);
                        stats.draw_calls += 1;

                        run_start = i as u32;
                        current_tex_id = sprite.texture_id;
                    }
                }

                let bind_group = params
                    .texture_cache
                    .get_bind_group(current_tex_id)
                    .unwrap_or(params.fallback_bind_group);
                render_pass.set_bind_group(1, bind_group, &[]);
                let idx_start = run_start * 6;
                let idx_end = params.batch.sprites.len() as u32 * 6;
                render_pass.draw_indexed(idx_start..idx_end, 0, 0..1);
                stats.draw_calls += 1;
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
        stats
    }

    fn upload_buffers(
        device: &wgpu::Device,
        vertices: &[Vertex2D],
        indices: &[u16],
    ) -> (wgpu::Buffer, wgpu::Buffer) {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite_vertex_buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite_index_buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        (vertex_buffer, index_buffer)
    }

    fn begin_pass<'a>(
        encoder: &'a mut wgpu::CommandEncoder,
        view: &'a wgpu::TextureView,
        clear_color: Option<Color>,
    ) -> wgpu::RenderPass<'a> {
        let load_op = match clear_color {
            Some(c) => wgpu::LoadOp::Clear(c.to_wgpu()),
            None => wgpu::LoadOp::Load,
        };

        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        })
    }

    /// Draw a sprite batch into an existing render pass (for egui/editor integration).
    pub fn draw_into_pass<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        batch: &SpriteBatch,
        texture_bind_group: &'a wgpu::BindGroup,
        device: &wgpu::Device,
    ) {
        if batch.is_empty() {
            return;
        }
        let (vertices, indices) = batch_to_vertices(batch);
        let (vertex_buffer, index_buffer) = Self::upload_buffers(device, &vertices, &indices);
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sprite::Sprite;

    #[test]
    fn orthographic_projection_values() {
        let proj = orthographic_projection(800.0, 600.0);
        assert_eq!(proj.len(), 16);
        // Column-major: proj[0] = 2/w, proj[5] = -2/h, proj[12] = -1, proj[13] = 1
        assert!((proj[0] - 2.0 / 800.0).abs() < f32::EPSILON);
        assert!((proj[5] - (-2.0 / 600.0)).abs() < f32::EPSILON);
        assert!((proj[12] - (-1.0)).abs() < f32::EPSILON);
        assert!((proj[13] - 1.0).abs() < f32::EPSILON);
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
        // No rotation: corners should match sprite bounds
        assert_eq!(verts[0].position, [10.0, 20.0]);
        assert_eq!(verts[1].position, [42.0, 20.0]);
        assert_eq!(verts[2].position, [42.0, 52.0]);
        assert_eq!(verts[3].position, [10.0, 52.0]);
    }

    #[test]
    fn batch_to_vertices_rotated_sprite() {
        let mut batch = SpriteBatch::new();
        // 90 degrees CCW
        let half_pi = std::f32::consts::FRAC_PI_2;
        batch.push(Sprite::new(0.0, 0.0, 100.0, 100.0).with_rotation(half_pi));
        let (verts, _) = batch_to_vertices(&batch);

        // Center is (50, 50). After 90° CCW rotation:
        // (-50,-50) rotated = (50,-50) + center = (100, 0)... let me check
        // rot(90): x' = x*cos - y*sin, y' = x*sin + y*cos
        // cos(90)=0, sin(90)=1
        // (-50,-50): x'= -50*0 - (-50)*1 = 50, y'= -50*1 + (-50)*0 = -50 => (100, 0)
        // (50,-50):  x'= 50*0 - (-50)*1 = 50,  y'= 50*1 + (-50)*0 = 50 => (100, 100)
        // (50,50):   x'= 50*0 - 50*1 = -50,    y'= 50*1 + 50*0 = 50 => (0, 100)
        // (-50,50):  x'= -50*0 - 50*1 = -50,   y'= -50*1 + 50*0 = -50 => (0, 0)
        let eps = 0.01;
        assert!((verts[0].position[0] - 100.0).abs() < eps);
        assert!((verts[0].position[1] - 0.0).abs() < eps);
        assert!((verts[2].position[0] - 0.0).abs() < eps);
        assert!((verts[2].position[1] - 100.0).abs() < eps);
    }

    #[test]
    fn batch_to_vertices_zero_rotation_matches_unrotated() {
        let mut batch_a = SpriteBatch::new();
        batch_a.push(Sprite::new(10.0, 20.0, 50.0, 30.0));
        let mut batch_b = SpriteBatch::new();
        batch_b.push(Sprite::new(10.0, 20.0, 50.0, 30.0).with_rotation(0.0));
        let (va, _) = batch_to_vertices(&batch_a);
        let (vb, _) = batch_to_vertices(&batch_b);
        for (a, b) in va.iter().zip(vb.iter()) {
            assert_eq!(a.position, b.position);
        }
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
    fn batch_to_vertices_into_reuses_buffers() {
        let mut verts = Vec::new();
        let mut indices = Vec::new();

        let mut batch = SpriteBatch::new();
        batch.push(Sprite::new(0.0, 0.0, 10.0, 10.0));
        batch_to_vertices_into(&batch, &mut verts, &mut indices);
        assert_eq!(verts.len(), 4);
        assert_eq!(indices.len(), 6);

        // Second call reuses the same Vecs
        batch.push(Sprite::new(20.0, 0.0, 10.0, 10.0));
        batch_to_vertices_into(&batch, &mut verts, &mut indices);
        assert_eq!(verts.len(), 8);
        assert_eq!(indices.len(), 12);
        // Capacity should be >= 8 (no realloc for second call)
        assert!(verts.capacity() >= 8);
    }

    #[test]
    fn frame_stats_default() {
        let stats = FrameStats::default();
        assert_eq!(stats.draw_calls, 0);
        assert_eq!(stats.triangles, 0);
        assert_eq!(stats.sprites, 0);
    }

    #[test]
    fn batch_to_vertices_with_uv() {
        use crate::sprite::UvRect;
        let uv = UvRect::from_pixel_rect(0, 0, 16, 16, 64, 64);
        let mut batch = SpriteBatch::new();
        batch.push(Sprite::new(0.0, 0.0, 32.0, 32.0).with_uv(uv));
        let (verts, _) = batch_to_vertices(&batch);
        assert_eq!(verts[0].tex_coords, [uv.u_min, uv.v_min]);
        assert_eq!(verts[1].tex_coords, [uv.u_max, uv.v_min]);
        assert_eq!(verts[2].tex_coords, [uv.u_max, uv.v_max]);
        assert_eq!(verts[3].tex_coords, [uv.u_min, uv.v_max]);
    }

    #[test]
    fn batch_to_vertices_into_matches_batch_to_vertices() {
        let mut batch = SpriteBatch::new();
        for i in 0..10 {
            batch.push(Sprite::new(i as f32 * 10.0, 0.0, 32.0, 32.0).with_color(Color::RED));
        }

        let (verts_a, indices_a) = batch_to_vertices(&batch);
        let mut verts_b = Vec::new();
        let mut indices_b = Vec::new();
        batch_to_vertices_into(&batch, &mut verts_b, &mut indices_b);

        assert_eq!(verts_a, verts_b);
        assert_eq!(indices_a, indices_b);
    }

    #[test]
    fn max_sprites_constant() {
        assert_eq!(MAX_SPRITES_PER_BATCH, 16383);
    }

    #[test]
    fn batch_to_vertices_respects_limit() {
        let mut batch = SpriteBatch::new();
        // Push more than the limit
        for i in 0..16400 {
            batch.push(Sprite::new(i as f32, 0.0, 1.0, 1.0));
        }
        let (verts, indices) = batch_to_vertices(&batch);
        // Should be clamped to MAX_SPRITES_PER_BATCH
        assert_eq!(verts.len(), MAX_SPRITES_PER_BATCH * 4);
        assert_eq!(indices.len(), MAX_SPRITES_PER_BATCH * 6);
        // All indices should be valid u16
        for &idx in &indices {
            assert!(idx < u16::MAX);
        }
    }

    #[test]
    fn batch_to_vertices_u32_no_limit() {
        let mut batch = SpriteBatch::new();
        // Push more than u16 limit
        for i in 0..20000 {
            batch.push(Sprite::new(i as f32, 0.0, 1.0, 1.0));
        }
        let (verts, indices) = batch_to_vertices_u32(&batch);
        // Should NOT be clamped
        assert_eq!(verts.len(), 20000 * 4);
        assert_eq!(indices.len(), 20000 * 6);
    }

    #[test]
    fn batch_to_vertices_u32_matches_u16_for_small() {
        let mut batch = SpriteBatch::new();
        for i in 0..10 {
            batch.push(Sprite::new(i as f32, 0.0, 32.0, 32.0));
        }
        let (verts_16, indices_16) = batch_to_vertices(&batch);
        let (verts_32, indices_32) = batch_to_vertices_u32(&batch);
        assert_eq!(verts_16, verts_32);
        // Index values should match (just different types)
        for (a, b) in indices_16.iter().zip(indices_32.iter()) {
            assert_eq!(*a as u32, *b);
        }
    }

    #[test]
    fn batch_to_vertices_at_exact_limit() {
        let mut batch = SpriteBatch::new();
        for i in 0..MAX_SPRITES_PER_BATCH {
            batch.push(Sprite::new(i as f32, 0.0, 1.0, 1.0));
        }
        let (verts, indices) = batch_to_vertices(&batch);
        assert_eq!(verts.len(), MAX_SPRITES_PER_BATCH * 4);
        assert_eq!(indices.len(), MAX_SPRITES_PER_BATCH * 6);
        // Last index should be valid
        let last_idx = *indices.last().unwrap();
        assert!(last_idx < u16::MAX);
    }
}
