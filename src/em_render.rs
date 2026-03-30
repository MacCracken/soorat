//! Electromagnetism visualization — FDTD field heatmaps, field line traces,
//! point charges, radiation patterns, and vector field arrows.
//!
//! Requires feature: `em` (dep: bijli).

use crate::color::Color;
#[cfg(feature = "em")]
use crate::debug_draw::LineBatch;
#[cfg(feature = "em")]
use crate::vertex::Vertex3D;

/// Color mode for electromagnetic field visualization.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmColorMode {
    /// Blue→cyan→green→yellow→red gradient based on field magnitude.
    Magnitude,
    /// Signed coloring: blue for negative, red for positive field values.
    Signed,
}

/// Parameters for EM field visualization.
pub struct EmVisParams {
    /// Color mapping mode.
    pub color_mode: EmColorMode,
    /// Base color for solid-color fallback.
    pub base_color: Color,
    /// Alpha for generated geometry.
    pub alpha: f32,
}

impl Default for EmVisParams {
    fn default() -> Self {
        Self {
            color_mode: EmColorMode::Magnitude,
            base_color: Color::CORNFLOWER_BLUE,
            alpha: 1.0,
        }
    }
}

// ── CPU-only helpers (no feature gate) ──────────────────────────────────────

use crate::color::{signed_value_color, visualization_heat_map};
use crate::math_util::normal_to_basis;

/// EM heat map exposed for general use.
#[must_use]
pub fn em_heat_map(t: f32) -> Color {
    visualization_heat_map(t)
}

// ── 1. FDTD field heatmap ─────────────────────────────────────────────────

/// Generate a mesh for a 2D FDTD field slice as a colored heatmap.
///
/// Each grid cell becomes a colored quad on the XZ plane at `y_pos`.
/// Field values are normalized to \[0,1\] for color mapping.
#[must_use]
#[cfg(feature = "em")]
pub fn field_slice_2d_to_mesh(
    slice: &bijli::integration::soorat::FieldSlice2D,
    y_pos: f32,
    params: &EmVisParams,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let [nx, ny] = slice.dimensions;
    if nx == 0 || ny == 0 || slice.values.is_empty() {
        return (Vec::new(), Vec::new());
    }

    // Find min/max for normalization
    let (mut min_val, mut max_val) = (f64::MAX, f64::MIN);
    for &v in &slice.values {
        if v < min_val {
            min_val = v;
        }
        if v > max_val {
            max_val = v;
        }
    }
    let range = (max_val - min_val).max(f64::EPSILON);

    let mut vertices = Vec::with_capacity(nx * ny * 4);
    let mut indices = Vec::with_capacity(nx * ny * 6);

    let ox = slice.origin[0] as f32;
    let oz = slice.origin[1] as f32;
    let sp = slice.spacing as f32;

    for iy in 0..ny {
        for ix in 0..nx {
            let idx = iy * nx + ix;
            let v = if idx < slice.values.len() {
                slice.values[idx]
            } else {
                0.0
            };

            let color = match params.color_mode {
                EmColorMode::Magnitude => {
                    let t = ((v - min_val) / range) as f32;
                    let mut c = visualization_heat_map(t);
                    c.a = params.alpha;
                    c
                }
                EmColorMode::Signed => {
                    let t = if range > f64::EPSILON {
                        ((v - (min_val + max_val) * 0.5) / (range * 0.5)) as f32
                    } else {
                        0.0
                    };
                    let mut c = signed_value_color(t);
                    c.a = params.alpha;
                    c
                }
            };
            let c = color.to_array();

            let x0 = ox + ix as f32 * sp;
            let z0 = oz + iy as f32 * sp;

            let base = match (vertices.len() as u64).checked_mul(1) {
                Some(v) if v <= u32::MAX as u64 => vertices.len() as u32,
                _ => continue,
            };

            vertices.push(Vertex3D {
                position: [x0, y_pos, z0],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [ix as f32 / nx.max(1) as f32, iy as f32 / ny.max(1) as f32],
                color: c,
            });
            vertices.push(Vertex3D {
                position: [x0 + sp, y_pos, z0],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [
                    (ix + 1) as f32 / nx.max(1) as f32,
                    iy as f32 / ny.max(1) as f32,
                ],
                color: c,
            });
            vertices.push(Vertex3D {
                position: [x0 + sp, y_pos, z0 + sp],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [
                    (ix + 1) as f32 / nx.max(1) as f32,
                    (iy + 1) as f32 / ny.max(1) as f32,
                ],
                color: c,
            });
            vertices.push(Vertex3D {
                position: [x0, y_pos, z0 + sp],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [
                    ix as f32 / nx.max(1) as f32,
                    (iy + 1) as f32 / ny.max(1) as f32,
                ],
                color: c,
            });

            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);
            indices.push(base + 2);
            indices.push(base + 3);
            indices.push(base);
        }
    }

    (vertices, indices)
}

