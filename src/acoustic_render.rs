//! Acoustic visualization — ray paths, pressure maps, room modes, portals,
//! directivity balloons, and coupled decay curves.
//!
//! Requires feature: `acoustics` (dep: goonj).

use crate::color::Color;
#[cfg(feature = "acoustics")]
use crate::debug_draw::LineBatch;
#[cfg(feature = "acoustics")]
use crate::vertex::Vertex3D;

/// Color mode for acoustic pressure visualization.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcousticColorMode {
    /// Blue→cyan→green→yellow→red gradient based on pressure magnitude.
    Pressure,
    /// Signed coloring: blue for negative pressure, red for positive.
    SignedPressure,
    /// Per-band energy coloring (averaged across octave bands).
    Energy,
}

/// Parameters for pressure and mode visualization.
pub struct AcousticVisParams {
    /// Color mapping mode.
    pub color_mode: AcousticColorMode,
    /// Base color for solid-color fallback.
    pub base_color: Color,
    /// Alpha for generated geometry.
    pub alpha: f32,
}

impl Default for AcousticVisParams {
    fn default() -> Self {
        Self {
            color_mode: AcousticColorMode::Pressure,
            base_color: Color::CORNFLOWER_BLUE,
            alpha: 1.0,
        }
    }
}

// ── CPU-only helpers (no feature gate) ──────────────────────────────────────

#[cfg(feature = "acoustics")]
use crate::color::signed_value_color;
use crate::color::visualization_heat_map;

/// Acoustic heat map exposed for general use.
#[must_use]
pub fn acoustic_heat_map(t: f32) -> Color {
    visualization_heat_map(t)
}

// ── 1. Ray path rendering ──────────────────────────────────────────────────

/// Add acoustic ray paths as colored line segments to a [`LineBatch`].
///
/// Each ray path is drawn as connected line segments from source through each
/// bounce point. Color fades from `start_color` to `end_color` based on
/// remaining energy (averaged across frequency bands).
#[cfg(feature = "acoustics")]
pub fn ray_paths_to_lines(
    ray_viz: &goonj::integration::soorat::RayVisualization,
    start_color: Color,
    end_color: Color,
    batch: &mut LineBatch,
) {
    let src: [f32; 3] = ray_viz.source.into();

    for path in &ray_viz.paths {
        if path.bounces.is_empty() {
            continue;
        }

        let mut prev = src;
        for bounce in &path.bounces {
            // Energy fraction: average across bands
            let energy_avg =
                bounce.energy_after.iter().sum::<f32>() / bounce.energy_after.len() as f32;
            let color = start_color.lerp(end_color, 1.0 - energy_avg.clamp(0.0, 1.0));

            let point: [f32; 3] = bounce.point.into();
            batch.line(prev, point, color);
            prev = point;
        }
    }
}

// ── 2. Pressure map heatmap ────────────────────────────────────────────────

