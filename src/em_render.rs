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

use crate::color::visualization_heat_map;
#[cfg(feature = "em")]
use crate::math_util::normal_to_basis;
#[cfg(feature = "em")]
use crate::viz::{
    MAX_SEGMENTS, XzGrid, finite_min_max, grid_is_backed, heat_rgba, lattice_indices, magnitude_t,
    signed_rgba, signed_t, xz_heatmap_mesh,
};

/// EM heat map exposed for general use.
#[must_use]
pub fn em_heat_map(t: f32) -> Color {
    visualization_heat_map(t)
}

// ── 1. FDTD field heatmap ─────────────────────────────────────────────────

/// Color for one field sample under `mode`, given the slice's finite range.
#[cfg(feature = "em")]
#[must_use]
#[inline]
fn field_rgba(mode: EmColorMode, v: f64, min: f64, max: f64, alpha: f32) -> [f32; 4] {
    match mode {
        EmColorMode::Magnitude => heat_rgba(magnitude_t(v, min, max), alpha),
        EmColorMode::Signed => signed_rgba(signed_t(v, min, max), alpha),
    }
}

/// Generate a mesh for a 2D FDTD field slice as a colored heatmap.
///
/// Each grid cell becomes a colored, +Y-facing quad on the XZ plane at
/// `y_pos` (simulation x → world X, simulation y → world Z).
/// [`EmColorMode::Magnitude`] colors `|v|` relative to the largest magnitude;
/// [`EmColorMode::Signed`] is a diverging map centred on zero. Non-finite
/// samples render as the zero color.
///
/// Returns an empty mesh if `values` holds fewer than `nx * ny` samples.
#[must_use]
#[cfg(feature = "em")]
pub fn field_slice_2d_to_mesh(
    slice: &bijli::integration::soorat::FieldSlice2D,
    y_pos: f32,
    params: &EmVisParams,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let [nx, ny] = slice.dimensions;
    if !grid_is_backed(&slice.dimensions, slice.values.len(), "field_slice_2d") {
        return (Vec::new(), Vec::new());
    }
    tracing::debug!(nx, ny, component = %slice.component, "building 2D field heatmap");

    let cells = &slice.values[..nx * ny];
    let (min, max) = finite_min_max(cells.iter().copied()).unwrap_or((0.0, 0.0));
    let sp = slice.spacing as f32;
    let grid = XzGrid {
        nx,
        nz: ny,
        origin_x: slice.origin[0] as f32,
        origin_z: slice.origin[1] as f32,
        y: y_pos,
        dx: sp,
        dz: sp,
    };
    xz_heatmap_mesh(&grid, |ix, iy| {
        field_rgba(
            params.color_mode,
            cells[iy * nx + ix],
            min,
            max,
            params.alpha,
        )
    })
}

/// Generate a mesh for a 3D FDTD field at a specific Z-slice as a colored heatmap.
///
/// `z_index` selects which Z-layer to render (0..dimensions\[2\]). Coloring
/// and placement match [`field_slice_2d_to_mesh`].
///
/// Returns an empty mesh if `z_index` is out of range or `values` holds fewer
/// than `nx * ny * nz` samples.
#[must_use]
#[cfg(feature = "em")]
pub fn field_slice_3d_to_mesh(
    slice: &bijli::integration::soorat::FieldSlice3D,
    z_index: usize,
    y_pos: f32,
    params: &EmVisParams,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let [nx, ny, nz] = slice.dimensions;
    if z_index >= nz || !grid_is_backed(&slice.dimensions, slice.values.len(), "field_slice_3d") {
        return (Vec::new(), Vec::new());
    }
    tracing::debug!(nx, ny, z_index, "building 3D field slice heatmap");

    // In bounds: nx * ny * nz <= values.len() was checked above.
    let layer_start = z_index * ny * nx;
    let layer = &slice.values[layer_start..layer_start + nx * ny];
    let (min, max) = finite_min_max(layer.iter().copied()).unwrap_or((0.0, 0.0));
    let sp = slice.spacing as f32;
    let grid = XzGrid {
        nx,
        nz: ny,
        origin_x: slice.origin[0] as f32,
        origin_z: slice.origin[1] as f32,
        y: y_pos,
        dx: sp,
        dz: sp,
    };
    xz_heatmap_mesh(&grid, |ix, iy| {
        field_rgba(
            params.color_mode,
            layer[iy * nx + ix],
            min,
            max,
            params.alpha,
        )
    })
}

// ── 2. Field line rendering ───────────────────────────────────────────────