/// Generate a mesh for a 3D FDTD field at a specific Z-slice as a colored heatmap.
///
/// `z_index` selects which Z-layer to render (0..dimensions\[2\]).
#[must_use]
#[cfg(feature = "em")]
pub fn field_slice_3d_to_mesh(
    slice: &bijli::integration::soorat::FieldSlice3D,
    z_index: usize,
    y_pos: f32,
    params: &EmVisParams,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let [nx, ny, nz] = slice.dimensions;
    if nx == 0 || ny == 0 || nz == 0 || z_index >= nz || slice.values.is_empty() {
        return (Vec::new(), Vec::new());
    }

    // Extract the Z-slice values
    let mut slice_vals = Vec::with_capacity(nx * ny);
    for iy in 0..ny {
        for ix in 0..nx {
            let idx = z_index * ny * nx + iy * nx + ix;
            let v = if idx < slice.values.len() {
                slice.values[idx]
            } else {
                0.0
            };
            slice_vals.push(v);
        }
    }

    // Find min/max for normalization
    let (mut min_val, mut max_val) = (f64::MAX, f64::MIN);
    for &v in &slice_vals {
        if v < min_val {
            min_val = v;
        }
        if v > max_val {
            max_val = v;
        }
    }
    let range = (max_val - min_val).max(f64::EPSILON);

    let mut vertices = Vec::with_capacity(nx * ny * 4);
    let mut indices = Vec::with_capacity(nx * ny * 6);

    let ox = slice.origin[0] as f32;
    let oz = slice.origin[1] as f32;
    let sp = slice.spacing as f32;

    for iy in 0..ny {
        for ix in 0..nx {
            let v = slice_vals[iy * nx + ix];

            let color = match params.color_mode {
                EmColorMode::Magnitude => {
                    let t = ((v - min_val) / range) as f32;
                    let mut c = visualization_heat_map(t);
                    c.a = params.alpha;
                    c
                }
                EmColorMode::Signed => {
                    let t = if range > f64::EPSILON {
                        ((v - (min_val + max_val) * 0.5) / (range * 0.5)) as f32
                    } else {
                        0.0
                    };
                    let mut c = signed_value_color(t);
                    c.a = params.alpha;
                    c
                }
            };
            let c = color.to_array();

            let x0 = ox + ix as f32 * sp;
            let z0 = oz + iy as f32 * sp;

            let base = match (vertices.len() as u64).checked_mul(1) {
                Some(v) if v <= u32::MAX as u64 => vertices.len() as u32,
                _ => continue,
            };

            vertices.push(Vertex3D {
                position: [x0, y_pos, z0],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [ix as f32 / nx.max(1) as f32, iy as f32 / ny.max(1) as f32],
                color: c,
            });
            vertices.push(Vertex3D {
                position: [x0 + sp, y_pos, z0],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [
                    (ix + 1) as f32 / nx.max(1) as f32,
                    iy as f32 / ny.max(1) as f32,
                ],
                color: c,
            });
            vertices.push(Vertex3D {
                position: [x0 + sp, y_pos, z0 + sp],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [
                    (ix + 1) as f32 / nx.max(1) as f32,
                    (iy + 1) as f32 / ny.max(1) as f32,
                ],
                color: c,
            });
            vertices.push(Vertex3D {
                position: [x0, y_pos, z0 + sp],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [
                    ix as f32 / nx.max(1) as f32,
                    (iy + 1) as f32 / ny.max(1) as f32,
                ],
                color: c,
            });

            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);
            indices.push(base + 2);
            indices.push(base + 3);
            indices.push(base);
        }
    }

    (vertices, indices)
}

// ── 2. Field line rendering ───────────────────────────────────────────────

