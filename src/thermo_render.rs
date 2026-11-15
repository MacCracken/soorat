//! Thermodynamics visualization — thermal grid heatmaps, temperature profiles,
//! cycle diagrams, thermal network graphs, and heat flux arrows.
//!
//! Requires feature: `thermo` (dep: ushma).

use crate::color::Color;
#[cfg(feature = "thermo")]
use crate::debug_draw::LineBatch;
#[cfg(feature = "thermo")]
use crate::vertex::Vertex3D;

/// Color mode for thermal visualization.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalColorMode {
    /// Blue (cold) → cyan → green → yellow → red (hot) gradient.
    Temperature,
    /// Single color with alpha mapped to temperature.
    AlphaBlend,
}

/// Parameters for thermal visualization.
pub struct ThermalVisParams {
    /// Color mapping mode.
    pub color_mode: ThermalColorMode,
    /// Base color for alpha-blend mode.
    pub base_color: Color,
    /// Alpha for generated geometry.
    pub alpha: f32,
}

impl Default for ThermalVisParams {
    fn default() -> Self {
        Self {
            color_mode: ThermalColorMode::Temperature,
            base_color: Color::RED,
            alpha: 1.0,
        }
    }
}

/// Parameters for cycle diagram rendering.
pub struct CycleDiagramParams {
    /// World-space origin of the diagram.
    pub origin: [f32; 3],
    /// Scale factor for the X axis.
    pub x_scale: f32,
    /// Scale factor for the Y axis.
    pub y_scale: f32,
    /// Color for the T-s diagram.
    pub ts_color: Color,
    /// Color for the P-v diagram.
    pub pv_color: Color,
}

impl Default for CycleDiagramParams {
    fn default() -> Self {
        Self {
            origin: [0.0; 3],
            x_scale: 1.0,
            y_scale: 1.0,
            ts_color: Color::RED,
            pv_color: Color::new(0.2, 0.5, 1.0, 1.0),
        }
    }
}

// ── CPU-only helpers (no feature gate) ──────────────────────────────────────

use crate::color::visualization_heat_map;
#[cfg(feature = "thermo")]
use crate::viz::{
    XzGrid, finite_min_max, grid_is_backed, heat_rgba, unit_t, with_alpha, xz_heatmap_mesh,
};

/// Thermal heat map exposed for general use.
#[must_use]
pub fn thermal_heat_map(t: f32) -> Color {
    visualization_heat_map(t)
}

// ── 1. Thermal grid heatmap ───────────────────────────────────────────────

/// Generate a mesh for a 2D thermal grid as a colored heatmap.
///
/// Each grid cell becomes a colored, +Y-facing quad on the XZ plane at `y_pos`.
/// Temperatures are normalized to \[0,1\] over the grid's `min_temp..max_temp`
/// (values outside that range are clamped; non-finite values map to 0).
///
/// Returns an empty mesh if `values` holds fewer than `nx * ny` samples.
#[must_use]
#[cfg(feature = "thermo")]
pub fn thermal_grid_to_mesh(
    grid: &ushma::integration::soorat::ThermalGridVisualization,
    y_pos: f32,
    params: &ThermalVisParams,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let [nx, ny] = grid.dimensions;
    if !grid_is_backed(&grid.dimensions, grid.values.len(), "thermal_grid_to_mesh") {
        return (Vec::new(), Vec::new());
    }
    tracing::debug!(nx, ny, "building thermal grid heatmap");

    let cells = &grid.values[..nx * ny];
    let plane = XzGrid {
        nx,
        nz: ny,
        origin_x: grid.origin[0] as f32,
        origin_z: grid.origin[1] as f32,
        y: y_pos,
        dx: grid.spacing[0] as f32,
        dz: grid.spacing[1] as f32,
    };
    xz_heatmap_mesh(&plane, |ix, iy| {
        let t = unit_t(cells[iy * nx + ix], grid.min_temp, grid.max_temp);
        match params.color_mode {
            ThermalColorMode::Temperature => heat_rgba(t, params.alpha),
            ThermalColorMode::AlphaBlend => with_alpha(params.base_color, t * params.alpha),
        }
    })
}

// ── 2. Temperature profile ────────────────────────────────────────────────

