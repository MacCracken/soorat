//! Shared CPU helpers for the scientific-visualization modules
//! ([`crate::acoustic_render`], [`crate::em_render`], [`crate::thermo_render`],
//! [`crate::fluid_render`]).
//!
//! Centralises the pieces every heatmap needs so they behave identically:
//! - non-finite (NaN/±inf) samples never produce NaN colors or panics,
//! - grid sizes are overflow-checked and bounded by the input data,
//! - quads are wound counter-clockwise around their +Y normal (the engine's
//!   front-face convention), so back-face culling keeps them visible from above.

// Each viz feature uses a different subset of these helpers; unused-code
// detection is only meaningful when all of them are enabled (e.g. `full`).
#![cfg_attr(
    not(all(
        feature = "acoustics",
        feature = "em",
        feature = "thermo",
        feature = "fluids"
    )),
    allow(dead_code)
)]

use crate::color::{Color, signed_value_color, visualization_heat_map};
use crate::vertex::Vertex3D;

/// Maximum number of quads in one generated mesh (4 vertices per quad must stay
/// addressable by `u32` indices).
pub(crate) const MAX_QUADS: usize = (u32::MAX / 4) as usize;

/// Upper bound for user-supplied tessellation segment counts (sphere/balloon/
/// circle resolution). 1024² ≈ 1M vertices (~50 MB) is already far beyond any
/// useful display, and keeps a bogus `u32::MAX` request bounded.
pub(crate) const MAX_SEGMENTS: u32 = 1024;

/// Index pattern for a quad whose corners are pushed as
/// `(x0,z0), (x1,z0), (x1,z1), (x0,z1)` on the XZ plane: two triangles that are
/// counter-clockwise when viewed from +Y.
pub(crate) const XZ_QUAD_INDICES: [u32; 6] = [0, 3, 2, 0, 2, 1];

/// Checked element count of a grid (`product(dims)`), `None` on overflow.
#[must_use]
#[inline]
pub(crate) fn grid_len(dims: &[usize]) -> Option<usize> {
    dims.iter().try_fold(1usize, |acc, &d| acc.checked_mul(d))
}

/// Validate that a grid with `dims` is non-empty, does not overflow, and is
/// fully backed by `data_len` samples. Logs and returns `false` otherwise.
///
/// Bounding the grid by the data it came with keeps output size proportional
/// to input size (a malformed header cannot request billions of cells).
#[must_use]
pub(crate) fn grid_is_backed(dims: &[usize], data_len: usize, what: &str) -> bool {
    match grid_len(dims) {
        Some(0) => false,
        Some(n) if n <= data_len => true,
        Some(n) => {
            tracing::warn!(
                what,
                ?dims,
                expected = n,
                actual = data_len,
                "visualization grid larger than its data — skipping"
            );
            false
        }
        None => {
            tracing::warn!(what, ?dims, "visualization grid size overflows — skipping");
            false
        }
    }
}

/// Finite minimum and maximum of a sequence, ignoring NaN and ±inf.
/// Returns `None` if the sequence has no finite values.
#[must_use]
#[inline]
pub(crate) fn finite_min_max<I: IntoIterator<Item = f64>>(values: I) -> Option<(f64, f64)> {
    let mut range: Option<(f64, f64)> = None;
    for v in values {
        if !v.is_finite() {
            continue;
        }
        range = Some(match range {
            None => (v, v),
            Some((lo, hi)) => (lo.min(v), hi.max(v)),
        });
    }
    range
}

/// Map `v` into `[0, 1]` over `[min, max]`. Non-finite input or a degenerate
/// range maps to `0`.
#[must_use]
#[inline]
pub(crate) fn unit_t(v: f64, min: f64, max: f64) -> f32 {
    let range = max - min;
    if !v.is_finite() || range.is_nan() || range <= f64::EPSILON {
        return 0.0;
    }
    (((v - min) / range) as f32).clamp(0.0, 1.0)
}

/// Largest absolute value of a `[min, max]` range (the symmetric scale used by
/// signed and magnitude colouring).
#[must_use]
#[inline]
fn abs_scale(min: f64, max: f64) -> f64 {
    min.abs().max(max.abs())
}

/// Map signed `v` into `[-1, 1]` symmetric around **zero** (zero stays black
/// in a diverging map regardless of how asymmetric the data range is).
#[must_use]
#[inline]
pub(crate) fn signed_t(v: f64, min: f64, max: f64) -> f32 {
    let scale = abs_scale(min, max);
    if !v.is_finite() || scale.is_nan() || scale <= f64::EPSILON {
        return 0.0;
    }
    ((v / scale) as f32).clamp(-1.0, 1.0)
}

