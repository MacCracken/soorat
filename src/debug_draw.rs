//! Debug line rendering pipeline and shape helpers.

use crate::color::Color;
use crate::error::Result;
use crate::mesh_pipeline::DepthBuffer;
use wgpu::util::DeviceExt;

/// A vertex for debug line rendering: 3D position + RGBA color.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl LineVertex {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Accumulator for debug line segments.
#[derive(Debug, Clone, Default)]
pub struct LineBatch {
    pub vertices: Vec<LineVertex>,
}

impl LineBatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(lines: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(lines * 2),
        }
    }

    /// Add a line segment between two points.
    pub fn line(&mut self, a: [f32; 3], b: [f32; 3], color: Color) {
        let c = color.to_array();
        self.vertices.push(LineVertex {
            position: a,
            color: c,
        });
        self.vertices.push(LineVertex {
            position: b,
            color: c,
        });
    }

    /// Draw a wireframe box (12 edges).
    pub fn wire_box(&mut self, min: [f32; 3], max: [f32; 3], color: Color) {
        let [x0, y0, z0] = min;
        let [x1, y1, z1] = max;

        // Bottom face
        self.line([x0, y0, z0], [x1, y0, z0], color);
        self.line([x1, y0, z0], [x1, y0, z1], color);
        self.line([x1, y0, z1], [x0, y0, z1], color);
        self.line([x0, y0, z1], [x0, y0, z0], color);

        // Top face
        self.line([x0, y1, z0], [x1, y1, z0], color);
        self.line([x1, y1, z0], [x1, y1, z1], color);
        self.line([x1, y1, z1], [x0, y1, z1], color);
        self.line([x0, y1, z1], [x0, y1, z0], color);

        // Vertical edges
        self.line([x0, y0, z0], [x0, y1, z0], color);
        self.line([x1, y0, z0], [x1, y1, z0], color);
        self.line([x1, y0, z1], [x1, y1, z1], color);
        self.line([x0, y0, z1], [x0, y1, z1], color);
    }

    /// Draw a wireframe circle in the XZ plane.
    pub fn wire_circle(&mut self, center: [f32; 3], radius: f32, segments: u32, color: Color) {
        if segments == 0 {
            return;
        }
        let step = std::f32::consts::TAU / segments as f32;
        let (mut prev_s, mut prev_c) = (0.0_f32, 1.0_f32); // sin(0), cos(0)
        for i in 1..=segments {
            let angle = step * i as f32;
            let (next_s, next_c) = (angle.sin(), angle.cos());
            self.line(
                [
                    center[0] + radius * prev_c,
                    center[1],
                    center[2] + radius * prev_s,
                ],
                [
                    center[0] + radius * next_c,
                    center[1],
                    center[2] + radius * next_s,
                ],
                color,
            );
            prev_s = next_s;
            prev_c = next_c;
        }
    }

    /// Draw a wireframe sphere (3 circles: XZ, XY, YZ planes).
    pub fn wire_sphere(&mut self, center: [f32; 3], radius: f32, segments: u32, color: Color) {
        if segments == 0 {
            return;
        }
        let step = std::f32::consts::TAU / segments as f32;
        let (mut ps, mut pc) = (0.0_f32, 1.0_f32);
        for i in 1..=segments {
            let angle = step * i as f32;
            let (ns, nc) = (angle.sin(), angle.cos());

            // XZ plane
            self.line(
                [center[0] + radius * pc, center[1], center[2] + radius * ps],
                [center[0] + radius * nc, center[1], center[2] + radius * ns],
                color,
            );
            // XY plane
            self.line(
                [center[0] + radius * pc, center[1] + radius * ps, center[2]],
                [center[0] + radius * nc, center[1] + radius * ns, center[2]],
                color,
            );
            // YZ plane
            self.line(
                [center[0], center[1] + radius * pc, center[2] + radius * ps],
                [center[0], center[1] + radius * nc, center[2] + radius * ns],
                color,
            );

            ps = ns;
            pc = nc;
        }
    }

    /// Draw a ground grid in the XZ plane.
    pub fn grid(&mut self, half_extent: f32, spacing: f32, color: Color) {
        if spacing <= 0.0 || half_extent <= 0.0 {
            return;
        }
        let count = (half_extent / spacing).ceil() as i32;
        for i in -count..=count {
            let pos = i as f32 * spacing;
            // Lines along Z
            self.line([pos, 0.0, -half_extent], [pos, 0.0, half_extent], color);
            // Lines along X
            self.line([-half_extent, 0.0, pos], [half_extent, 0.0, pos], color);
        }
    }

    /// Draw a wireframe capsule (two circles + connecting lines) along the Y axis.
    pub fn wire_capsule(
        &mut self,
        center: [f32; 3],
        half_height: f32,
        radius: f32,
        segments: u32,
        color: Color,
    ) {
        // Top and bottom circle
        let top = [center[0], center[1] + half_height, center[2]];
        let bot = [center[0], center[1] - half_height, center[2]];
        self.wire_circle(top, radius, segments, color);
        self.wire_circle(bot, radius, segments, color);

        // Connecting vertical lines (4 cardinal directions)
        for &(dx, dz) in &[(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            self.line(
                [top[0] + radius * dx, top[1], top[2] + radius * dz],
                [bot[0] + radius * dx, bot[1], bot[2] + radius * dz],
                color,
            );
        }
    }

    /// Draw a wireframe for an impetus ColliderShape at a given position.
    #[cfg(feature = "physics-debug")]
    pub fn collider(
        &mut self,
        shape: &impetus::collider::ColliderShape,
        position: [f32; 3],
        color: Color,
    ) {
        match shape {
            impetus::collider::ColliderShape::Box { half_extents } => {
                let he = [
                    half_extents[0] as f32,
                    half_extents[1] as f32,
                    half_extents[2] as f32,
                ];
                self.wire_box(
                    [
                        position[0] - he[0],
                        position[1] - he[1],
                        position[2] - he[2],
                    ],
                    [
                        position[0] + he[0],
                        position[1] + he[1],
                        position[2] + he[2],
                    ],
                    color,
                );
            }
            impetus::collider::ColliderShape::Ball { radius } => {
                self.wire_sphere(position, *radius as f32, 16, color);
            }
            impetus::collider::ColliderShape::Capsule {
                half_height,
                radius,
            } => {
                self.wire_capsule(position, *half_height as f32, *radius as f32, 16, color);
            }
            impetus::collider::ColliderShape::Segment { a, b } => {
                self.line(
                    [
                        a[0] as f32 + position[0],
                        a[1] as f32 + position[1],
                        a[2] as f32 + position[2],
                    ],
                    [
                        b[0] as f32 + position[0],
                        b[1] as f32 + position[1],
                        b[2] as f32 + position[2],
                    ],
                    color,
                );
            }
            _ => {
                // ConvexHull, TriMesh, Heightfield — complex shapes, skip for now
            }
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    pub fn line_count(&self) -> usize {
        self.vertices.len() / 2
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
}

/// Debug line rendering pipeline.
pub struct LinePipeline {
    render_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
}

impl LinePipeline {
    /// Create a new line pipeline for the given surface format.
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Result<Self> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("line_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("line.wgsl").into()),
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("line_uniform_layout"),
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
            label: Some("line_pipeline_layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("line_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[LineVertex::layout()],
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
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DepthBuffer::FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("line_uniform_buffer"),
            size: 64, // mat4x4<f32>
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("line_uniform_bind_group"),
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
        })
    }

    /// Update the view-projection matrix.
    pub fn update_view_proj(&self, queue: &wgpu::Queue, view_proj: &[f32; 16]) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(view_proj));
    }

    /// Draw debug lines. Renders on top of existing content (LoadOp::Load).
    pub fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_view: &wgpu::TextureView,
        depth: &DepthBuffer,
        batch: &LineBatch,
    ) {
        if batch.is_empty() {
            return;
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("line_vertex_buffer"),
            contents: bytemuck::cast_slice(&batch.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("line_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("line_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.draw(0..batch.vertices.len() as u32, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }

    /// Draw debug lines into an existing render pass (for egui/editor integration).
    pub fn draw_into_pass<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        batch: &LineBatch,
        device: &wgpu::Device,
    ) {
        if batch.is_empty() {
            return;
        }
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("line_vertex_buffer"),
            contents: bytemuck::cast_slice(&batch.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..batch.vertices.len() as u32, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_vertex_size() {
        // 3 + 4 = 7 floats = 28 bytes
        assert_eq!(std::mem::size_of::<LineVertex>(), 28);
    }

    #[test]
    fn line_vertex_layout() {
        let layout = LineVertex::layout();
        assert_eq!(layout.array_stride, 28);
        assert_eq!(layout.attributes.len(), 2);
    }

    #[test]
    fn line_batch_empty() {
        let batch = LineBatch::new();
        assert!(batch.is_empty());
        assert_eq!(batch.line_count(), 0);
    }

    #[test]
    fn line_batch_single_line() {
        let mut batch = LineBatch::new();
        batch.line([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], Color::RED);
        assert_eq!(batch.line_count(), 1);
        assert_eq!(batch.vertices.len(), 2);
        assert_eq!(batch.vertices[0].position, [0.0, 0.0, 0.0]);
        assert_eq!(batch.vertices[1].position, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn wire_box_generates_12_edges() {
        let mut batch = LineBatch::new();
        batch.wire_box([0.0, 0.0, 0.0], [1.0, 1.0, 1.0], Color::GREEN);
        assert_eq!(batch.line_count(), 12);
        assert_eq!(batch.vertices.len(), 24);
    }

    #[test]
    fn wire_circle_generates_correct_segments() {
        let mut batch = LineBatch::new();
        batch.wire_circle([0.0, 0.0, 0.0], 1.0, 16, Color::BLUE);
        assert_eq!(batch.line_count(), 16);
    }

    #[test]
    fn wire_sphere_generates_3_circles() {
        let mut batch = LineBatch::new();
        batch.wire_sphere([0.0, 0.0, 0.0], 1.0, 16, Color::WHITE);
        assert_eq!(batch.line_count(), 48); // 3 * 16
    }

    #[test]
    fn grid_generates_lines() {
        let mut batch = LineBatch::new();
        batch.grid(5.0, 1.0, Color::WHITE);
        // half_extent=5, spacing=1: count=5, lines from -5 to 5 = 11 per axis = 22 total
        assert_eq!(batch.line_count(), 22);
    }

    #[test]
    fn line_batch_clear() {
        let mut batch = LineBatch::new();
        batch.line([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], Color::RED);
        batch.clear();
        assert!(batch.is_empty());
    }

    #[test]
    fn line_batch_with_capacity() {
        let batch = LineBatch::with_capacity(100);
        assert!(batch.is_empty());
        assert!(batch.vertices.capacity() >= 200);
    }

    #[test]
    fn wire_circle_zero_segments() {
        let mut batch = LineBatch::new();
        batch.wire_circle([0.0, 0.0, 0.0], 1.0, 0, Color::RED);
        assert!(batch.is_empty());
    }

    #[test]
    fn wire_sphere_zero_segments() {
        let mut batch = LineBatch::new();
        batch.wire_sphere([0.0, 0.0, 0.0], 1.0, 0, Color::RED);
        assert!(batch.is_empty());
    }

    #[test]
    fn grid_zero_spacing() {
        let mut batch = LineBatch::new();
        batch.grid(5.0, 0.0, Color::WHITE);
        assert!(batch.is_empty());
    }

    #[test]
    fn grid_negative_spacing() {
        let mut batch = LineBatch::new();
        batch.grid(5.0, -1.0, Color::WHITE);
        assert!(batch.is_empty());
    }

    #[test]
    fn wire_capsule_generates_lines() {
        let mut batch = LineBatch::new();
        batch.wire_capsule([0.0, 0.0, 0.0], 1.0, 0.5, 16, Color::WHITE);
        // 2 circles (16 each) + 4 vertical lines = 36
        assert_eq!(batch.line_count(), 36);
    }

    #[test]
    fn line_vertex_bytemuck() {
        let v = LineVertex {
            position: [1.0, 2.0, 3.0],
            color: [1.0, 0.0, 0.0, 1.0],
        };
        let bytes = bytemuck::bytes_of(&v);
        assert_eq!(bytes.len(), 28);
    }
}