/// Draw a 1D temperature profile as a colored line strip in 3D space.
///
/// The profile is laid out along `direction` starting from `origin`.
/// Color is heat-mapped from normalized temperature.
#[cfg(feature = "thermo")]
pub fn temperature_profile_to_lines(
    profile: &ushma::integration::soorat::TemperatureProfile,
    batch: &mut LineBatch,
) {
    if profile.temperatures.len() < 2 {
        return;
    }

    let range = (profile.max_temp - profile.min_temp).max(f64::EPSILON);
    let dir = profile.direction;
    let dx = profile.dx as f32;

    for i in 0..profile.temperatures.len() - 1 {
        let t0 = profile.temperatures[i];
        let t1 = profile.temperatures[i + 1];
        let norm0 = ((t0 - profile.min_temp) / range) as f32;
        let norm1 = ((t1 - profile.min_temp) / range) as f32;

        let d0 = i as f32 * dx;
        let d1 = (i + 1) as f32 * dx;

        let p0 = [
            profile.origin[0] as f32 + dir[0] as f32 * d0,
            profile.origin[1] as f32 + dir[1] as f32 * d0,
            profile.origin[2] as f32 + dir[2] as f32 * d0,
        ];
        let p1 = [
            profile.origin[0] as f32 + dir[0] as f32 * d1,
            profile.origin[1] as f32 + dir[1] as f32 * d1,
            profile.origin[2] as f32 + dir[2] as f32 * d1,
        ];

        // Blend color between the two endpoints
        let color = visualization_heat_map((norm0 + norm1) * 0.5);
        batch.line(p0, p1, color);
    }
}

// ── 3. Cycle diagrams ─────────────────────────────────────────────────────

/// Draw T-s and P-v cycle diagrams as colored line plots in a [`LineBatch`].
///
/// T-s diagram is drawn at `origin`, P-v diagram is offset by `pv_offset`
/// along the X axis. Both use the same scale parameters.
#[cfg(feature = "thermo")]
pub fn cycle_diagram_to_lines(
    cycle: &ushma::integration::soorat::CycleDiagramData,
    params: &CycleDiagramParams,
    pv_offset: f32,
    batch: &mut LineBatch,
) {
    // T-s diagram
    draw_diagram_lines(
        &cycle.ts_points,
        params.origin,
        params.x_scale,
        params.y_scale,
        params.ts_color,
        batch,
    );

    // P-v diagram (offset along X)
    let pv_origin = [
        params.origin[0] + pv_offset,
        params.origin[1],
        params.origin[2],
    ];
    draw_diagram_lines(
        &cycle.pv_points,
        pv_origin,
        params.x_scale,
        params.y_scale,
        params.pv_color,
        batch,
    );
}

/// Draw a 2D point sequence as connected line segments in the XY plane.
#[cfg(feature = "thermo")]
fn draw_diagram_lines(
    points: &[[f64; 2]],
    origin: [f32; 3],
    x_scale: f32,
    y_scale: f32,
    color: Color,
    batch: &mut LineBatch,
) {
    if points.len() < 2 {
        return;
    }

    // Ranges over finite coordinates only.
    let Some((min_x, max_x)) = finite_min_max(points.iter().map(|p| p[0])) else {
        return;
    };
    let Some((min_y, max_y)) = finite_min_max(points.iter().map(|p| p[1])) else {
        return;
    };
    let rx = (max_x - min_x).max(f64::EPSILON);
    let ry = (max_y - min_y).max(f64::EPSILON);

    for i in 0..points.len() - 1 {
        if !points[i]
            .iter()
            .chain(points[i + 1].iter())
            .all(|c| c.is_finite())
        {
            continue;
        }
        let nx0 = ((points[i][0] - min_x) / rx) as f32;
        let ny0 = ((points[i][1] - min_y) / ry) as f32;
        let nx1 = ((points[i + 1][0] - min_x) / rx) as f32;
        let ny1 = ((points[i + 1][1] - min_y) / ry) as f32;

        let p0 = [
            origin[0] + nx0 * x_scale,
            origin[1] + ny0 * y_scale,
            origin[2],
        ];
        let p1 = [
            origin[0] + nx1 * x_scale,
            origin[1] + ny1 * y_scale,
            origin[2],
        ];

        batch.line(p0, p1, color);
    }
}

// ── 4. Thermal network graph ──────────────────────────────────────────────