/// Map `|v|` into `[0, 1]` relative to the largest magnitude in the range.
#[must_use]
#[inline]
pub(crate) fn magnitude_t(v: f64, min: f64, max: f64) -> f32 {
    let scale = abs_scale(min, max);
    if !v.is_finite() || scale.is_nan() || scale <= f64::EPSILON {
        return 0.0;
    }
    ((v.abs() / scale) as f32).clamp(0.0, 1.0)
}

/// Heat-map color with an explicit alpha, as a vertex color array.
#[must_use]
#[inline]
pub(crate) fn heat_rgba(t: f32, alpha: f32) -> [f32; 4] {
    let mut c = visualization_heat_map(t);
    c.a = alpha;
    c.to_array()
}

/// Diverging (blue/black/red) color with an explicit alpha.
#[must_use]
#[inline]
pub(crate) fn signed_rgba(t: f32, alpha: f32) -> [f32; 4] {
    let mut c = signed_value_color(t);
    c.a = alpha;
    c.to_array()
}

/// `base` with its alpha replaced; non-finite alpha becomes 0.
#[must_use]
#[inline]
pub(crate) fn with_alpha(base: Color, alpha: f32) -> [f32; 4] {
    let a = if alpha.is_finite() {
        alpha.clamp(0.0, 1.0)
    } else {
        0.0
    };
    [base.r, base.g, base.b, a]
}

/// Placement of a cell-per-quad heatmap on the XZ plane.
#[derive(Debug, Clone, Copy)]
pub(crate) struct XzGrid {
    /// Cells along X.
    pub nx: usize,
    /// Cells along Z.
    pub nz: usize,
    /// World X of the grid's min corner.
    pub origin_x: f32,
    /// World Z of the grid's min corner.
    pub origin_z: f32,
    /// World Y of the plane.
    pub y: f32,
    /// Cell size along X.
    pub dx: f32,
    /// Cell size along Z.
    pub dz: f32,
}

/// Build a heatmap mesh: one +Y-facing quad per cell, colored by
/// `color_at(ix, iz)`. Returns an empty mesh if the cell count is zero or
/// exceeds [`MAX_QUADS`].
#[must_use]
pub(crate) fn xz_heatmap_mesh(
    grid: &XzGrid,
    mut color_at: impl FnMut(usize, usize) -> [f32; 4],
) -> (Vec<Vertex3D>, Vec<u32>) {
    let Some(cells) = grid.nx.checked_mul(grid.nz).filter(|&c| c > 0) else {
        return (Vec::new(), Vec::new());
    };
    if cells > MAX_QUADS {
        tracing::warn!(cells, "heatmap exceeds u32 index range — skipping");
        return (Vec::new(), Vec::new());
    }

    let mut vertices = Vec::with_capacity(cells * 4);
    let mut indices = Vec::with_capacity(cells * 6);
    let inv_nx = 1.0 / grid.nx as f32;
    let inv_nz = 1.0 / grid.nz as f32;
    let up = [0.0, 1.0, 0.0];

    for iz in 0..grid.nz {
        let z0 = grid.origin_z + iz as f32 * grid.dz;
        let z1 = z0 + grid.dz;
        let v0 = iz as f32 * inv_nz;
        let v1 = (iz + 1) as f32 * inv_nz;
        for ix in 0..grid.nx {
            let color = color_at(ix, iz);
            let x0 = grid.origin_x + ix as f32 * grid.dx;
            let x1 = x0 + grid.dx;
            let u0 = ix as f32 * inv_nx;
            let u1 = (ix + 1) as f32 * inv_nx;

            // cells <= MAX_QUADS, so base + 3 always fits in u32.
            let base = vertices.len() as u32;
            for (position, tex_coords) in [
                ([x0, grid.y, z0], [u0, v0]),
                ([x1, grid.y, z0], [u1, v0]),
                ([x1, grid.y, z1], [u1, v1]),
                ([x0, grid.y, z1], [u0, v1]),
            ] {
                vertices.push(Vertex3D {
                    position,
                    normal: up,
                    tex_coords,
                    color,
                });
            }
            indices.extend(XZ_QUAD_INDICES.iter().map(|&i| base + i));
        }
    }

    (vertices, indices)
}

/// Placement of a vertex-per-sample height field on the XZ plane.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HeightField {
    /// Samples along X.
    pub cols: usize,
    /// Samples along Z.
    pub rows: usize,
    /// World X of sample (0, 0).
    pub origin_x: f32,
    /// World Z of sample (0, 0).
    pub origin_z: f32,
    /// Sample spacing along X.
    pub dx: f32,
    /// Sample spacing along Z.
    pub dz: f32,
}