/// Generate a mesh for a single XZ-plane slice of a 3D pressure map.
///
/// `y_index` selects which Y-layer to render (0..dimensions\[1\]).
/// Each grid cell becomes a colored quad.
#[must_use]
#[cfg(feature = "acoustics")]
pub fn pressure_map_slice(
    pressure: &goonj::integration::soorat::PressureMap,
    y_index: usize,
    params: &AcousticVisParams,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let [nx, ny, nz] = pressure.dimensions;
    if nx == 0 || ny == 0 || nz == 0 || y_index >= ny {
        return (Vec::new(), Vec::new());
    }

    // Find min/max for normalization
    let (mut min_val, mut max_val) = (f32::MAX, f32::MIN);
    for iz in 0..nz {
        for ix in 0..nx {
            let idx = iz * ny * nx + y_index * nx + ix;
            if idx < pressure.values.len() {
                let v = pressure.values[idx];
                if v < min_val {
                    min_val = v;
                }
                if v > max_val {
                    max_val = v;
                }
            }
        }
    }
    let range = (max_val - min_val).max(f32::EPSILON);

    let mut vertices = Vec::with_capacity(nx * nz * 4);
    let mut indices = Vec::with_capacity(nx * nz * 6);

    let origin: [f32; 3] = pressure.origin.into();
    let sp = pressure.spacing;
    let y_pos = origin[1] + y_index as f32 * sp;

    for iz in 0..nz {
        for ix in 0..nx {
            let idx = iz * ny * nx + y_index * nx + ix;
            let v = if idx < pressure.values.len() {
                pressure.values[idx]
            } else {
                0.0
            };

            let color = match params.color_mode {
                AcousticColorMode::Pressure | AcousticColorMode::Energy => {
                    let t = (v - min_val) / range;
                    let mut c = visualization_heat_map(t);
                    c.a = params.alpha;
                    c
                }
                AcousticColorMode::SignedPressure => {
                    let t = if range > f32::EPSILON {
                        (v - (min_val + max_val) * 0.5) / (range * 0.5)
                    } else {
                        0.0
                    };
                    let mut c = signed_value_color(t);
                    c.a = params.alpha;
                    c
                }
            };
            let c = color.to_array();

            let x0 = origin[0] + ix as f32 * sp;
            let z0 = origin[2] + iz as f32 * sp;

            let base = match (vertices.len() as u64).checked_mul(1) {
                Some(v) if v <= u32::MAX as u64 => vertices.len() as u32,
                _ => continue,
            };

            vertices.push(Vertex3D {
                position: [x0, y_pos, z0],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [ix as f32 / nx.max(1) as f32, iz as f32 / nz.max(1) as f32],
                color: c,
            });
            vertices.push(Vertex3D {
                position: [x0 + sp, y_pos, z0],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [
                    (ix + 1) as f32 / nx.max(1) as f32,
                    iz as f32 / nz.max(1) as f32,
                ],
                color: c,
            });
            vertices.push(Vertex3D {
                position: [x0 + sp, y_pos, z0 + sp],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [
                    (ix + 1) as f32 / nx.max(1) as f32,
                    (iz + 1) as f32 / nz.max(1) as f32,
                ],
                color: c,
            });
            vertices.push(Vertex3D {
                position: [x0, y_pos, z0 + sp],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [
                    ix as f32 / nx.max(1) as f32,
                    (iz + 1) as f32 / nz.max(1) as f32,
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

// ── 3. Room mode patterns ──────────────────────────────────────────────────

/// Generate a height-field mesh from a room mode visualization pattern.
///
/// The 2D pattern is mapped onto an XZ plane, with Y = pattern value × `height_scale`.
/// Vertex colors are heat-mapped from the absolute pattern value.
#[must_use]
#[cfg(feature = "acoustics")]
pub fn mode_pattern_to_mesh(
    mode: &goonj::integration::soorat::ModeVisualization,
    width: f32,
    depth: f32,
    height_scale: f32,
    params: &AcousticVisParams,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let [cols, rows] = mode.pattern_dimensions;
    if cols == 0 || rows == 0 || mode.pattern.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut vertices = Vec::with_capacity(cols * rows);
    let mut indices = Vec::with_capacity((cols.saturating_sub(1)) * (rows.saturating_sub(1)) * 6);

    let dx = if cols > 1 {
        width / (cols - 1) as f32
    } else {
        0.0
    };
    let dz = if rows > 1 {
        depth / (rows - 1) as f32
    } else {
        0.0
    };
    let x_offset = -width * 0.5;
    let z_offset = -depth * 0.5;

    for iz in 0..rows {
        for ix in 0..cols {
            let idx = iz * cols + ix;
            let v = if idx < mode.pattern.len() {
                mode.pattern[idx]
            } else {
                0.0
            };

            let y = v * height_scale;

            let color = match params.color_mode {
                AcousticColorMode::Pressure | AcousticColorMode::Energy => {
                    // Map [-1, 1] → [0, 1] for heat map
                    let t = (v * 0.5 + 0.5).clamp(0.0, 1.0);
                    let mut c = visualization_heat_map(t);
                    c.a = params.alpha;
                    c
                }
                AcousticColorMode::SignedPressure => {
                    let mut c = signed_value_color(v);
                    c.a = params.alpha;
                    c
                }
            };
            let c = color.to_array();

            // Compute normals from neighbors
            let get_val = |ix: usize, iz: usize| -> f32 {
                let i = iz * cols + ix;
                if i < mode.pattern.len() {
                    mode.pattern[i] * height_scale
                } else {
                    0.0
                }
            };
            let hl = if ix > 0 { get_val(ix - 1, iz) } else { y };
            let hr = if ix < cols - 1 {
                get_val(ix + 1, iz)
            } else {
                y
            };
            let hd = if iz > 0 { get_val(ix, iz - 1) } else { y };
            let hu = if iz < rows - 1 {
                get_val(ix, iz + 1)
            } else {
                y
            };
            let ndx = (hl - hr) / (2.0 * dx.max(f32::EPSILON));
            let ndz = (hd - hu) / (2.0 * dz.max(f32::EPSILON));
            let len = (ndx * ndx + 1.0 + ndz * ndz).sqrt();

            vertices.push(Vertex3D {
                position: [x_offset + ix as f32 * dx, y, z_offset + iz as f32 * dz],
                normal: [ndx / len, 1.0 / len, ndz / len],
                tex_coords: [
                    ix as f32 / (cols - 1).max(1) as f32,
                    iz as f32 / (rows - 1).max(1) as f32,
                ],
                color: c,
            });
        }
    }

    // Triangle indices
    for iz in 0..(rows.saturating_sub(1)) {
        for ix in 0..(cols.saturating_sub(1)) {
            let tl = (iz * cols + ix) as u32;
            let tr = tl + 1;
            let bl = tl + cols as u32;
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

// ── 4. Portal visualization ────────────────────────────────────────────────

/// Add a portal (opening) as a wireframe rectangle with a normal arrow to a [`LineBatch`].
///
/// The portal is drawn as a rectangle centered at `portal.position` with
/// an arrow along `portal.normal` indicating energy flow direction.
#[cfg(feature = "acoustics")]
pub fn portal_to_lines(
    portal: &goonj::portal::Portal,
    color: Color,
    arrow_length: f32,
    batch: &mut LineBatch,
) {
    let pos: [f32; 3] = portal.position.into();
    let n: [f32; 3] = portal.normal.into();

    // Build a local coordinate frame from the normal
    let (right, up) = normal_to_basis(n);

    let hw = portal.width * 0.5;
    let hh = portal.height * 0.5;

    // Four corners of the portal rectangle
    let corners = [
        [
            pos[0] - right[0] * hw - up[0] * hh,
            pos[1] - right[1] * hw - up[1] * hh,
            pos[2] - right[2] * hw - up[2] * hh,
        ],
        [
            pos[0] + right[0] * hw - up[0] * hh,
            pos[1] + right[1] * hw - up[1] * hh,
            pos[2] + right[2] * hw - up[2] * hh,
        ],
        [
            pos[0] + right[0] * hw + up[0] * hh,
            pos[1] + right[1] * hw + up[1] * hh,
            pos[2] + right[2] * hw + up[2] * hh,
        ],
        [
            pos[0] - right[0] * hw + up[0] * hh,
            pos[1] - right[1] * hw + up[1] * hh,
            pos[2] - right[2] * hw + up[2] * hh,
        ],
    ];

    // Rectangle edges
    batch.line(corners[0], corners[1], color);
    batch.line(corners[1], corners[2], color);
    batch.line(corners[2], corners[3], color);
    batch.line(corners[3], corners[0], color);

    // Normal arrow from center
    let tip = [
        pos[0] + n[0] * arrow_length,
        pos[1] + n[1] * arrow_length,
        pos[2] + n[2] * arrow_length,
    ];
    batch.line(pos, tip, color);

    // Arrowhead (two small lines)
    let head_size = arrow_length * 0.2;
    let back = [
        tip[0] - n[0] * head_size,
        tip[1] - n[1] * head_size,
        tip[2] - n[2] * head_size,
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
}

#[cfg(feature = "acoustics")]
use crate::math_util::normal_to_basis;

// ── 5. Directivity balloons ────────────────────────────────────────────────

/// Generate a 3D mesh for a directivity balloon (deformed sphere).
///
/// The sphere surface is deformed radially by the broadband directivity gain
/// at each vertex direction. `base_radius` controls the minimum radius,
/// `gain_scale` controls how much gain deforms the surface.
/// `front` is the source's main radiation axis.
#[must_use]
#[cfg(feature = "acoustics")]
pub fn directivity_balloon_to_mesh(
    balloon: &goonj::directivity::DirectivityBalloon,
    center: [f32; 3],
    front: [f32; 3],
    base_radius: f32,
    gain_scale: f32,
    lat_segments: u32,
    lon_segments: u32,
) -> (Vec<Vertex3D>, Vec<u32>) {
    use std::f32::consts::PI;

    if lat_segments < 2 || lon_segments < 3 {
        return (Vec::new(), Vec::new());
    }

    let front_vec = hisab::Vec3::new(front[0], front[1], front[2]);
    let front_len = front_vec.length();
    let front_norm = if front_len > f32::EPSILON {
        front_vec / front_len
    } else {
        hisab::Vec3::Z
    };

    let vert_count = (lat_segments + 1) * (lon_segments + 1);
    let mut vertices = Vec::with_capacity(vert_count as usize);
    let mut indices = Vec::with_capacity((lat_segments as usize) * (lon_segments as usize) * 6);

    for lat in 0..=lat_segments {
        let theta = PI * lat as f32 / lat_segments as f32; // 0..π (pole to pole)
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=lon_segments {
            let phi = 2.0 * PI * lon as f32 / lon_segments as f32; // 0..2π
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            // Direction on unit sphere
            let dir = hisab::Vec3::new(sin_theta * cos_phi, cos_theta, sin_theta * sin_phi);

            // Look up broadband gain at this direction
            let gain = balloon_broadband_gain(balloon, dir, front_norm);
            let r = base_radius + gain * gain_scale;

            let pos = [
                center[0] + dir.x * r,
                center[1] + dir.y * r,
                center[2] + dir.z * r,
            ];
            let normal = [dir.x, dir.y, dir.z];

            // Color by gain
            let t = gain.clamp(0.0, 1.0);
            let c = visualization_heat_map(t).to_array();

            vertices.push(Vertex3D {
                position: pos,
                normal,
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

/// Compute broadband gain from a directivity balloon for a given direction.
#[cfg(feature = "acoustics")]
fn balloon_broadband_gain(
    balloon: &goonj::directivity::DirectivityBalloon,
    direction: hisab::Vec3,
    front: hisab::Vec3,
) -> f32 {
    if balloon.azimuths.is_empty() || balloon.elevations.is_empty() {
        return 1.0;
    }

    let cos_theta = direction.dot(front).clamp(-1.0, 1.0);
    let theta = cos_theta.acos();

    // Nearest-neighbor lookup (axisymmetric fallback: az=theta, el=0)
    let az_idx = balloon
        .azimuths
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            ((**a - theta).abs())
                .partial_cmp(&((**b - theta).abs()))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    let el_idx = balloon
        .elevations
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.abs()
                .partial_cmp(&b.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    let flat_idx = el_idx * balloon.azimuths.len() + az_idx;
    let num_bands = balloon.gains.len();
    if num_bands == 0 {
        return 1.0;
    }

    let sum: f32 = balloon
        .gains
        .iter()
        .map(|band| {
            if flat_idx < band.len() {
                band[flat_idx]
            } else {
                1.0
            }
        })
        .sum();
    sum / num_bands as f32
}

// ── 6. Coupled room decay curves ───────────────────────────────────────────

/// Parameters for rendering a coupled decay curve.
pub struct DecayCurveParams {
    /// World-space origin of the curve.
    pub origin: [f32; 3],
    /// Time range in seconds (X axis extent).
    pub time_range: f32,
    /// dB range (negative, e.g. -60.0).
    pub db_range: f32,
    /// Scale factor for the X axis.
    pub x_scale: f32,
    /// Scale factor for the Y axis.
    pub y_scale: f32,
    /// Number of sample steps along the curve.
    pub steps: u32,
    /// Color for the early (fast) decay component.
    pub color_early: Color,
    /// Color for the late (slow) decay component.
    pub color_late: Color,
}

impl Default for DecayCurveParams {
    fn default() -> Self {
        Self {
            origin: [0.0; 3],
            time_range: 3.0,
            db_range: -60.0,
            x_scale: 1.0,
            y_scale: 1.0,
            steps: 100,
            color_early: Color::GREEN,
            color_late: Color::RED,
        }
    }
}

/// Generate a 2D line-strip mesh representing a double-slope energy decay curve.
///
/// The curve shows dB vs time, drawn as a series of connected line segments
/// in the XY plane (X = time, Y = dB).
/// Returns vertices suitable for rendering with `LineBatch`.
#[cfg(feature = "acoustics")]
pub fn coupled_decay_to_lines(
    decay: &goonj::coupled::CoupledDecay,
    params: &DecayCurveParams,
    batch: &mut LineBatch,
) {
    let DecayCurveParams {
        origin,
        time_range,
        db_range,
        x_scale,
        y_scale,
        steps,
        color_early,
        color_late,
    } = *params;
    if steps < 2 || time_range <= 0.0 {
        return;
    }

    let dt = time_range / (steps - 1) as f32;

    // Double-slope decay: E(t) = A_early * exp(-t/τ_early) + A_late * exp(-t/τ_late)
    // where τ = RT60 / (6 * ln(10)) ≈ RT60 / 13.8155
    let decay_const = 6.0 * 10.0_f32.ln();
    let tau_early = if decay.rt60_early > 0.0 {
        decay.rt60_early / decay_const
    } else {
        f32::EPSILON
    };
    let tau_late = if decay.rt60_late > 0.0 {
        decay.rt60_late / decay_const
    } else {
        f32::EPSILON
    };

    let a_early = decay.early_amplitude.clamp(0.0, 1.0);
    let a_late = 1.0 - a_early;

    let mut prev: Option<[f32; 3]> = None;

    for i in 0..steps {
        let t = i as f32 * dt;
        let e_early = a_early * (-t / tau_early).exp();
        let e_late = a_late * (-t / tau_late).exp();
        let e_total = (e_early + e_late).max(1e-10);

        let db = 10.0 * e_total.log10();
        let db_norm = (db / db_range.min(-1.0)).clamp(0.0, 1.0);

        let x = origin[0] + t * x_scale;
        let y = origin[1] + db_norm * y_scale;
        let point = [x, y, origin[2]];

        // Blend color based on which component dominates
        let early_frac = if e_total > 1e-10 {
            e_early / e_total
        } else {
            0.5
        };
        let color = color_early.lerp(color_late, 1.0 - early_frac);

        if let Some(p) = prev {
            batch.line(p, point, color);
        }
        prev = Some(point);
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
    fn signed_pressure_color_zero() {
        let c = signed_value_color(0.0);
        assert_eq!(c.r, 0.0);
        assert_eq!(c.b, 0.0);
    }

    #[test]
    fn signed_pressure_color_positive() {
        let c = signed_value_color(1.0);
        assert_eq!(c.r, 1.0);
        assert_eq!(c.b, 0.0);
    }

    #[test]
    fn signed_pressure_color_negative() {
        let c = signed_value_color(-1.0);
        assert_eq!(c.r, 0.0);
        assert_eq!(c.b, 1.0);
    }

    #[test]
    fn acoustic_heat_map_public() {
        let c = acoustic_heat_map(0.5);
        assert_eq!(c.g, 1.0);
    }

    // ── Normal basis tests ─────────────────────────────────────────────────

    #[test]
    fn normal_to_basis_z_forward() {
        let (right, up) = normal_to_basis([0.0, 0.0, 1.0]);
        // right · normal = 0, up · normal = 0
        let dot_rn = right[0] * 0.0 + right[1] * 0.0 + right[2] * 1.0;
        let dot_un = up[0] * 0.0 + up[1] * 0.0 + up[2] * 1.0;
        assert!(dot_rn.abs() < 0.01);
        assert!(dot_un.abs() < 0.01);
    }

    #[test]
    fn normal_to_basis_y_up() {
        let (right, up) = normal_to_basis([0.0, 1.0, 0.0]);
        let dot_rn = right[0] * 0.0 + right[1] * 1.0 + right[2] * 0.0;
        let dot_un = up[0] * 0.0 + up[1] * 1.0 + up[2] * 0.0;
        assert!(dot_rn.abs() < 0.01);
        assert!(dot_un.abs() < 0.01);
    }

    #[test]
    fn normal_to_basis_orthogonal() {
        let (right, up) = normal_to_basis([0.577, 0.577, 0.577]);
        // right · up = 0
        let dot = right[0] * up[0] + right[1] * up[1] + right[2] * up[2];
        assert!(dot.abs() < 0.01);
    }

    // ── Default params ─────────────────────────────────────────────────────

    #[test]
    fn acoustic_vis_params_default() {
        let p = AcousticVisParams::default();
        assert_eq!(p.color_mode, AcousticColorMode::Pressure);
        assert_eq!(p.alpha, 1.0);
    }

    // ── Feature-gated tests ────────────────────────────────────────────────

    #[cfg(feature = "acoustics")]
    mod acoustics_tests {
        use super::*;
        use crate::debug_draw::LineBatch;

        #[test]
        fn ray_paths_empty() {
            let viz = goonj::integration::soorat::RayVisualization {
                source: hisab::Vec3::ZERO,
                paths: vec![],
            };
            let mut batch = LineBatch::new();
            ray_paths_to_lines(&viz, Color::GREEN, Color::RED, &mut batch);
            assert!(batch.is_empty());
        }

        #[test]
        fn ray_paths_single_bounce() {
            let viz = goonj::integration::soorat::RayVisualization {
                source: hisab::Vec3::ZERO,
                paths: vec![goonj::ray::RayPath {
                    bounces: vec![goonj::ray::RayBounce {
                        point: hisab::Vec3::new(1.0, 0.0, 0.0),
                        normal: hisab::Vec3::Y,
                        wall_index: 0,
                        distance_from_previous: 1.0,
                        energy_after: [0.9; 8],
                    }],
                    total_distance: 1.0,
                    final_energy: [0.9; 8],
                }],
            };
            let mut batch = LineBatch::new();
            ray_paths_to_lines(&viz, Color::GREEN, Color::RED, &mut batch);
            assert_eq!(batch.line_count(), 1);
        }

        #[test]
        fn ray_paths_multi_bounce() {
            let viz = goonj::integration::soorat::RayVisualization {
                source: hisab::Vec3::ZERO,
                paths: vec![goonj::ray::RayPath {
                    bounces: vec![
                        goonj::ray::RayBounce {
                            point: hisab::Vec3::new(1.0, 0.0, 0.0),
                            normal: hisab::Vec3::Y,
                            wall_index: 0,
                            distance_from_previous: 1.0,
                            energy_after: [0.8; 8],
                        },
                        goonj::ray::RayBounce {
                            point: hisab::Vec3::new(1.0, 0.0, 1.0),
                            normal: hisab::Vec3::Y,
                            wall_index: 1,
                            distance_from_previous: 1.0,
                            energy_after: [0.6; 8],
                        },
                        goonj::ray::RayBounce {
                            point: hisab::Vec3::new(0.0, 0.0, 1.0),
                            normal: hisab::Vec3::Y,
                            wall_index: 2,
                            distance_from_previous: 1.0,
                            energy_after: [0.4; 8],
                        },
                    ],
                    total_distance: 3.0,
                    final_energy: [0.4; 8],
                }],
            };
            let mut batch = LineBatch::new();
            ray_paths_to_lines(&viz, Color::GREEN, Color::RED, &mut batch);
            assert_eq!(batch.line_count(), 3);
        }

        #[test]
        fn pressure_map_slice_empty() {
            let map = goonj::integration::soorat::PressureMap {
                values: vec![],
                dimensions: [0, 0, 0],
                origin: hisab::Vec3::ZERO,
                spacing: 1.0,
                frequency_hz: 1000.0,
            };
            let (v, i) = pressure_map_slice(&map, 0, &AcousticVisParams::default());
            assert!(v.is_empty());
            assert!(i.is_empty());
        }

        #[test]
        fn pressure_map_slice_2x2() {
            let map = goonj::integration::soorat::PressureMap {
                values: vec![0.0, 0.5, 0.3, 1.0, 0.1, 0.8, 0.6, 0.2],
                dimensions: [2, 2, 2],
                origin: hisab::Vec3::ZERO,
                spacing: 1.0,
                frequency_hz: 1000.0,
            };
            let (v, i) = pressure_map_slice(&map, 0, &AcousticVisParams::default());
            assert_eq!(v.len(), 2 * 2 * 4); // 4 cells × 4 verts
            assert_eq!(i.len(), 2 * 2 * 6);
        }

        #[test]
        fn pressure_map_out_of_bounds_y() {
            let map = goonj::integration::soorat::PressureMap {
                values: vec![1.0; 8],
                dimensions: [2, 2, 2],
                origin: hisab::Vec3::ZERO,
                spacing: 1.0,
                frequency_hz: 500.0,
            };
            let (v, _) = pressure_map_slice(&map, 5, &AcousticVisParams::default());
            assert!(v.is_empty());
        }

        #[test]
        fn mode_pattern_empty() {
            let mode = goonj::integration::soorat::ModeVisualization {
                frequency_hz: 100.0,
                mode_indices: [1, 0, 0],
                pattern: vec![],
                pattern_dimensions: [0, 0],
            };
            let (v, i) = mode_pattern_to_mesh(&mode, 10.0, 8.0, 1.0, &AcousticVisParams::default());
            assert!(v.is_empty());
            assert!(i.is_empty());
        }

        #[test]
        fn mode_pattern_3x3() {
            let mode = goonj::integration::soorat::ModeVisualization::for_shoebox(
                1, 0, 10.0, 8.0, 343.0, 3,
            );
            let (v, i) = mode_pattern_to_mesh(&mode, 10.0, 8.0, 1.0, &AcousticVisParams::default());
            assert_eq!(v.len(), 9); // 3×3
            assert_eq!(i.len(), 2 * 2 * 6); // (3-1)×(3-1) quads × 6
        }

        #[test]
        fn mode_pattern_has_height() {
            let mode = goonj::integration::soorat::ModeVisualization::for_shoebox(
                1, 0, 10.0, 8.0, 343.0, 5,
            );
            let (v, _) = mode_pattern_to_mesh(&mode, 10.0, 8.0, 2.0, &AcousticVisParams::default());
            // Not all vertices at y=0 (mode pattern produces non-zero values)
            let has_height = v.iter().any(|vert| vert.position[1].abs() > 0.01);
            assert!(has_height);
        }

        #[test]
        fn portal_to_lines_basic() {
            let portal = goonj::portal::Portal {
                position: hisab::Vec3::ZERO,
                normal: hisab::Vec3::Z,
                width: 2.0,
                height: 3.0,
            };
            let mut batch = LineBatch::new();
            portal_to_lines(&portal, Color::WHITE, 1.0, &mut batch);
            // 4 rectangle edges + 1 normal arrow + 2 arrowhead = 7 lines
            assert_eq!(batch.line_count(), 7);
        }

        #[test]
        fn portal_different_normal() {
            let portal = goonj::portal::Portal {
                position: hisab::Vec3::new(5.0, 0.0, 0.0),
                normal: hisab::Vec3::X,
                width: 1.0,
                height: 2.0,
            };
            let mut batch = LineBatch::new();
            portal_to_lines(&portal, Color::RED, 0.5, &mut batch);
            assert_eq!(batch.line_count(), 7);
        }

        #[test]
        fn directivity_balloon_empty() {
            let balloon = goonj::directivity::DirectivityBalloon {
                azimuths: vec![],
                elevations: vec![],
                gains: std::array::from_fn(|_| vec![]),
            };
            let (v, i) =
                directivity_balloon_to_mesh(&balloon, [0.0; 3], [0.0, 0.0, 1.0], 1.0, 1.0, 8, 8);
            // Still generates sphere geometry (empty balloon → gain 1.0)
            assert!(!v.is_empty());
            assert!(!i.is_empty());
        }

        #[test]
        fn directivity_balloon_min_segments() {
            let balloon = goonj::directivity::DirectivityBalloon {
                azimuths: vec![0.0],
                elevations: vec![0.0],
                gains: std::array::from_fn(|_| vec![0.5]),
            };
            // Too few segments
            let (v, i) =
                directivity_balloon_to_mesh(&balloon, [0.0; 3], [0.0, 0.0, 1.0], 1.0, 1.0, 1, 2);
            assert!(v.is_empty());
            assert!(i.is_empty());
        }

        #[test]
        fn directivity_balloon_sphere() {
            let balloon = goonj::directivity::DirectivityBalloon {
                azimuths: vec![0.0, std::f32::consts::FRAC_PI_2, std::f32::consts::PI],
                elevations: vec![0.0],
                gains: std::array::from_fn(|_| vec![1.0, 0.5, 0.2]),
            };
            let (v, i) =
                directivity_balloon_to_mesh(&balloon, [0.0; 3], [0.0, 0.0, 1.0], 0.5, 2.0, 4, 6);
            // (4+1) * (6+1) = 35 vertices
            assert_eq!(v.len(), 35);
            // 4 * 6 * 6 = 144 indices
            assert_eq!(i.len(), 144);
        }

        #[test]
        fn coupled_decay_basic() {
            let decay = goonj::coupled::CoupledDecay {
                rt60_early: 0.5,
                rt60_late: 2.0,
                early_amplitude: 0.7,
                coupling_strength: 0.3,
            };
            let mut batch = LineBatch::new();
            coupled_decay_to_lines(&decay, &DecayCurveParams::default(), &mut batch);
            // 100 steps → 99 line segments
            assert_eq!(batch.line_count(), 99);
        }

        #[test]
        fn coupled_decay_zero_steps() {
            let decay = goonj::coupled::CoupledDecay {
                rt60_early: 1.0,
                rt60_late: 2.0,
                early_amplitude: 0.5,
                coupling_strength: 0.5,
            };
            let mut batch = LineBatch::new();
            let params = DecayCurveParams {
                steps: 0,
                ..DecayCurveParams::default()
            };
            coupled_decay_to_lines(&decay, &params, &mut batch);
            assert!(batch.is_empty());
        }

        #[test]
        fn coupled_decay_monotonic() {
            let decay = goonj::coupled::CoupledDecay {
                rt60_early: 0.5,
                rt60_late: 2.0,
                early_amplitude: 0.7,
                coupling_strength: 0.3,
            };
            let mut batch = LineBatch::new();
            let params = DecayCurveParams {
                steps: 50,
                ..DecayCurveParams::default()
            };
            coupled_decay_to_lines(&decay, &params, &mut batch);
            // Y should be monotonically non-increasing (decay curve goes down)
            // Each line has 2 vertices, consecutive lines share endpoint
            for i in (0..batch.vertices.len()).step_by(2) {
                if i + 1 < batch.vertices.len() {
                    let y0 = batch.vertices[i].position[1];
                    let y1 = batch.vertices[i + 1].position[1];
                    assert!(
                        y1 >= y0 - 0.001,
                        "decay should be monotonic: y0={y0}, y1={y1} at segment {i}"
                    );
                }
            }
        }

        #[test]
        fn pressure_map_single_cell() {
            // Adversarial: 1x1x1 grid must work correctly
            let map = goonj::integration::soorat::PressureMap {
                values: vec![0.5],
                dimensions: [1, 1, 1],
                origin: hisab::Vec3::ZERO,
                spacing: 1.0,
                frequency_hz: 1000.0,
            };
            let (v, i) = pressure_map_slice(&map, 0, &AcousticVisParams::default());
            assert_eq!(v.len(), 4); // 1 cell × 4 verts
            assert_eq!(i.len(), 6); // 1 cell × 6 indices
            // No NaN in vertex data
            for vert in &v {
                for &p in &vert.position {
                    assert!(!p.is_nan(), "vertex position contains NaN");
                }
            }
        }
    }
}