/// Draw field line traces as colored polylines in a [`LineBatch`].
///
/// Each field line is rendered as connected line segments. Color is mapped
/// from field magnitude at each point using the heat map gradient.
#[cfg(feature = "em")]
pub fn field_lines_to_lines(
    viz: &bijli::integration::soorat::FieldLineVisualization,
    batch: &mut LineBatch,
) {
    for line in &viz.lines {
        if line.points.len() < 2 {
            continue;
        }

        // Find max magnitude for normalization
        let max_mag = line
            .magnitudes
            .iter()
            .cloned()
            .fold(0.0_f64, f64::max)
            .max(f64::EPSILON);

        for i in 0..line.points.len() - 1 {
            let p0 = [
                line.points[i][0] as f32,
                line.points[i][1] as f32,
                line.points[i][2] as f32,
            ];
            let p1 = [
                line.points[i + 1][0] as f32,
                line.points[i + 1][1] as f32,
                line.points[i + 1][2] as f32,
            ];

            let mag = if i < line.magnitudes.len() {
                line.magnitudes[i]
            } else {
                0.0
            };
            let t = (mag / max_mag) as f32;
            let color = visualization_heat_map(t);

            batch.line(p0, p1, color);
        }
    }
}

// ── 3. Point charge visualization ─────────────────────────────────────────

/// Draw point charges as colored wireframe spheres with charge-based sizing.
///
/// Positive charges are red, negative charges are blue. Sphere radius scales
/// with `|charge| * size_scale`. Each charge gets a small wireframe halo.
#[cfg(feature = "em")]
pub fn charges_to_lines(
    viz: &bijli::integration::soorat::ChargeVisualization,
    size_scale: f32,
    segments: u32,
    batch: &mut LineBatch,
) {
    let segments = segments.max(4);

    for cp in &viz.charges {
        let color = if cp.charge >= 0.0 {
            Color::RED
        } else {
            Color::new(0.2, 0.3, 1.0, 1.0)
        };

        let radius = cp.magnitude * size_scale;
        let radius = radius.max(0.01); // minimum visible size

        batch.wire_sphere(cp.position, radius, segments, color);
    }
}

// ── 4. Radiation pattern ──────────────────────────────────────────────────

/// Generate a 3D mesh for a radiation pattern as a polar balloon.
///
/// The pattern is rendered as a deformed sphere where the radius at each
/// angle is proportional to the pattern magnitude. `base_radius` is the
/// minimum radius, `gain_scale` controls deformation amplitude.
#[must_use]
#[cfg(feature = "em")]
pub fn radiation_pattern_to_mesh(
    pattern: &bijli::integration::soorat::RadiationPattern,
    center: [f32; 3],
    base_radius: f32,
    gain_scale: f32,
    lat_segments: u32,
    lon_segments: u32,
) -> (Vec<Vertex3D>, Vec<u32>) {
    use std::f32::consts::PI;

    if lat_segments < 2 || lon_segments < 3 || pattern.angles.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let max_gain = if pattern.max_gain > 0.0 {
        pattern.max_gain as f32
    } else {
        1.0
    };

    let vert_count = (lat_segments + 1) * (lon_segments + 1);
    let mut vertices = Vec::with_capacity(vert_count as usize);
    let mut indices = Vec::with_capacity((lat_segments as usize) * (lon_segments as usize) * 6);

    for lat in 0..=lat_segments {
        let theta = PI * lat as f32 / lat_segments as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=lon_segments {
            let phi = 2.0 * PI * lon as f32 / lon_segments as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            // Direction on unit sphere
            let dir = [sin_theta * cos_phi, cos_theta, sin_theta * sin_phi];

            // Look up gain at this angle (nearest-neighbor on theta)
            let gain = pattern_gain_at_angle(pattern, theta) / max_gain;
            let r = base_radius + gain * gain_scale;

            let pos = [
                center[0] + dir[0] * r,
                center[1] + dir[1] * r,
                center[2] + dir[2] * r,
            ];

            // Color by normalized gain
            let t = gain.clamp(0.0, 1.0);
            let c = visualization_heat_map(t).to_array();

            vertices.push(Vertex3D {
                position: pos,
                normal: dir,
                tex_coords: [
                    lon as f32 / lon_segments as f32,
                    lat as f32 / lat_segments as f32,
                ],
                color: c,
            });
        }
    }

    // Indices
    let stride = lon_segments + 1;
    for lat in 0..lat_segments {
        for lon in 0..lon_segments {
            let tl = lat * stride + lon;
            let tr = tl + 1;
            let bl = (lat + 1) * stride + lon;
            let br = bl + 1;
            indices.push(tl);
            indices.push(bl);
            indices.push(tr);
            indices.push(tr);
            indices.push(bl);
            indices.push(br);
        }
    }

    (vertices, indices)
}