/// Draw a thermal network as a node-link diagram in a [`LineBatch`].
///
/// Nodes are colored by temperature (heat-mapped), edges are drawn as lines.
/// Node positions are laid out in a circle by default. `radius` controls the
/// circle size, `center` the world-space center.
#[cfg(feature = "thermo")]
pub fn thermal_network_to_lines(
    network: &ushma::integration::soorat::ThermalNetworkVisualization,
    center: [f32; 3],
    radius: f32,
    node_size: f32,
    batch: &mut LineBatch,
) {
    let n = network.node_temperatures.len();
    if n == 0 {
        return;
    }

    // Find temp range for color mapping
    let mut min_t = f64::MAX;
    let mut max_t = f64::MIN;
    for &t in &network.node_temperatures {
        if t < min_t {
            min_t = t;
        }
        if t > max_t {
            max_t = t;
        }
    }
    let range = (max_t - min_t).max(f64::EPSILON);

    // Lay out nodes in a circle
    let positions: Vec<[f32; 3]> = (0..n)
        .map(|i| {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / n as f32;
            [
                center[0] + radius * angle.cos(),
                center[1],
                center[2] + radius * angle.sin(),
            ]
        })
        .collect();

    // Draw edges
    let max_cond = network
        .conductances
        .iter()
        .cloned()
        .fold(0.0_f64, f64::max)
        .max(f64::EPSILON);

    for (i, edge) in network.edges.iter().enumerate() {
        let [a, b] = *edge;
        if a >= n || b >= n {
            continue;
        }

        // Color blend between the two node temperatures
        let ta = ((network.node_temperatures[a] - min_t) / range) as f32;
        let tb = ((network.node_temperatures[b] - min_t) / range) as f32;
        let color = visualization_heat_map((ta + tb) * 0.5);

        batch.line(positions[a], positions[b], color);

        // If conductance is high, draw a second parallel line for visual weight
        let cond = if i < network.conductances.len() {
            network.conductances[i]
        } else {
            0.0
        };
        if cond / max_cond > 0.5 {
            let offset = [0.0, node_size * 0.1, 0.0];
            let pa = [
                positions[a][0] + offset[0],
                positions[a][1] + offset[1],
                positions[a][2] + offset[2],
            ];
            let pb = [
                positions[b][0] + offset[0],
                positions[b][1] + offset[1],
                positions[b][2] + offset[2],
            ];
            batch.line(pa, pb, color);
        }
    }

    // Draw nodes as small wireframe crosses
    for (i, pos) in positions.iter().enumerate() {
        let t = ((network.node_temperatures[i] - min_t) / range) as f32;
        let color = visualization_heat_map(t);
        let s = node_size;

        batch.line(
            [pos[0] - s, pos[1], pos[2]],
            [pos[0] + s, pos[1], pos[2]],
            color,
        );
        batch.line(
            [pos[0], pos[1] - s, pos[2]],
            [pos[0], pos[1] + s, pos[2]],
            color,
        );
        batch.line(
            [pos[0], pos[1], pos[2] - s],
            [pos[0], pos[1], pos[2] + s],
            color,
        );
    }
}

// ── 5. Heat flux arrows ──────────────────────────────────────────────────

