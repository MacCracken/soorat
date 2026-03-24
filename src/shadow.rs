//! Shadow mapping for directional lights.

use crate::math_util::{IDENTITY_MAT4, cross, look_at, mul_mat4, normalize3, perspective_90};
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

// ── Cascaded Shadow Maps ────────────────────────────────────────────────────

/// Maximum number of shadow cascades.
pub const MAX_CASCADES: usize = 4;

/// Cascaded shadow map — multiple depth textures at different distance ranges.
pub struct CascadedShadowMap {
    pub cascades: Vec<ShadowMap>,
    pub split_distances: Vec<f32>,
    pub view_proj_matrices: Vec<[f32; 16]>,
}

impl CascadedShadowMap {
    /// Create cascaded shadow maps with the given number of cascades and resolution.
    pub fn new(device: &wgpu::Device, cascade_count: u32, resolution: u32) -> Self {
        let count = (cascade_count as usize).clamp(1, MAX_CASCADES);
        let cascades = (0..count)
            .map(|_| ShadowMap::new(device, resolution))
            .collect();
        Self {
            cascades,
            split_distances: vec![0.0; count + 1],
            view_proj_matrices: vec![IDENTITY_MAT4; count],
        }
    }

    /// Compute cascade split distances using practical split scheme (Nvidia GPU Gems 3).
    /// `near`/`far`: camera frustum range.
    /// `lambda`: blend between logarithmic (1.0) and uniform (0.0) splits. 0.5 is typical.
    pub fn compute_splits(&mut self, near: f32, far: f32, lambda: f32) {
        let count = self.cascades.len();
        self.split_distances[0] = near;
        for i in 1..count {
            let ratio = i as f32 / count as f32;
            let log_split = near * (far / near).powf(ratio);
            let uniform_split = near + (far - near) * ratio;
            self.split_distances[i] = lambda * log_split + (1.0 - lambda) * uniform_split;
        }
        self.split_distances[count] = far;
    }

    /// Update the view-projection matrix for a specific cascade.
    pub fn set_cascade_matrix(&mut self, index: usize, matrix: [f32; 16]) {
        if index < self.view_proj_matrices.len() {
            self.view_proj_matrices[index] = matrix;
        }
    }

    /// Number of cascades.
    pub fn cascade_count(&self) -> usize {
        self.cascades.len()
    }
}

/// Cascade uniforms for the PBR shader — split distances + matrices.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CascadeUniforms {
    /// Split distances (x,y,z,w = split 0-3).
    pub splits: [f32; 4],
    /// View-projection matrices for each cascade.
    pub matrices: [[f32; 16]; MAX_CASCADES],
}

impl Default for CascadeUniforms {
    fn default() -> Self {
        Self {
            splits: [10.0, 30.0, 100.0, 500.0],
            matrices: [IDENTITY_MAT4; MAX_CASCADES],
        }
    }
}

// ── Shadow Atlas ────────────────────────────────────────────────────────────

/// Shadow atlas — a single large texture subdivided into regions for multiple lights.
pub struct ShadowAtlas {
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: u32,
    pub tile_size: u32,
    pub columns: u32,
}

impl ShadowAtlas {
    /// Create a shadow atlas.
    /// `size`: total atlas resolution (e.g., 4096).
    /// `tile_size`: resolution per light (e.g., 1024 → 4×4 = 16 lights).
    pub fn new(device: &wgpu::Device, size: u32, tile_size: u32) -> Self {
        let columns = size / tile_size.max(1);

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow_atlas"),
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_atlas_sampler"),
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
            tile_size,
            columns,
        }
    }

    /// Get the viewport (x, y, w, h) for a given light index in the atlas.
    pub fn tile_viewport(&self, index: u32) -> (u32, u32, u32, u32) {
        let col = index % self.columns;
        let row = index / self.columns;
        (
            col * self.tile_size,
            row * self.tile_size,
            self.tile_size,
            self.tile_size,
        )
    }

    /// Get the UV offset and scale for a tile (for shader sampling).
    pub fn tile_uv(&self, index: u32) -> [f32; 4] {
        let col = index % self.columns;
        let row = index / self.columns;
        let scale = self.tile_size as f32 / self.size as f32;
        [col as f32 * scale, row as f32 * scale, scale, scale]
    }

    /// Maximum number of lights that fit in the atlas.
    pub fn max_lights(&self) -> u32 {
        self.columns * self.columns
    }
}

// ── Point Light Shadows ─────────────────────────────────────────────────────

/// Point light shadow map — 6 faces (cube map emulated as 6 atlas tiles).
pub struct PointShadowMap {
    /// 6 view-projection matrices (one per cube face: +X, -X, +Y, -Y, +Z, -Z).
    pub face_matrices: [[f32; 16]; 6],
}

impl PointShadowMap {
    /// Compute the 6 face view-projection matrices for a point light.
    pub fn new(position: [f32; 3], near: f32, far: f32) -> Self {
        // Perspective projection for 90° FOV (cube face)
        let proj = perspective_90(near, far);

        // 6 look directions + up vectors for cube faces
        let faces: [([f32; 3], [f32; 3]); 6] = [
            ([1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),  // +X
            ([-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]), // -X
            ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),   // +Y
            ([0.0, -1.0, 0.0], [0.0, 0.0, -1.0]), // -Y
            ([0.0, 0.0, 1.0], [0.0, -1.0, 0.0]),  // +Z
            ([0.0, 0.0, -1.0], [0.0, -1.0, 0.0]), // -Z
        ];

        let mut face_matrices = [IDENTITY_MAT4; 6];
        for (i, (dir, up)) in faces.iter().enumerate() {
            let view = look_at(position, *dir, *up);
            face_matrices[i] = mul_mat4(proj, view);
        }

        Self { face_matrices }
    }
}

