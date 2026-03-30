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

/// Thermal heat map: scalar \[0,1\] → blue→cyan→green→yellow→red.
fn heat_map(t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    if t < 0.25 {
        let s = t / 0.25;
        Color::new(0.0, s, 1.0, 1.0)
    } else if t < 0.5 {
        let s = (t - 0.25) / 0.25;
        Color::new(0.0, 1.0, 1.0 - s, 1.0)
    } else if t < 0.75 {
        let s = (t - 0.5) / 0.25;
        Color::new(s, 1.0, 0.0, 1.0)
    } else {
        let s = (t - 0.75) / 0.25;
        Color::new(1.0, 1.0 - s, 0.0, 1.0)
    }
}

/// Thermal heat map exposed for general use.
#[must_use]
pub fn thermal_heat_map(t: f32) -> Color {
    heat_map(t)
}

// ── 1. Thermal grid heatmap ───────────────────────────────────────────────

/// Generate a mesh for a 2D thermal grid as a colored heatmap.
///
/// Each grid cell becomes a colored quad on the XZ plane at `y_pos`.
/// Temperature values are normalized to \[0,1\] using min/max for color mapping.
#[cfg(feature = "thermo")]
pub fn thermal_grid_to_mesh(
    grid: &ushma::integration::soorat::ThermalGridVisualization,
    y_pos: f32,
    params: &ThermalVisParams,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let [nx, ny] = grid.dimensions;
    if nx == 0 || ny == 0 || grid.values.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let range = (grid.max_temp - grid.min_temp).max(f64::EPSILON);

    let mut vertices = Vec::with_capacity(nx * ny * 4);
    let mut indices = Vec::with_capacity(nx * ny * 6);

    let ox = grid.origin[0] as f32;
    let oz = grid.origin[1] as f32;
    let dx = grid.spacing[0] as f32;
    let dz = grid.spacing[1] as f32;

    for iy in 0..ny {
        for ix in 0..nx {
            let idx = iy * nx + ix;
            let v = if idx < grid.values.len() {
                grid.values[idx]
            } else {
                grid.min_temp
            };

            let t = ((v - grid.min_temp) / range) as f32;
            let color = match params.color_mode {
                ThermalColorMode::Temperature => {
                    let mut c = heat_map(t);
                    c.a = params.alpha;
                    c
                }
                ThermalColorMode::AlphaBlend => {
                    let mut c = params.base_color;
                    c.a = t * params.alpha;
                    c
                }
            };
            let c = color.to_array();

            let x0 = ox + ix as f32 * dx;
            let z0 = oz + iy as f32 * dz;

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
                position: [x0 + dx, y_pos, z0],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [
                    (ix + 1) as f32 / nx.max(1) as f32,
                    iy as f32 / ny.max(1) as f32,
                ],
                color: c,
            });
            vertices.push(Vertex3D {
                position: [x0 + dx, y_pos, z0 + dz],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [
                    (ix + 1) as f32 / nx.max(1) as f32,
                    (iy + 1) as f32 / ny.max(1) as f32,
                ],
                color: c,
            });
            vertices.push(Vertex3D {
                position: [x0, y_pos, z0 + dz],
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
        let color = heat_map((norm0 + norm1) * 0.5);
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

    // Find ranges for normalization
    let (mut min_x, mut max_x) = (f64::MAX, f64::MIN);
    let (mut min_y, mut max_y) = (f64::MAX, f64::MIN);
    for p in points {
        if p[0] < min_x {
            min_x = p[0];
        }
        if p[0] > max_x {
            max_x = p[0];
        }
        if p[1] < min_y {
            min_y = p[1];
        }
        if p[1] > max_y {
            max_y = p[1];
        }
    }
    let rx = (max_x - min_x).max(f64::EPSILON);
    let ry = (max_y - min_y).max(f64::EPSILON);

    for i in 0..points.len() - 1 {
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
        let color = heat_map((ta + tb) * 0.5);

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
        let color = heat_map(t);
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
    if flux.fluxes.is_empty() || nx == 0 || ny == 0 {
        return;
    }

    let max_mag = (flux.max_magnitude as f32).max(f32::EPSILON);
    let dx = flux.spacing[0] as f32;
    let dy = flux.spacing[1] as f32;

    for iy in 0..ny {
        for ix in 0..nx {
            let idx = iy * nx + ix;
            if idx >= flux.fluxes.len() {
                continue;
            }

            let qx = flux.fluxes[idx][0] as f32;
            let qy = flux.fluxes[idx][1] as f32;
            let mag = (qx * qx + qy * qy).sqrt();

            if mag < f32::EPSILON {
                continue;
            }

            let dir_x = qx / mag;
            let dir_z = qy / mag; // map 2D Y → 3D Z
            let len = (mag / max_mag) * arrow_scale;

            let base = [ix as f32 * dx, y_pos, iy as f32 * dy];
            let tip = [base[0] + dir_x * len, y_pos, base[2] + dir_z * len];

            let t = (mag / max_mag).clamp(0.0, 1.0);
            let color = heat_map(t);

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
        let cold = heat_map(0.0);
        assert_eq!(cold.b, 1.0);
        let hot = heat_map(1.0);
        assert_eq!(hot.r, 1.0);
        assert_eq!(hot.g, 0.0);
    }

    #[test]
    fn heat_map_clamps() {
        let under = heat_map(-1.0);
        assert_eq!(under.b, 1.0);
        let over = heat_map(2.0);
        assert_eq!(over.r, 1.0);
    }

    #[test]
    fn heat_map_gradient_smooth() {
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let c = heat_map(t);
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
    }
}