/// Draw 2D heat flux vectors as arrow glyphs in a [`LineBatch`].
///
/// Each grid point gets a line segment (shaft) with an arrowhead on the XZ plane.
/// Arrow length is proportional to flux magnitude, color is heat-mapped.
#[cfg(feature = "thermo")]
pub fn heat_flux_to_arrows(
    flux: &ushma::integration::soorat::HeatFluxField,
    y_pos: f32,
    arrow_scale: f32,
    batch: &mut LineBatch,
) {
    let [nx, ny] = flux.dimensions;
    if !grid_is_backed(&flux.dimensions, flux.fluxes.len(), "heat_flux_to_arrows") {
        return;
    }
    tracing::debug!(nx, ny, "building heat flux arrows");

    let magnitude = |q: [f64; 2]| (q[0] * q[0] + q[1] * q[1]).sqrt();
    // Trust the stored maximum only if it is usable; otherwise measure it.
    let max_mag = if flux.max_magnitude.is_finite() && flux.max_magnitude > 0.0 {
        flux.max_magnitude as f32
    } else {
        finite_min_max(flux.fluxes.iter().map(|&q| magnitude(q))).map_or(0.0, |(_, hi)| hi as f32)
    }
    .max(f32::EPSILON);
    let dx = flux.spacing[0] as f32;
    let dy = flux.spacing[1] as f32;

    for iy in 0..ny {
        for ix in 0..nx {
            // In bounds: nx * ny <= fluxes.len() was checked above.
            let q = flux.fluxes[iy * nx + ix];
            let (qx, qy) = (q[0] as f32, q[1] as f32);
            let mag = (qx * qx + qy * qy).sqrt();

            if !mag.is_finite() || mag < f32::EPSILON {
                continue;
            }

            let dir_x = qx / mag;
            let dir_z = qy / mag; // map 2D Y → 3D Z
            // Never longer than `arrow_scale`, even if the stored maximum is stale.
            let t = (mag / max_mag).clamp(0.0, 1.0);
            let len = t * arrow_scale;

            let base = [ix as f32 * dx, y_pos, iy as f32 * dy];
            let tip = [base[0] + dir_x * len, y_pos, base[2] + dir_z * len];

            let color = visualization_heat_map(t);

            // Shaft
            batch.line(base, tip, color);

            // Arrowhead — two lines in the XZ plane
            let head_size = len * 0.2;
            let perp_x = -dir_z;
            let perp_z = dir_x;
            let back = [
                tip[0] - dir_x * head_size,
                y_pos,
                tip[2] - dir_z * head_size,
            ];
            batch.line(
                tip,
                [
                    back[0] + perp_x * head_size,
                    y_pos,
                    back[2] + perp_z * head_size,
                ],
                color,
            );
            batch.line(
                tip,
                [
                    back[0] - perp_x * head_size,
                    y_pos,
                    back[2] - perp_z * head_size,
                ],
                color,
            );
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
    fn thermal_heat_map_public() {
        let c = thermal_heat_map(0.5);
        assert_eq!(c.g, 1.0);
    }

    // ── Default params ─────────────────────────────────────────────────────

    #[test]
    fn thermal_vis_params_default() {
        let p = ThermalVisParams::default();
        assert_eq!(p.color_mode, ThermalColorMode::Temperature);
        assert_eq!(p.alpha, 1.0);
    }

    #[test]
    fn cycle_diagram_params_default() {
        let p = CycleDiagramParams::default();
        assert_eq!(p.origin, [0.0; 3]);
        assert_eq!(p.x_scale, 1.0);
        assert_eq!(p.y_scale, 1.0);
    }

    // ── Feature-gated tests ────────────────────────────────────────────────

    #[cfg(feature = "thermo")]
    mod thermo_tests {
        use super::*;
        use crate::debug_draw::LineBatch;

        use crate::viz::assert_winding_matches_normals;

        #[test]
        fn alpha_blend_alpha_stays_in_range() {
            let grid = ushma::integration::soorat::ThermalGridVisualization {
                values: vec![250.0, 500.0, f64::NAN, 350.0],
                dimensions: [2, 2],
                origin: [0.0, 0.0],
                spacing: [1.0, 1.0],
                min_temp: 300.0,
                max_temp: 400.0,
            };
            let params = ThermalVisParams {
                color_mode: ThermalColorMode::AlphaBlend,
                ..ThermalVisParams::default()
            };
            let (v, i) = thermal_grid_to_mesh(&grid, 0.0, &params);
            assert_eq!(v.len(), 16);
            for vert in &v {
                assert!(
                    (0.0..=1.0).contains(&vert.color[3]),
                    "alpha {}",
                    vert.color[3]
                );
            }
            assert_eq!(v[0].color[3], 0.0); // below range
            assert_eq!(v[4].color[3], 1.0); // above range
            assert_winding_matches_normals(&v, &i);
        }

        #[test]
        fn heat_flux_nan_and_stale_max_are_safe() {
            let flux = ushma::integration::soorat::HeatFluxField {
                fluxes: vec![[f64::NAN, 0.0], [50.0, 0.0]],
                dimensions: [2, 1],
                spacing: [1.0, 1.0],
                max_magnitude: 0.0,
            };
            let mut batch = LineBatch::new();
            heat_flux_to_arrows(&flux, 0.0, 1.0, &mut batch);
            assert_eq!(batch.line_count(), 3, "only the finite vector draws");
            assert!(
                batch
                    .vertices
                    .iter()
                    .all(|v| v.position.iter().all(|c| c.is_finite()))
            );
            let shaft_len = batch.vertices[1].position[0] - batch.vertices[0].position[0];
            assert!((shaft_len - 1.0).abs() < 1e-5, "arrow length {shaft_len}");
        }

        #[test]
        fn thermal_grid_empty() {
            let grid = ushma::integration::soorat::ThermalGridVisualization {
                values: vec![],
                dimensions: [0, 0],
                origin: [0.0, 0.0],
                spacing: [1.0, 1.0],
                min_temp: 0.0,
                max_temp: 0.0,
            };
            let (v, i) = thermal_grid_to_mesh(&grid, 0.0, &ThermalVisParams::default());
            assert!(v.is_empty());
            assert!(i.is_empty());
        }

        #[test]
        fn thermal_grid_basic() {
            let grid = ushma::integration::soorat::ThermalGridVisualization {
                values: vec![300.0, 350.0, 400.0, 450.0],
                dimensions: [2, 2],
                origin: [0.0, 0.0],
                spacing: [1.0, 1.0],
                min_temp: 300.0,
                max_temp: 450.0,
            };
            let (v, i) = thermal_grid_to_mesh(&grid, 0.0, &ThermalVisParams::default());
            assert_eq!(v.len(), 4 * 4);
            assert_eq!(i.len(), 4 * 6);
        }

        #[test]
        fn thermal_grid_alpha_blend() {
            let grid = ushma::integration::soorat::ThermalGridVisualization {
                values: vec![300.0, 400.0, 500.0, 600.0],
                dimensions: [2, 2],
                origin: [0.0, 0.0],
                spacing: [0.5, 0.5],
                min_temp: 300.0,
                max_temp: 600.0,
            };
            let params = ThermalVisParams {
                color_mode: ThermalColorMode::AlphaBlend,
                ..ThermalVisParams::default()
            };
            let (v, _) = thermal_grid_to_mesh(&grid, 1.0, &params);
            assert_eq!(v.len(), 16);
            // Coldest cell should have lowest alpha
            let cold_alpha = v[0].color[3];
            let hot_alpha = v[12].color[3]; // last cell
            assert!(hot_alpha > cold_alpha);
        }

        #[test]
        fn temperature_profile_empty() {
            let prof = ushma::integration::soorat::TemperatureProfile {
                temperatures: vec![],
                dx: 1.0,
                origin: [0.0; 3],
                direction: [1.0, 0.0, 0.0],
                min_temp: 0.0,
                max_temp: 0.0,
            };
            let mut batch = LineBatch::new();
            temperature_profile_to_lines(&prof, &mut batch);
            assert!(batch.is_empty());
        }

        #[test]
        fn temperature_profile_single() {
            let prof = ushma::integration::soorat::TemperatureProfile {
                temperatures: vec![300.0],
                dx: 1.0,
                origin: [0.0; 3],
                direction: [1.0, 0.0, 0.0],
                min_temp: 300.0,
                max_temp: 300.0,
            };
            let mut batch = LineBatch::new();
            temperature_profile_to_lines(&prof, &mut batch);
            assert!(batch.is_empty());
        }

        #[test]
        fn temperature_profile_basic() {
            let prof = ushma::integration::soorat::TemperatureProfile {
                temperatures: vec![300.0, 350.0, 400.0, 450.0],
                dx: 1.0,
                origin: [0.0; 3],
                direction: [1.0, 0.0, 0.0],
                min_temp: 300.0,
                max_temp: 450.0,
            };
            let mut batch = LineBatch::new();
            temperature_profile_to_lines(&prof, &mut batch);
            assert_eq!(batch.line_count(), 3);
        }

        #[test]
        fn cycle_diagram_empty() {
            let cycle = ushma::integration::soorat::CycleDiagramData {
                ts_points: vec![],
                pv_points: vec![],
                state_points: vec![],
                kind: "Empty".to_string(),
                efficiency: 0.0,
            };
            let mut batch = LineBatch::new();
            cycle_diagram_to_lines(&cycle, &CycleDiagramParams::default(), 5.0, &mut batch);
            assert!(batch.is_empty());
        }

        #[test]
        fn cycle_diagram_basic() {
            let cycle = ushma::integration::soorat::CycleDiagramData {
                ts_points: vec![[0.0, 300.0], [100.0, 600.0], [200.0, 300.0]],
                pv_points: vec![[0.001, 100000.0], [0.01, 50000.0], [0.001, 100000.0]],
                state_points: vec![],
                kind: "Otto".to_string(),
                efficiency: 0.56,
            };
            let mut batch = LineBatch::new();
            cycle_diagram_to_lines(&cycle, &CycleDiagramParams::default(), 5.0, &mut batch);
            // T-s: 2 segments + P-v: 2 segments = 4
            assert_eq!(batch.line_count(), 4);
        }

        #[test]
        fn thermal_network_empty() {
            let net = ushma::integration::soorat::ThermalNetworkVisualization {
                node_temperatures: vec![],
                edges: vec![],
                conductances: vec![],
            };
            let mut batch = LineBatch::new();
            thermal_network_to_lines(&net, [0.0; 3], 5.0, 0.2, &mut batch);
            assert!(batch.is_empty());
        }

        #[test]
        fn thermal_network_basic() {
            let net = ushma::integration::soorat::ThermalNetworkVisualization {
                node_temperatures: vec![300.0, 350.0, 400.0],
                edges: vec![[0, 1], [1, 2]],
                conductances: vec![0.1, 0.05],
            };
            let mut batch = LineBatch::new();
            thermal_network_to_lines(&net, [0.0; 3], 5.0, 0.2, &mut batch);
            // 2 edges + 1 double-line (high conductance) + 3 nodes × 3 crosses = 12
            assert!(batch.line_count() >= 11);
        }

        #[test]
        fn thermal_network_out_of_bounds_edge() {
            let net = ushma::integration::soorat::ThermalNetworkVisualization {
                node_temperatures: vec![300.0, 400.0],
                edges: vec![[0, 1], [0, 5]], // edge to non-existent node
                conductances: vec![0.1, 0.1],
            };
            let mut batch = LineBatch::new();
            thermal_network_to_lines(&net, [0.0; 3], 5.0, 0.2, &mut batch);
            // Should not crash, invalid edge is skipped
            assert!(batch.line_count() > 0);
        }

        #[test]
        fn heat_flux_empty() {
            let flux = ushma::integration::soorat::HeatFluxField {
                fluxes: vec![],
                dimensions: [0, 0],
                spacing: [1.0, 1.0],
                max_magnitude: 0.0,
            };
            let mut batch = LineBatch::new();
            heat_flux_to_arrows(&flux, 0.0, 1.0, &mut batch);
            assert!(batch.is_empty());
        }

        #[test]
        fn heat_flux_basic() {
            let flux = ushma::integration::soorat::HeatFluxField {
                fluxes: vec![[100.0, 0.0], [0.0, -50.0], [50.0, 50.0], [0.0, 0.0]],
                dimensions: [2, 2],
                spacing: [1.0, 1.0],
                max_magnitude: 100.0,
            };
            let mut batch = LineBatch::new();
            heat_flux_to_arrows(&flux, 0.0, 1.0, &mut batch);
            // 3 non-zero vectors × (1 shaft + 2 arrowhead) = 9
            assert_eq!(batch.line_count(), 9);
        }

        #[test]
        fn heat_flux_zero_vectors_skipped() {
            let flux = ushma::integration::soorat::HeatFluxField {
                fluxes: vec![[0.0, 0.0], [0.0, 0.0]],
                dimensions: [2, 1],
                spacing: [1.0, 1.0],
                max_magnitude: 0.0,
            };
            let mut batch = LineBatch::new();
            heat_flux_to_arrows(&flux, 0.0, 1.0, &mut batch);
            assert!(batch.is_empty());
        }

        #[test]
        fn thermal_grid_equal_temps() {
            // Adversarial: min_temp == max_temp (zero range) must not produce NaN/panic
            let grid = ushma::integration::soorat::ThermalGridVisualization {
                values: vec![350.0, 350.0, 350.0, 350.0],
                dimensions: [2, 2],
                origin: [0.0, 0.0],
                spacing: [1.0, 1.0],
                min_temp: 350.0,
                max_temp: 350.0,
            };
            let (v, i) = thermal_grid_to_mesh(&grid, 0.0, &ThermalVisParams::default());
            assert_eq!(v.len(), 4 * 4);
            assert_eq!(i.len(), 4 * 6);
            // No NaN in vertex colors or positions
            for vert in &v {
                for &p in &vert.position {
                    assert!(!p.is_nan(), "vertex position contains NaN");
                }
                for &c in &vert.color {
                    assert!(!c.is_nan(), "vertex color contains NaN");
                }
            }
        }

        #[test]
        fn heat_flux_single_cell() {
            // Adversarial: 1x1 grid must work correctly
            let flux = ushma::integration::soorat::HeatFluxField {
                fluxes: vec![[100.0, 50.0]],
                dimensions: [1, 1],
                spacing: [1.0, 1.0],
                max_magnitude: 111.8,
            };
            let mut batch = LineBatch::new();
            heat_flux_to_arrows(&flux, 0.0, 1.0, &mut batch);
            // 1 non-zero vector × (1 shaft + 2 arrowhead) = 3
            assert_eq!(batch.line_count(), 3);
        }
    }
}