/// Compute practical cascade split distances (standalone, no GPU needed).
pub fn compute_practical_splits(near: f32, far: f32, count: usize, lambda: f32) -> Vec<f32> {
    let mut splits = Vec::with_capacity(count + 1);
    splits.push(near);
    for i in 1..count {
        let ratio = i as f32 / count as f32;
        let log_split = near * (far / near).powf(ratio);
        let uniform_split = near + (far - near) * ratio;
        splits.push(lambda * log_split + (1.0 - lambda) * uniform_split);
    }
    splits.push(far);
    splits
}

/// Atlas configuration for pure-CPU tile math (no GPU needed).
pub struct ShadowAtlasConfig {
    pub size: u32,
    pub tile_size: u32,
}

/// Get viewport for a tile index in an atlas config.
pub fn tile_viewport(config: &ShadowAtlasConfig, index: u32) -> (u32, u32, u32, u32) {
    let columns = config.size / config.tile_size.max(1);
    let col = index % columns;
    let row = index / columns;
    (
        col * config.tile_size,
        row * config.tile_size,
        config.tile_size,
        config.tile_size,
    )
}

/// Get UV offset+scale for a tile index in an atlas config.
pub fn tile_uv(config: &ShadowAtlasConfig, index: u32) -> [f32; 4] {
    let columns = config.size / config.tile_size.max(1);
    let col = index % columns;
    let row = index / columns;
    let scale = config.tile_size as f32 / config.size as f32;
    [col as f32 * scale, row as f32 * scale, scale, scale]
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

    #[test]
    fn cascade_uniforms_size() {
        // 4 floats + 4 * 16 floats = 4*4 + 4*64 = 16 + 256 = 272
        assert_eq!(std::mem::size_of::<CascadeUniforms>(), 272);
    }

    #[test]
    fn cascade_uniforms_default() {
        let u = CascadeUniforms::default();
        assert_eq!(u.splits[0], 10.0);
        assert_eq!(u.splits[3], 500.0);
    }

    #[test]
    fn cascade_splits_practical() {
        // Test the practical split scheme
        let splits = compute_practical_splits(0.1, 100.0, 4, 0.5);
        assert_eq!(splits.len(), 5); // 4 cascades = 5 split points
        assert_eq!(splits[0], 0.1);
        assert_eq!(splits[4], 100.0);
        // Splits should be monotonically increasing
        for i in 1..splits.len() {
            assert!(splits[i] > splits[i - 1]);
        }
    }

    #[test]
    fn shadow_atlas_tile_viewport() {
        let atlas = ShadowAtlasConfig {
            size: 4096,
            tile_size: 1024,
        };
        let columns = atlas.size / atlas.tile_size;
        assert_eq!(columns, 4);
        // tile 0 = (0,0), tile 1 = (1024,0), tile 4 = (0,1024)
        assert_eq!(tile_viewport(&atlas, 0), (0, 0, 1024, 1024));
        assert_eq!(tile_viewport(&atlas, 1), (1024, 0, 1024, 1024));
        assert_eq!(tile_viewport(&atlas, 4), (0, 1024, 1024, 1024));
    }

    #[test]
    fn shadow_atlas_tile_uv() {
        let atlas = ShadowAtlasConfig {
            size: 4096,
            tile_size: 1024,
        };
        let uv = tile_uv(&atlas, 0);
        assert_eq!(uv, [0.0, 0.0, 0.25, 0.25]);
        let uv1 = tile_uv(&atlas, 1);
        assert!((uv1[0] - 0.25).abs() < 0.001);
    }

    #[test]
    fn shadow_atlas_max_lights() {
        let atlas = ShadowAtlasConfig {
            size: 4096,
            tile_size: 1024,
        };
        assert_eq!(
            atlas.size / atlas.tile_size * (atlas.size / atlas.tile_size),
            16
        );
    }

    #[test]
    fn point_shadow_6_faces() {
        let psm = PointShadowMap::new([0.0, 5.0, 0.0], 0.1, 25.0);
        assert_eq!(psm.face_matrices.len(), 6);
        // All matrices should be different
        for i in 0..6 {
            for j in (i + 1)..6 {
                assert!(psm.face_matrices[i] != psm.face_matrices[j]);
            }
        }
    }

    #[test]
    fn point_shadow_no_nan() {
        let psm = PointShadowMap::new([10.0, 3.0, -5.0], 0.1, 50.0);
        for face in &psm.face_matrices {
            for &v in face {
                assert!(!v.is_nan(), "Point shadow matrix contains NaN");
            }
        }
    }

    #[test]
    fn perspective_90_valid() {
        let p = perspective_90(0.1, 100.0);
        assert_eq!(p[0], 1.0); // aspect=1, fov=90 → f=1
        assert_eq!(p[5], 1.0);
        assert!(!p[10].is_nan());
    }
}