/// Negative slope `(h[i-1] - h[i+1]) / span` along one axis of a height grid,
/// where sample `k` of that axis lives at `h[k * stride]` and the axis has `n`
/// samples. Central difference inside, one-sided at the borders (so edge
/// slopes are not halved); a single sample or zero spacing gives slope 0.
#[must_use]
#[inline]
fn neg_slope(h: &[f32], i: usize, stride: usize, n: usize, step: f32) -> f32 {
    let lo = i.saturating_sub(1);
    let hi = (i + 1).min(n - 1);
    let span = (hi - lo) as f32 * step;
    if span.is_nan() || span.abs() <= f32::EPSILON {
        return 0.0;
    }
    (h[lo * stride] - h[hi * stride]) / span
}

/// Build a lit height-field mesh: one vertex per sample at `y = height(ix, iz)`
/// (non-finite heights become 0), central-difference normals (one-sided at the
/// borders), and triangles counter-clockwise around +Y.
///
/// `color(ix, iz, y)` supplies each vertex color. Returns an empty mesh if the
/// grid is empty or its vertex count does not fit `u32` indices.
#[must_use]
pub(crate) fn heightfield_mesh(
    hf: &HeightField,
    height: impl Fn(usize, usize) -> f32,
    mut color: impl FnMut(usize, usize, f32) -> [f32; 4],
) -> (Vec<Vertex3D>, Vec<u32>) {
    let (cols, rows) = (hf.cols, hf.rows);
    let Some(count) = cols.checked_mul(rows).filter(|&c| c > 0) else {
        return (Vec::new(), Vec::new());
    };
    if count > u32::MAX as usize {
        tracing::warn!(
            cols,
            rows,
            "height field exceeds u32 index range — skipping"
        );
        return (Vec::new(), Vec::new());
    }

    let mut heights = Vec::with_capacity(count);
    for iz in 0..rows {
        for ix in 0..cols {
            let h = height(ix, iz);
            heights.push(if h.is_finite() { h } else { 0.0 });
        }
    }

    let inv_u = 1.0 / (cols - 1).max(1) as f32;
    let inv_v = 1.0 / (rows - 1).max(1) as f32;
    let mut vertices = Vec::with_capacity(count);
    for iz in 0..rows {
        let row = &heights[iz * cols..(iz + 1) * cols];
        for ix in 0..cols {
            let y = row[ix];
            // Row-relative slices keep both differences in-bounds by construction.
            let sx = neg_slope(row, ix, 1, cols, hf.dx);
            let column = &heights[ix..];
            let sz = neg_slope(column, iz, cols, rows, hf.dz);
            let len = (sx * sx + 1.0 + sz * sz).sqrt();
            vertices.push(Vertex3D {
                position: [
                    hf.origin_x + ix as f32 * hf.dx,
                    y,
                    hf.origin_z + iz as f32 * hf.dz,
                ],
                normal: [sx / len, 1.0 / len, sz / len],
                tex_coords: [ix as f32 * inv_u, iz as f32 * inv_v],
                color: color(ix, iz, y),
            });
        }
    }

    let quads = (cols - 1) * rows.saturating_sub(1);
    let mut indices = Vec::with_capacity(quads * 6);
    // count <= u32::MAX, so every vertex index fits.
    let stride = cols as u32;
    for iz in 0..rows.saturating_sub(1) as u32 {
        for ix in 0..(cols - 1) as u32 {
            let tl = iz * stride + ix;
            let tr = tl + 1;
            let bl = tl + stride;
            let br = bl + 1;
            indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }

    (vertices, indices)
}

/// Triangle-list indices for a `(rows+1) × (cols+1)` vertex lattice laid out
/// row-major, wound so that a lattice parameterised like
/// [`crate::primitives::sphere`] faces outward.
#[must_use]
pub(crate) fn lattice_indices(rows: u32, cols: u32) -> Vec<u32> {
    let stride = cols + 1;
    let mut indices = Vec::with_capacity(rows as usize * cols as usize * 6);
    for r in 0..rows {
        for c in 0..cols {
            let tl = r * stride + c;
            let tr = tl + 1;
            let bl = tl + stride;
            let br = bl + 1;
            indices.extend_from_slice(&[tl, tr, bl, tr, br, bl]);
        }
    }
    indices
}

/// Test helper: geometric (cross-product) normal of a triangle.
#[cfg(test)]
pub(crate) fn tri_normal(v: &[Vertex3D], i: &[u32]) -> [f32; 3] {
    let a = v[i[0] as usize].position;
    let b = v[i[1] as usize].position;
    let c = v[i[2] as usize].position;
    crate::math_util::cross(
        [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
        [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
    )
}

/// Test helper: every non-degenerate triangle must be counter-clockwise around
/// its vertices' normals (geometric normal · vertex normal > 0), i.e. front-
/// facing under the engine's CCW + back-face-culling convention.
#[cfg(test)]
pub(crate) fn assert_winding_matches_normals(vertices: &[Vertex3D], indices: &[u32]) {
    assert_eq!(indices.len() % 3, 0, "index count not a multiple of 3");
    for tri in indices.chunks(3) {
        let g = tri_normal(vertices, tri);
        let area2 = g[0] * g[0] + g[1] * g[1] + g[2] * g[2];
        if area2 < 1e-12 {
            continue; // degenerate (e.g. sphere pole)
        }
        let mut n = [0.0f32; 3];
        for &i in tri {
            let vn = vertices[i as usize].normal;
            n = [n[0] + vn[0], n[1] + vn[1], n[2] + vn[2]];
        }
        let d = g[0] * n[0] + g[1] * n[1] + g[2] * n[2];
        assert!(
            d > 0.0,
            "triangle {tri:?} winding {g:?} disagrees with vertex normals {n:?}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(nx: usize, nz: usize) -> XzGrid {
        XzGrid {
            nx,
            nz,
            origin_x: -1.0,
            origin_z: 2.0,
            y: 0.5,
            dx: 0.5,
            dz: 0.25,
        }
    }

    #[test]
    fn xz_heatmap_faces_plus_y() {
        let (v, i) = xz_heatmap_mesh(&grid(3, 2), |_, _| [1.0; 4]);
        assert_eq!(v.len(), 3 * 2 * 4);
        assert_eq!(i.len(), 3 * 2 * 6);
        assert_winding_matches_normals(&v, &i);
        for tri in i.chunks(3) {
            let n = tri_normal(&v, tri);
            assert!(n[1] > 0.0, "triangle {tri:?} faces {n:?}, expected +Y");
            assert!(n[0].abs() < 1e-6 && n[2].abs() < 1e-6);
        }
    }

    #[test]
    fn xz_heatmap_positions_and_colors() {
        let (v, _) = xz_heatmap_mesh(&grid(2, 1), |ix, iz| [ix as f32, iz as f32, 0.0, 1.0]);
        assert_eq!(v[0].position, [-1.0, 0.5, 2.0]);
        assert_eq!(v[2].position, [-0.5, 0.5, 2.25]);
        assert_eq!(v[4].color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(v[7].tex_coords, [0.5, 1.0]);
    }

    #[test]
    fn xz_heatmap_empty_grid() {
        let (v, i) = xz_heatmap_mesh(&grid(0, 5), |_, _| [1.0; 4]);
        assert!(v.is_empty() && i.is_empty());
        let (v, i) = xz_heatmap_mesh(&grid(usize::MAX, 2), |_, _| [1.0; 4]);
        assert!(v.is_empty() && i.is_empty());
    }

    #[test]
    fn grid_backing_checks() {
        assert!(grid_is_backed(&[2, 3], 6, "t"));
        assert!(grid_is_backed(&[2, 3], 10, "t"));
        assert!(!grid_is_backed(&[2, 3], 5, "t"));
        assert!(!grid_is_backed(&[0, 3], 10, "t"));
        assert!(!grid_is_backed(&[usize::MAX, 2], 10, "t"));
        assert!(!grid_is_backed(&[100_000, 1, 100_000], 0, "t"));
    }

    #[test]
    fn finite_min_max_ignores_non_finite() {
        assert_eq!(
            finite_min_max([f64::NAN, 2.0, f64::INFINITY, -1.0, f64::NEG_INFINITY]),
            Some((-1.0, 2.0))
        );
        assert_eq!(finite_min_max([f64::NAN]), None);
        assert_eq!(finite_min_max(std::iter::empty()), None);
    }

    #[test]
    fn unit_t_edges() {
        assert_eq!(unit_t(5.0, 0.0, 10.0), 0.5);
        assert_eq!(unit_t(f64::NAN, 0.0, 10.0), 0.0);
        assert_eq!(unit_t(3.0, 3.0, 3.0), 0.0);
        assert_eq!(unit_t(20.0, 0.0, 10.0), 1.0);
    }

    #[test]
    fn signed_t_is_zero_centred() {
        // Asymmetric data: zero must stay at zero, not at the range midpoint.
        assert_eq!(signed_t(0.0, 0.0, 1.0), 0.0);
        assert_eq!(signed_t(1.0, 0.0, 1.0), 1.0);
        assert_eq!(signed_t(-2.0, -2.0, 1.0), -1.0);
        assert_eq!(signed_t(1.0, -2.0, 1.0), 0.5);
        assert_eq!(signed_t(f64::NAN, -1.0, 1.0), 0.0);
        assert_eq!(signed_t(1.0, 0.0, 0.0), 0.0);
    }

    #[test]
    fn magnitude_t_uses_absolute_value() {
        assert_eq!(magnitude_t(-1.0, -1.0, 1.0), 1.0);
        assert_eq!(magnitude_t(1.0, -1.0, 1.0), 1.0);
        assert_eq!(magnitude_t(0.0, -1.0, 1.0), 0.0);
        assert_eq!(magnitude_t(f64::INFINITY, -1.0, 1.0), 0.0);
    }

    #[test]
    fn with_alpha_clamps() {
        assert_eq!(with_alpha(Color::RED, 2.0)[3], 1.0);
        assert_eq!(with_alpha(Color::RED, -0.5)[3], 0.0);
        assert_eq!(with_alpha(Color::RED, f32::NAN)[3], 0.0);
    }

    fn hf(cols: usize, rows: usize) -> HeightField {
        HeightField {
            cols,
            rows,
            origin_x: 0.0,
            origin_z: 0.0,
            dx: 1.0,
            dz: 1.0,
        }
    }

    #[test]
    fn heightfield_flat_and_upward() {
        let (v, i) = heightfield_mesh(&hf(4, 3), |_, _| 2.0, |_, _, _| [1.0; 4]);
        assert_eq!(v.len(), 12);
        assert_eq!(i.len(), 3 * 2 * 6);
        for vert in &v {
            assert_eq!(vert.normal, [0.0, 1.0, 0.0]);
            assert_eq!(vert.position[1], 2.0);
        }
        assert_winding_matches_normals(&v, &i);
    }

    #[test]
    fn heightfield_edge_slope_not_halved() {
        // Linear ramp h = x along X: every vertex (edges included) must have
        // the same normal, normalize(-1, 1, 0).
        let (v, _) = heightfield_mesh(&hf(4, 2), |ix, _| ix as f32, |_, _, _| [1.0; 4]);
        let expected = -std::f32::consts::FRAC_1_SQRT_2;
        for vert in &v {
            assert!(
                (vert.normal[0] - expected).abs() < 1e-6,
                "{:?}",
                vert.normal
            );
            assert!((vert.normal[1] + expected).abs() < 1e-6);
        }
    }

    #[test]
    fn heightfield_sanitizes_non_finite() {
        let (v, i) = heightfield_mesh(
            &hf(3, 3),
            |ix, iz| {
                if ix == 1 && iz == 1 {
                    f32::NAN
                } else {
                    f32::INFINITY
                }
            },
            |_, _, _| [1.0; 4],
        );
        assert_eq!(v.len(), 9);
        assert_eq!(i.len(), 2 * 2 * 6);
        for vert in &v {
            assert!(vert.position.iter().all(|p| p.is_finite()));
            assert!(vert.normal.iter().all(|n| n.is_finite()));
        }
    }

    #[test]
    fn heightfield_degenerate_sizes() {
        let (v, i) = heightfield_mesh(&hf(0, 3), |_, _| 0.0, |_, _, _| [1.0; 4]);
        assert!(v.is_empty() && i.is_empty());
        // A single row has vertices but no triangles.
        let (v, i) = heightfield_mesh(&hf(3, 1), |_, _| 0.0, |_, _, _| [1.0; 4]);
        assert_eq!(v.len(), 3);
        assert!(i.is_empty());
        // Zero spacing: normals stay finite.
        let flat = HeightField {
            dx: 0.0,
            dz: 0.0,
            ..hf(3, 3)
        };
        let (v, _) = heightfield_mesh(&flat, |ix, _| ix as f32, |_, _, _| [1.0; 4]);
        assert!(v.iter().all(|vert| vert.normal == [0.0, 1.0, 0.0]));
    }

    #[test]
    fn lattice_matches_sphere_winding() {
        // Same parameterisation as primitives::sphere: lattice indices must
        // produce outward-facing triangles.
        let (verts, _) = crate::primitives::sphere(8, 6);
        assert_winding_matches_normals(&verts, &lattice_indices(6, 8));
    }
}