/// Nearest-neighbor gain lookup for a radiation pattern at a given angle.
#[cfg(feature = "em")]
fn pattern_gain_at_angle(
    pattern: &bijli::integration::soorat::RadiationPattern,
    theta: f32,
) -> f32 {
    if pattern.angles.is_empty() || pattern.pattern.is_empty() {
        return 1.0;
    }

    let idx = pattern
        .angles
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            ((**a as f32 - theta).abs())
                .partial_cmp(&((**b as f32 - theta).abs()))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    if idx < pattern.pattern.len() {
        pattern.pattern[idx] as f32
    } else {
        1.0
    }
}

// ── 5. Vector field arrows ────────────────────────────────────────────────

/// Draw sampled vector field as arrow glyphs in a [`LineBatch`].
///
/// Each grid point gets a line segment (shaft) with an arrowhead.
/// Arrow length is proportional to vector magnitude, color is heat-mapped.
/// `arrow_scale` controls the overall arrow size.
#[cfg(feature = "em")]
pub fn vector_field_to_arrows(
    field: &bijli::integration::soorat::VectorFieldSample,
    arrow_scale: f32,
    batch: &mut LineBatch,
) {
    let [nx, ny, nz] = field.dimensions;
    if field.vectors.is_empty() || nx == 0 || ny == 0 || nz == 0 {
        return;
    }

    let max_mag = field.max_magnitude.max(f64::EPSILON) as f32;
    let sp = field.spacing as f32;
    let ox = field.origin[0] as f32;
    let oy = field.origin[1] as f32;
    let oz = field.origin[2] as f32;

    for iz in 0..nz {
        for iy in 0..ny {
            for ix in 0..nx {
                let idx = iz * ny * nx + iy * nx + ix;
                if idx >= field.vectors.len() {
                    continue;
                }

                let v = field.vectors[idx];
                let vx = v[0] as f32;
                let vy = v[1] as f32;
                let vz = v[2] as f32;
                let mag = (vx * vx + vy * vy + vz * vz).sqrt();

                if !mag.is_finite() || mag < f32::EPSILON {
                    continue;
                }

                // Normalized direction
                let dx = vx / mag;
                let dy = vy / mag;
                let dz = vz / mag;

                // Arrow length proportional to magnitude
                let len = (mag / max_mag) * arrow_scale;

                let base = [
                    ox + ix as f32 * sp,
                    oy + iy as f32 * sp,
                    oz + iz as f32 * sp,
                ];
                let tip = [base[0] + dx * len, base[1] + dy * len, base[2] + dz * len];

                let t = (mag / max_mag).clamp(0.0, 1.0);
                let color = visualization_heat_map(t);

                // Shaft
                batch.line(base, tip, color);

                // Arrowhead — two lines perpendicular to the direction
                let head_size = len * 0.2;
                let (right, up) = normal_to_basis([dx, dy, dz]);
                let back = [
                    tip[0] - dx * head_size,
                    tip[1] - dy * head_size,
                    tip[2] - dz * head_size,
                ];
                batch.line(
                    tip,
                    [
                        back[0] + right[0] * head_size,
                        back[1] + right[1] * head_size,
                        back[2] + right[2] * head_size,
                    ],
                    color,
                );
                batch.line(
                    tip,
                    [
                        back[0] - right[0] * head_size,
                        back[1] - right[1] * head_size,
                        back[2] - right[2] * head_size,
                    ],
                    color,
                );
                batch.line(
                    tip,
                    [
                        back[0] + up[0] * head_size,
                        back[1] + up[1] * head_size,
                        back[2] + up[2] * head_size,
                    ],
                    color,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Heat map tests ─────────────────────────────────────────────────────

    #[test]
    fn heat_map_endpoints() {
        let cold = visualization_heat_map(0.0);
        assert_eq!(cold.b, 1.0);
        let hot = visualization_heat_map(1.0);
        assert_eq!(hot.r, 1.0);
        assert_eq!(hot.g, 0.0);
    }

    #[test]
    fn heat_map_clamps() {
        let under = visualization_heat_map(-1.0);
        assert_eq!(under.b, 1.0);
        let over = visualization_heat_map(2.0);
        assert_eq!(over.r, 1.0);
    }

    #[test]
    fn heat_map_gradient_smooth() {
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let c = visualization_heat_map(t);
            assert!(c.r >= 0.0 && c.r <= 1.0);
            assert!(c.g >= 0.0 && c.g <= 1.0);
            assert!(c.b >= 0.0 && c.b <= 1.0);
            assert_eq!(c.a, 1.0);
        }
    }

    #[test]
    fn signed_field_color_zero() {
        let c = signed_value_color(0.0);
        assert_eq!(c.r, 0.0);
        assert_eq!(c.b, 0.0);
    }

    #[test]
    fn signed_field_color_positive() {
        let c = signed_value_color(1.0);
        assert_eq!(c.r, 1.0);
        assert_eq!(c.b, 0.0);
    }

    #[test]
    fn signed_field_color_negative() {
        let c = signed_value_color(-1.0);
        assert_eq!(c.r, 0.0);
        assert_eq!(c.b, 1.0);
    }

    #[test]
    fn em_heat_map_public() {
        let c = em_heat_map(0.5);
        assert_eq!(c.g, 1.0);
    }

    // ── Normal basis tests ─────────────────────────────────────────────────

    #[test]
    fn normal_to_basis_z_forward() {
        let (right, up) = normal_to_basis([0.0, 0.0, 1.0]);
        let dot_rn = right[2];
        let dot_un = up[2];
        assert!(dot_rn.abs() < 0.01);
        assert!(dot_un.abs() < 0.01);
    }

    #[test]
    fn normal_to_basis_y_up() {
        let (right, up) = normal_to_basis([0.0, 1.0, 0.0]);
        let dot_rn = right[1];
        let dot_un = up[1];
        assert!(dot_rn.abs() < 0.01);
        assert!(dot_un.abs() < 0.01);
    }

    #[test]
    fn normal_to_basis_orthogonal() {
        let (right, up) = normal_to_basis([0.577, 0.577, 0.577]);
        let dot = right[0] * up[0] + right[1] * up[1] + right[2] * up[2];
        assert!(dot.abs() < 0.01);
    }

    // ── Default params ─────────────────────────────────────────────────────

    #[test]
    fn em_vis_params_default() {
        let p = EmVisParams::default();
        assert_eq!(p.color_mode, EmColorMode::Magnitude);
        assert_eq!(p.alpha, 1.0);
    }

    // ── Feature-gated tests ────────────────────────────────────────────────

    #[cfg(feature = "em")]
    mod em_tests {
        use super::*;
        use crate::debug_draw::LineBatch;

        #[test]
        fn field_slice_2d_empty() {
            let slice = bijli::integration::soorat::FieldSlice2D {
                values: vec![],
                dimensions: [0, 0],
                origin: [0.0, 0.0],
                spacing: 1.0,
                component: "Ez".to_string(),
                step: 0,
            };
            let (v, i) = field_slice_2d_to_mesh(&slice, 0.0, &EmVisParams::default());
            assert!(v.is_empty());
            assert!(i.is_empty());
        }

        #[test]
        fn field_slice_2d_basic() {
            let slice = bijli::integration::soorat::FieldSlice2D {
                values: vec![0.0, 0.5, 0.3, 1.0],
                dimensions: [2, 2],
                origin: [0.0, 0.0],
                spacing: 1.0,
                component: "Ez".to_string(),
                step: 0,
            };
            let (v, i) = field_slice_2d_to_mesh(&slice, 0.0, &EmVisParams::default());
            assert_eq!(v.len(), 4 * 4); // 4 cells × 4 verts
            assert_eq!(i.len(), 4 * 6);
        }

        #[test]
        fn field_slice_2d_signed_mode() {
            let slice = bijli::integration::soorat::FieldSlice2D {
                values: vec![-1.0, 0.0, 0.0, 1.0],
                dimensions: [2, 2],
                origin: [0.0, 0.0],
                spacing: 0.5,
                component: "Ez".to_string(),
                step: 5,
            };
            let params = EmVisParams {
                color_mode: EmColorMode::Signed,
                ..EmVisParams::default()
            };
            let (v, _) = field_slice_2d_to_mesh(&slice, 1.0, &params);
            assert_eq!(v.len(), 16);
        }

        #[test]
        fn field_slice_3d_empty() {
            let slice = bijli::integration::soorat::FieldSlice3D {
                values: vec![],
                dimensions: [0, 0, 0],
                origin: [0.0; 3],
                spacing: 1.0,
                component: "ez".to_string(),
                step: 0,
            };
            let (v, i) = field_slice_3d_to_mesh(&slice, 0, 0.0, &EmVisParams::default());
            assert!(v.is_empty());
            assert!(i.is_empty());
        }

        #[test]
        fn field_slice_3d_basic() {
            let slice = bijli::integration::soorat::FieldSlice3D {
                values: vec![0.0; 8],
                dimensions: [2, 2, 2],
                origin: [0.0; 3],
                spacing: 1.0,
                component: "ez".to_string(),
                step: 10,
            };
            let (v, i) = field_slice_3d_to_mesh(&slice, 0, 0.0, &EmVisParams::default());
            assert_eq!(v.len(), 2 * 2 * 4);
            assert_eq!(i.len(), 2 * 2 * 6);
        }

        #[test]
        fn field_slice_3d_out_of_bounds() {
            let slice = bijli::integration::soorat::FieldSlice3D {
                values: vec![1.0; 8],
                dimensions: [2, 2, 2],
                origin: [0.0; 3],
                spacing: 1.0,
                component: "ez".to_string(),
                step: 0,
            };
            let (v, _) = field_slice_3d_to_mesh(&slice, 5, 0.0, &EmVisParams::default());
            assert!(v.is_empty());
        }

        #[test]
        fn field_lines_empty() {
            let viz = bijli::integration::soorat::FieldLineVisualization { lines: vec![] };
            let mut batch = LineBatch::new();
            field_lines_to_lines(&viz, &mut batch);
            assert!(batch.is_empty());
        }

        #[test]
        fn field_lines_single() {
            let viz = bijli::integration::soorat::FieldLineVisualization {
                lines: vec![bijli::integration::soorat::FieldLine {
                    points: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.5, 0.0]],
                    magnitudes: vec![1.0, 0.8, 0.5],
                }],
            };
            let mut batch = LineBatch::new();
            field_lines_to_lines(&viz, &mut batch);
            assert_eq!(batch.line_count(), 2);
        }

        #[test]
        fn field_lines_too_short() {
            let viz = bijli::integration::soorat::FieldLineVisualization {
                lines: vec![bijli::integration::soorat::FieldLine {
                    points: vec![[0.0, 0.0, 0.0]],
                    magnitudes: vec![1.0],
                }],
            };
            let mut batch = LineBatch::new();
            field_lines_to_lines(&viz, &mut batch);
            assert!(batch.is_empty());
        }

        #[test]
        fn charges_empty() {
            let viz = bijli::integration::soorat::ChargeVisualization { charges: vec![] };
            let mut batch = LineBatch::new();
            charges_to_lines(&viz, 1.0, 8, &mut batch);
            assert!(batch.is_empty());
        }

        #[test]
        fn charges_positive_negative() {
            let viz = bijli::integration::soorat::ChargeVisualization {
                charges: vec![
                    bijli::integration::soorat::ChargePoint {
                        position: [0.0, 0.0, 0.0],
                        charge: 1.0,
                        magnitude: 1.0,
                    },
                    bijli::integration::soorat::ChargePoint {
                        position: [2.0, 0.0, 0.0],
                        charge: -1.0,
                        magnitude: 1.0,
                    },
                ],
            };
            let mut batch = LineBatch::new();
            charges_to_lines(&viz, 0.5, 8, &mut batch);
            assert!(batch.line_count() > 0);
        }

        #[test]
        fn radiation_pattern_empty() {
            let pat = bijli::integration::soorat::RadiationPattern {
                angles: vec![],
                pattern: vec![],
                max_gain: 0.0,
            };
            let (v, i) = radiation_pattern_to_mesh(&pat, [0.0; 3], 1.0, 1.0, 4, 6);
            assert!(v.is_empty());
            assert!(i.is_empty());
        }

        #[test]
        fn radiation_pattern_basic() {
            let pat = bijli::integration::soorat::RadiationPattern {
                angles: vec![0.0, std::f64::consts::FRAC_PI_2, std::f64::consts::PI],
                pattern: vec![1.0, 0.5, 0.2],
                max_gain: 1.0,
            };
            let (v, i) = radiation_pattern_to_mesh(&pat, [0.0; 3], 0.5, 2.0, 4, 6);
            // (4+1) * (6+1) = 35 vertices
            assert_eq!(v.len(), 35);
            // 4 * 6 * 6 = 144 indices
            assert_eq!(i.len(), 144);
        }

        #[test]
        fn radiation_pattern_min_segments() {
            let pat = bijli::integration::soorat::RadiationPattern {
                angles: vec![0.0],
                pattern: vec![1.0],
                max_gain: 1.0,
            };
            let (v, i) = radiation_pattern_to_mesh(&pat, [0.0; 3], 1.0, 1.0, 1, 2);
            assert!(v.is_empty());
            assert!(i.is_empty());
        }

        #[test]
        fn vector_field_empty() {
            let field = bijli::integration::soorat::VectorFieldSample {
                vectors: vec![],
                dimensions: [0, 0, 0],
                origin: [0.0; 3],
                spacing: 1.0,
                max_magnitude: 0.0,
            };
            let mut batch = LineBatch::new();
            vector_field_to_arrows(&field, 1.0, &mut batch);
            assert!(batch.is_empty());
        }

        #[test]
        fn vector_field_basic() {
            let field = bijli::integration::soorat::VectorFieldSample {
                vectors: vec![
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [0.0, 0.0, 1.0],
                    [1.0, 1.0, 0.0],
                ],
                dimensions: [2, 2, 1],
                origin: [0.0; 3],
                spacing: 1.0,
                max_magnitude: std::f64::consts::SQRT_2,
            };
            let mut batch = LineBatch::new();
            vector_field_to_arrows(&field, 1.0, &mut batch);
            // 4 vectors × (1 shaft + 3 arrowhead lines) = 16 lines
            assert_eq!(batch.line_count(), 16);
        }

        #[test]
        fn vector_field_zero_vectors_skipped() {
            let field = bijli::integration::soorat::VectorFieldSample {
                vectors: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                dimensions: [2, 1, 1],
                origin: [0.0; 3],
                spacing: 1.0,
                max_magnitude: 1.0,
            };
            let mut batch = LineBatch::new();
            vector_field_to_arrows(&field, 1.0, &mut batch);
            // Only 1 non-zero vector × 4 lines = 4
            assert_eq!(batch.line_count(), 4);
        }

        #[test]
        fn field_slice_2d_mismatched_dimensions() {
            // Adversarial: dimensions say [3,3] but values has only 4 elements
            let slice = bijli::integration::soorat::FieldSlice2D {
                values: vec![0.0, 0.5, 0.3, 1.0],
                dimensions: [3, 3],
                origin: [0.0, 0.0],
                spacing: 1.0,
                component: "Ez".to_string(),
                step: 0,
            };
            // Must not panic — out-of-bounds indices fall back to 0.0
            let (v, i) = field_slice_2d_to_mesh(&slice, 0.0, &EmVisParams::default());
            // Should still produce geometry for the 3x3 grid
            assert_eq!(v.len(), 3 * 3 * 4);
            assert_eq!(i.len(), 3 * 3 * 6);
            // No NaN in vertex positions
            for vert in &v {
                for &p in &vert.position {
                    assert!(!p.is_nan(), "vertex position contains NaN");
                }
            }
        }

        #[test]
        fn vector_field_nan_magnitude() {
            // Adversarial: NaN in vector components must not panic
            let field = bijli::integration::soorat::VectorFieldSample {
                vectors: vec![[f64::NAN, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, f64::NAN, 0.0]],
                dimensions: [3, 1, 1],
                origin: [0.0; 3],
                spacing: 1.0,
                max_magnitude: 1.0,
            };
            let mut batch = LineBatch::new();
            // Must not panic
            vector_field_to_arrows(&field, 1.0, &mut batch);
            // At least the valid vector [1,0,0] should produce arrows
            assert!(batch.line_count() >= 4);
        }
    }
}