/// Draw field line traces as colored polylines in a [`LineBatch`].
///
/// Each field line is rendered as connected line segments. Color is mapped
/// from field magnitude at each point using the heat map gradient, normalized
/// by the largest finite magnitude across **all** lines (so strong and weak
/// lines are comparable). Segments with non-finite endpoints are skipped.
#[cfg(feature = "em")]
pub fn field_lines_to_lines(
    viz: &bijli::integration::soorat::FieldLineVisualization,
    batch: &mut LineBatch,
) {
    tracing::debug!(lines = viz.lines.len(), "building field lines");
    let max_mag = finite_min_max(viz.lines.iter().flat_map(|l| l.magnitudes.iter().copied()))
        .map_or(0.0, |(_, hi)| hi);

    let to_f32 = |p: [f64; 3]| [p[0] as f32, p[1] as f32, p[2] as f32];
    for line in &viz.lines {
        for (i, pair) in line.points.windows(2).enumerate() {
            let (p0, p1) = (to_f32(pair[0]), to_f32(pair[1]));
            if !p0.iter().chain(p1.iter()).all(|c| c.is_finite()) {
                continue;
            }
            let mag = line.magnitudes.get(i).copied().unwrap_or(0.0);
            let t = if max_mag > 0.0 {
                (mag / max_mag) as f32
            } else {
                0.0
            };
            batch.line(p0, p1, visualization_heat_map(t));
        }
    }
}

// ── 3. Point charge visualization ─────────────────────────────────────────

/// Draw point charges as colored wireframe spheres with charge-based sizing.
///
/// Positive charges are red, negative charges are blue. Sphere radius scales
/// with `|charge| * size_scale` (minimum 0.01). Each charge gets a small
/// wireframe halo; `segments` is clamped to `4..=1024`.
#[cfg(feature = "em")]
pub fn charges_to_lines(
    viz: &bijli::integration::soorat::ChargeVisualization,
    size_scale: f32,
    segments: u32,
    batch: &mut LineBatch,
) {
    let segments = segments.clamp(4, MAX_SEGMENTS);
    tracing::debug!(
        charges = viz.charges.len(),
        segments,
        "building charge halos"
    );

    for cp in &viz.charges {
        let color = if cp.charge >= 0.0 {
            Color::RED
        } else {
            Color::new(0.2, 0.3, 1.0, 1.0)
        };

        // f32::max ignores NaN, so a NaN magnitude also falls back to 0.01.
        let radius = (cp.magnitude * size_scale).max(0.01);

        batch.wire_sphere(cp.position, radius, segments, color);
    }
}

// ── 4. Radiation pattern ──────────────────────────────────────────────────

/// Generate a 3D mesh for a radiation pattern as a polar balloon.
///
/// bijli's far-field pattern is sampled over the in-plane azimuth φ
/// (0..2π, from +x toward +y in the simulation plane). It is drawn on the
/// world XZ plane (+X toward +Z, matching the heatmaps' sim-y → world-Z
/// mapping): the radius at longitude φ and polar angle θ is
/// `base_radius + gain(φ) / max_gain · gain_scale · sin θ`, using the nearest
/// sample with wrap-around. The deformation is full in the simulation plane
/// (equator) and fades to `base_radius` at the poles, where φ is undefined.
///
/// Segment counts are clamped to `1024`; triangles face outward.
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
    use std::f32::consts::{PI, TAU};

    if lat_segments < 2 || lon_segments < 3 || pattern.angles.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let lat_segments = lat_segments.min(MAX_SEGMENTS);
    let lon_segments = lon_segments.min(MAX_SEGMENTS);
    tracing::debug!(
        samples = pattern.angles.len(),
        lat_segments,
        lon_segments,
        "building radiation pattern balloon"
    );

    let max_gain = if pattern.max_gain.is_finite() && pattern.max_gain > 0.0 {
        pattern.max_gain as f32
    } else {
        finite_min_max(pattern.pattern.iter().copied())
            .map(|(_, hi)| hi as f32)
            .filter(|&hi| hi > 0.0)
            .unwrap_or(1.0)
    };

    // Gain depends only on longitude — resolve it once per column.
    let lon_gain: Vec<f32> = (0..=lon_segments)
        .map(|lon| {
            let phi = TAU * lon as f32 / lon_segments as f32;
            (pattern_gain_at_azimuth(pattern, phi) / max_gain).clamp(0.0, 1.0)
        })
        .collect();

    let mut vertices =
        Vec::with_capacity((lat_segments as usize + 1) * (lon_segments as usize + 1));
    for lat in 0..=lat_segments {
        let theta = PI * lat as f32 / lat_segments as f32;
        let (sin_theta, cos_theta) = theta.sin_cos();

        for (lon, &gain) in lon_gain.iter().enumerate() {
            let phi = TAU * lon as f32 / lon_segments as f32;
            let (sin_phi, cos_phi) = phi.sin_cos();
            let dir = [sin_theta * cos_phi, cos_theta, sin_theta * sin_phi];
            // In-plane pattern: full deformation at the equator, none at the poles.
            let gain = gain * sin_theta.max(0.0);
            let r = base_radius + gain * gain_scale;

            vertices.push(Vertex3D {
                position: [
                    center[0] + dir[0] * r,
                    center[1] + dir[1] * r,
                    center[2] + dir[2] * r,
                ],
                normal: dir,
                tex_coords: [
                    lon as f32 / lon_segments as f32,
                    lat as f32 / lat_segments as f32,
                ],
                color: visualization_heat_map(gain).to_array(),
            });
        }
    }

    (vertices, lattice_indices(lat_segments, lon_segments))
}

