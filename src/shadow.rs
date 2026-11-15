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
    /// Create a shadow map with the given resolution (clamped to at least 1).
    #[must_use]
    pub fn new(device: &wgpu::Device, size: u32) -> Self {
        let size = size.max(1);
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

/// Compute an orthographic light-space matrix for a directional light, with
/// the shadow frustum centred on the world origin.
///
/// Equivalent to [`directional_light_matrix_at`] with `center = [0, 0, 0]`:
/// light-space depth is `dot(direction, p)`, so the depth range covers points
/// whose distance **along the light direction from the origin** lies in
/// `[near, far]`. For a scene around the origin use a symmetric range such as
/// `near = -50.0, far = 50.0`.
#[must_use]
#[inline]
pub fn directional_light_matrix(
    direction: [f32; 3],
    extent: f32,
    near: f32,
    far: f32,
) -> [f32; 16] {
    directional_light_matrix_at(direction, [0.0; 3], extent, near, far)
}

/// Compute an orthographic light-space matrix for a directional light whose
/// shadow frustum is centred on `center`.
///
/// - `direction`: light direction (where the light points); need not be unit length.
/// - `center`: world-space point the frustum is centred on (e.g. the camera target).
/// - `extent`: half-size of the frustum in world units (X and Y in light space).
/// - `near`/`far`: depth range measured along `direction` from `center`
///   (negative `near` reaches toward the light, e.g. `-radius..radius`).
///
/// Returns the identity matrix (with a warning) for a non-positive `extent` or
/// `near ≈ far`. Output depth is in wgpu's `[0, 1]` range.
#[must_use]
#[inline]
pub fn directional_light_matrix_at(
    direction: [f32; 3],
    center: [f32; 3],
    extent: f32,
    near: f32,
    far: f32,
) -> [f32; 16] {
    if extent.is_nan() || extent <= 0.0 {
        tracing::warn!(
            extent,
            "directional_light_matrix: extent <= 0 — returning identity matrix"
        );
        return IDENTITY_MAT4;
    }
    let depth_span = (far - near).abs();
    if depth_span.is_nan() || depth_span < 1e-10 {
        tracing::warn!(
            near,
            far,
            "directional_light_matrix: far ≈ near — returning identity matrix"
        );
        return IDENTITY_MAT4;
    }

    // Light view basis: a regular (non-mirrored) view looking along `d`, i.e.
    // right × up = -d, the same handedness as `math_util::look_at`. Front faces
    // therefore keep their winding in the shadow pass.
    let d = normalize3(direction);
    let up_ref = if d[1].abs() > 0.99 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let right = normalize3(cross(d, up_ref));
    let up = cross(right, d);
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

    // proj · view, built directly. The view has rows (right, up, d) translated
    // by -center (light-space z = dot(d, p - center)); the orthographic
    // projection only scales x/y by 1/extent and maps z ∈ [near, far] → [0, 1],
    // so the product needs ~20 flops instead of a general 4×4 multiply.
    let sx = 1.0 / extent;
    let sz = 1.0 / (far - near);
    [
        sx * right[0],
        sx * up[0],
        sz * d[0],
        0.0,
        sx * right[1],
        sx * up[1],
        sz * d[1],
        0.0,
        sx * right[2],
        sx * up[2],
        sz * d[2],
        0.0,
        -sx * dot(right, center),
        -sx * dot(up, center),
        sz * (-dot(d, center) - near),
        1.0,
    ]
}

/// Shadow pass pipeline — renders depth-only from the light's perspective.
///
/// Note: This pipeline uses `fragment: None` (depth-only rendering), which is
/// not supported by `mabda::RenderPipelineBuilder`. The uniform buffer is
/// created via [`mabda::create_uniform_buffer`], and the bind group layout
/// uses [`mabda::BindGroupLayoutBuilder`].
pub struct ShadowPipeline {
    render_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
}

impl ShadowPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        tracing::debug!("creating shadow pipeline");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow.wgsl").into()),
        });

        let bind_group_layout = mabda::BindGroupLayoutBuilder::new()
            .uniform_buffer(wgpu::ShaderStages::VERTEX)
            .build(device, "shadow_uniform_layout");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        // Depth-only pipeline (fragment: None) — cannot use mabda::RenderPipelineBuilder
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
                // Back-face culling: light-facing surfaces are rendered and acne
                // is handled by the depth bias below. (Open geometry such as a
                // single-sided plane therefore still casts shadows.)
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DepthBuffer::FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let defaults = ShadowUniforms::default();
        let uniform_buffer = mabda::create_uniform_buffer(
            device,
            bytemuck::bytes_of(&defaults),
            "shadow_uniform_buffer",
        );

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

    /// Render meshes into the shadow map (depth-only pass).
    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        shadow_map: &ShadowMap,
        meshes: &[&Mesh],
    ) {
        tracing::debug!(
            meshes = meshes.len(),
            size = shadow_map.size,
            "rendering shadow pass"
        );
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
    #[must_use]
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
    /// See [`compute_practical_splits`] for how inputs are sanitized.
    pub fn compute_splits(&mut self, near: f32, far: f32, lambda: f32) {
        self.split_distances = compute_practical_splits(near, far, self.cascades.len(), lambda);
    }

    /// Update the view-projection matrix for a specific cascade.
    pub fn set_cascade_matrix(&mut self, index: usize, matrix: [f32; 16]) {
        if index < self.view_proj_matrices.len() {
            self.view_proj_matrices[index] = matrix;
        }
    }

    /// Number of cascades.
    #[must_use]
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
    ///
    /// `size` is clamped to at least 1. If `tile_size` exceeds `size`, the atlas
    /// holds a single (clipped) tile.
    pub fn new(device: &wgpu::Device, size: u32, tile_size: u32) -> Self {
        let size = size.max(1);
        let columns = atlas_columns(size, tile_size);

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
    /// `index` should be `< max_lights()`; larger indices map past the atlas.
    #[must_use]
    pub fn tile_viewport(&self, index: u32) -> (u32, u32, u32, u32) {
        tile_viewport_in(self.columns, self.tile_size, index)
    }

    /// Get the UV offset and scale for a tile (for shader sampling).
    #[must_use]
    pub fn tile_uv(&self, index: u32) -> [f32; 4] {
        tile_uv_in(self.columns, self.tile_size, self.size, index)
    }

    /// Maximum number of lights that fit in the atlas.
    #[must_use]
    pub fn max_lights(&self) -> u32 {
        self.columns.saturating_mul(self.columns)
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
    #[must_use]
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
///
/// Returns `count + 1` monotonically non-decreasing distances from `near` to
/// `far` (Nvidia GPU Gems 3 practical split scheme). Inputs are sanitized so
/// the result is always finite: a non-positive/NaN `near` becomes `0.001`,
/// `far` is raised to at least `near`, and `lambda` is clamped to `[0, 1]`.
#[must_use]
pub fn compute_practical_splits(near: f32, far: f32, count: usize, lambda: f32) -> Vec<f32> {
    let near = if near > 0.0 && near.is_finite() {
        near
    } else {
        0.001_f32
    };
    let far = if far.is_finite() { far.max(near) } else { near };
    let lambda = if lambda.is_nan() {
        0.5
    } else {
        lambda.clamp(0.0, 1.0)
    };
    let count = count.max(1);

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
#[must_use]
pub fn tile_viewport(config: &ShadowAtlasConfig, index: u32) -> (u32, u32, u32, u32) {
    tile_viewport_in(
        atlas_columns(config.size, config.tile_size),
        config.tile_size,
        index,
    )
}

/// Get UV offset+scale for a tile index in an atlas config.
#[must_use]
pub fn tile_uv(config: &ShadowAtlasConfig, index: u32) -> [f32; 4] {
    tile_uv_in(
        atlas_columns(config.size, config.tile_size),
        config.tile_size,
        config.size,
        index,
    )
}

/// Tiles per atlas row — at least 1, so tile math never divides by zero.
#[must_use]
#[inline]
fn atlas_columns(size: u32, tile_size: u32) -> u32 {
    (size / tile_size.max(1)).max(1)
}

#[must_use]
#[inline]
fn tile_viewport_in(columns: u32, tile_size: u32, index: u32) -> (u32, u32, u32, u32) {
    let columns = columns.max(1);
    let col = index % columns;
    let row = index / columns;
    (
        col.saturating_mul(tile_size),
        row.saturating_mul(tile_size),
        tile_size,
        tile_size,
    )
}

#[must_use]
#[inline]
fn tile_uv_in(columns: u32, tile_size: u32, size: u32, index: u32) -> [f32; 4] {
    let columns = columns.max(1);
    let col = index % columns;
    let row = index / columns;
    let scale = tile_size as f32 / size.max(1) as f32;
    [col as f32 * scale, row as f32 * scale, scale, scale]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transform(m: &[f32; 16], p: [f32; 3]) -> [f32; 4] {
        let mut out = [0.0; 4];
        for (row, o) in out.iter_mut().enumerate() {
            *o = m[row] * p[0] + m[4 + row] * p[1] + m[8 + row] * p[2] + m[12 + row];
        }
        out
    }

    #[test]
    fn light_matrix_at_centres_frustum_and_depth() {
        let center = [10.0, 3.0, -4.0];
        let d = [0.0, -1.0, 0.0];
        let m = directional_light_matrix_at(d, center, 5.0, -20.0, 20.0);
        // Center maps to the middle of the depth range and the XY origin.
        let c = transform(&m, center);
        assert!(c[0].abs() < 1e-5 && c[1].abs() < 1e-5);
        assert!((c[2] - 0.5).abs() < 1e-5);
        // Toward the light (up, against d) is nearer; along d is farther.
        let near = transform(&m, [10.0, 23.0, -4.0]);
        let far = transform(&m, [10.0, -17.0, -4.0]);
        assert!(near[2].abs() < 1e-5, "near plane depth {}", near[2]);
        assert!((far[2] - 1.0).abs() < 1e-5, "far plane depth {}", far[2]);
        // Points at the frustum edge map to NDC ±1.
        let edge = transform(&m, [15.0, 3.0, -4.0]);
        assert!((edge[0].abs() - 1.0).abs() < 1e-5 || (edge[1].abs() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn light_matrix_origin_variant_matches_at_origin() {
        let d = [0.3, -1.0, 0.2];
        assert_eq!(
            directional_light_matrix(d, 8.0, -10.0, 10.0),
            directional_light_matrix_at(d, [0.0; 3], 8.0, -10.0, 10.0)
        );
    }

    #[test]
    fn light_view_is_not_mirrored() {
        // A ground triangle wound CCW around +Y faces a light shining down.
        // In light NDC (x right, y up) it must stay CCW so that the shadow
        // pipeline's back-face culling keeps light-facing surfaces.
        for d in [[0.0, -1.0, 0.0], [0.4, -1.0, 0.1], [0.0, -0.5, -1.0]] {
            let m = directional_light_matrix_at(d, [0.0; 3], 10.0, -10.0, 10.0);
            let tri = [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]]; // CCW around +Y
            let p: Vec<[f32; 4]> = tri.iter().map(|&v| transform(&m, v)).collect();
            let area = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1])
                - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
            assert!(
                area > 0.0,
                "light-facing triangle turned clockwise for d={d:?}"
            );
        }
    }

    #[test]
    fn light_matrix_rejects_bad_inputs() {
        assert_eq!(
            directional_light_matrix(d3(), f32::NAN, 0.0, 1.0),
            IDENTITY_MAT4
        );
        assert_eq!(
            directional_light_matrix(d3(), 1.0, 1.0, f32::NAN),
            IDENTITY_MAT4
        );
    }

    fn d3() -> [f32; 3] {
        [0.0, -1.0, 0.0]
    }

    #[test]
    fn atlas_tile_math_never_divides_by_zero() {
        let cfg = ShadowAtlasConfig {
            size: 512,
            tile_size: 1024,
        };
        assert_eq!(tile_viewport(&cfg, 0), (0, 0, 1024, 1024));
        assert_eq!(tile_viewport(&cfg, 3), (0, 3 * 1024, 1024, 1024));
        let uv = tile_uv(&cfg, 1);
        assert!(uv.iter().all(|v| v.is_finite()));
        let zero = ShadowAtlasConfig {
            size: 0,
            tile_size: 0,
        };
        let _ = tile_viewport(&zero, 7);
        assert!(tile_uv(&zero, 7).iter().all(|v| v.is_finite()));
        // Huge index: saturates instead of overflowing.
        let _ = tile_viewport(
            &ShadowAtlasConfig {
                size: 4096,
                tile_size: 1,
            },
            u32::MAX,
        );
    }

    #[test]
    fn practical_splits_sanitize_inputs() {
        for (near, far, lambda) in [
            (0.1, -5.0, 0.5),
            (f32::NAN, 100.0, 0.5),
            (0.1, f32::NAN, 0.5),
            (0.1, 100.0, f32::NAN),
            (0.1, 100.0, 7.0),
            (-1.0, f32::INFINITY, -3.0),
        ] {
            let s = compute_practical_splits(near, far, 4, lambda);
            assert_eq!(s.len(), 5);
            assert!(
                s.iter().all(|v| v.is_finite()),
                "{s:?} for {near},{far},{lambda}"
            );
            assert!(s.windows(2).all(|w| w[1] >= w[0]), "not monotonic: {s:?}");
        }
        assert_eq!(compute_practical_splits(1.0, 10.0, 0, 0.5).len(), 2);
    }

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

    #[test]
    fn cascade_splits_zero_near() {
        // Division-by-zero regression: near=0 should not produce NaN
        let splits = compute_practical_splits(0.0, 100.0, 4, 0.5);
        assert_eq!(splits.len(), 5);
        for &s in &splits {
            assert!(!s.is_nan(), "split contains NaN");
            assert!(!s.is_infinite(), "split contains Inf");
        }
        // near should be clamped to 0.001
        assert!((splits[0] - 0.001).abs() < f32::EPSILON);
        assert_eq!(splits[4], 100.0);
        // Splits should be monotonically increasing
        for i in 1..splits.len() {
            assert!(splits[i] > splits[i - 1]);
        }
    }
}