/// Nearest-neighbor gain lookup at azimuth `phi` with 2π wrap-around.
/// Missing or non-finite samples read as 0.
#[cfg(feature = "em")]
#[must_use]
#[inline]
fn pattern_gain_at_azimuth(
    pattern: &bijli::integration::soorat::RadiationPattern,
    phi: f32,
) -> f32 {
    use std::f32::consts::TAU;
    let circular = |a: f64| {
        let d = (a as f32 - phi).rem_euclid(TAU);
        d.min(TAU - d)
    };
    pattern
        .angles
        .iter()
        .enumerate()
        .filter(|(_, a)| a.is_finite())
        .min_by(|(_, a), (_, b)| circular(**a).total_cmp(&circular(**b)))
        .and_then(|(i, _)| pattern.pattern.get(i))
        .map(|&g| g as f32)
        .filter(|g| g.is_finite())
        .unwrap_or(0.0)
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
    if !grid_is_backed(
        &field.dimensions,
        field.vectors.len(),
        "vector_field_to_arrows",
    ) {
        return;
    }
    tracing::debug!(nx, ny, nz, "building vector field arrows");

    let magnitude = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    // Trust the stored maximum only if it is usable; otherwise measure it.
    let max_mag = if field.max_magnitude.is_finite() && field.max_magnitude > 0.0 {
        field.max_magnitude as f32
    } else {
        finite_min_max(field.vectors.iter().map(|&v| magnitude(v))).map_or(0.0, |(_, hi)| hi as f32)
    }
    .max(f32::EPSILON);
    let sp = field.spacing as f32;
    let ox = field.origin[0] as f32;
    let oy = field.origin[1] as f32;
    let oz = field.origin[2] as f32;

    for iz in 0..nz {
        for iy in 0..ny {
            for ix in 0..nx {
                // In bounds: nx * ny * nz <= vectors.len() was checked above.
                let v = field.vectors[iz * ny * nx + iy * nx + ix];
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

                // Arrow length proportional to magnitude, never longer than
                // `arrow_scale` (even if the stored maximum is stale).
                let t = (mag / max_mag).clamp(0.0, 1.0);
                let len = t * arrow_scale;

                let base = [
                    ox + ix as f32 * sp,
                    oy + iy as f32 * sp,
                    oz + iz as f32 * sp,
                ];
                let tip = [base[0] + dx * len, base[1] + dy * len, base[2] + dz * len];

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
    use crate::color::signed_value_color;
    use crate::math_util::normal_to_basis;

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

        use crate::viz::assert_winding_matches_normals;

        fn slice(
            values: Vec<f64>,
            nx: usize,
            ny: usize,
        ) -> bijli::integration::soorat::FieldSlice2D {
            bijli::integration::soorat::FieldSlice2D {
                values,
                dimensions: [nx, ny],
                origin: [0.0, 0.0],
                spacing: 1.0,
                component: "Ez".to_string(),
                step: 0,
            }
        }

        #[test]
        fn field_slice_winding_and_non_finite() {
            let s = slice(vec![f64::NAN, -1.0, f64::INFINITY, 1.0], 2, 2);
            for mode in [EmColorMode::Magnitude, EmColorMode::Signed] {
                let params = EmVisParams {
                    color_mode: mode,
                    ..EmVisParams::default()
                };
                let (v, i) = field_slice_2d_to_mesh(&s, 0.0, &params);
                assert_eq!(v.len(), 16);
                assert!(v.iter().all(|x| x.color.iter().all(|c| c.is_finite())));
                assert_winding_matches_normals(&v, &i);
            }
        }

        #[test]
        fn magnitude_mode_uses_absolute_value() {
            let s = slice(vec![-1.0, 1.0], 2, 1);
            let (v, _) = field_slice_2d_to_mesh(&s, 0.0, &EmVisParams::default());
            assert_eq!(v[0].color, v[4].color, "|-1| and |1| must share a color");
        }

        #[test]
        fn radiation_pattern_uses_azimuth_with_wraparound() {
            use std::f64::consts::{FRAC_PI_2, PI};
            // Single lobe at φ = 3π/2 — only reachable via azimuth lookup.
            let pattern = bijli::integration::soorat::RadiationPattern {
                angles: vec![0.0, FRAC_PI_2, PI, 3.0 * FRAC_PI_2],
                pattern: vec![0.0, 0.0, 0.0, 1.0],
                max_gain: 1.0,
            };
            let (v, i) = radiation_pattern_to_mesh(&pattern, [0.0; 3], 1.0, 1.0, 4, 8);
            assert_winding_matches_normals(&v, &i);
            // Equator row (lat = 2 of 4), lon index 6 of 8 → φ = 3π/2 → radius 2.
            let at = |lon: usize| {
                let p = v[2 * 9 + lon].position;
                (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt()
            };
            assert!((at(6) - 2.0).abs() < 1e-4, "lobe radius {}", at(6));
            assert!((at(2) - 1.0).abs() < 1e-4, "off-lobe radius {}", at(2));
            // φ just below 2π wraps to the sample at 0, not to 3π/2.
            assert!((at(8) - 1.0).abs() < 1e-4);
        }

        #[test]
        fn radiation_pattern_bad_max_gain_is_measured() {
            let pattern = bijli::integration::soorat::RadiationPattern {
                angles: vec![0.0, 3.0],
                pattern: vec![4.0, 2.0],
                max_gain: 0.0,
            };
            let (v, _) = radiation_pattern_to_mesh(&pattern, [0.0; 3], 1.0, 1.0, 2, 3);
            assert!(v.iter().all(|x| x.position.iter().all(|c| c.is_finite())));
            let r = v.iter().map(|x| {
                let p = x.position;
                (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt()
            });
            assert!(
                r.fold(0.0f32, f32::max) <= 2.0 + 1e-4,
                "gain normalized by measured max"
            );
        }

        #[test]
        fn field_lines_share_one_normalization() {
            let line = |m: f64| bijli::integration::soorat::FieldLine {
                points: vec![[0.0; 3], [1.0, 0.0, 0.0]],
                magnitudes: vec![m, m],
            };
            let viz = bijli::integration::soorat::FieldLineVisualization {
                lines: vec![line(1.0), line(10.0)],
            };
            let mut batch = LineBatch::new();
            field_lines_to_lines(&viz, &mut batch);
            assert_ne!(
                batch.vertices[0].color, batch.vertices[2].color,
                "weak and strong lines must not look identical"
            );
        }

        #[test]
        fn arrows_bounded_when_max_magnitude_is_stale() {
            let field = bijli::integration::soorat::VectorFieldSample {
                vectors: vec![[100.0, 0.0, 0.0]],
                dimensions: [1, 1, 1],
                origin: [0.0; 3],
                spacing: 1.0,
                max_magnitude: 0.0,
            };
            let mut batch = LineBatch::new();
            vector_field_to_arrows(&field, 2.0, &mut batch);
            let tip = batch.vertices[1].position;
            assert!((tip[0] - 2.0).abs() < 1e-5, "arrow length {}", tip[0]);
        }

        #[test]
        fn charges_clamp_segments() {
            let viz = bijli::integration::soorat::ChargeVisualization { charges: vec![] };
            let mut batch = LineBatch::new();
            charges_to_lines(&viz, 1.0, u32::MAX, &mut batch);
            assert!(batch.is_empty());
        }

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
            // Must not panic. A grid larger than its data is rejected (output
            // size stays proportional to input size) rather than zero-filled.
            let (v, i) = field_slice_2d_to_mesh(&slice, 0.0, &EmVisParams::default());
            assert!(v.is_empty());
            assert!(i.is_empty());

            // Huge header, no data: returns immediately without allocating.
            let huge = bijli::integration::soorat::FieldSlice2D {
                values: Vec::new(),
                dimensions: [1 << 20, 1 << 20],
                ..slice
            };
            let (v, _) = field_slice_2d_to_mesh(&huge, 0.0, &EmVisParams::default());
            assert!(v.is_empty());
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
