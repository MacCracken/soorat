//! Fluid rendering — SPH particle visualization and shallow water surface meshes.
//!
//! Requires feature: `fluids` (dep: pravash).

use crate::color::Color;
#[cfg(feature = "fluids")]
use crate::vertex::Vertex3D;

/// Color mapping mode for fluid particles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidColorMode {
    /// Uniform color for all particles.
    Solid,
    /// Color based on velocity magnitude (blue=slow → red=fast).
    Velocity,
    /// Color based on density (dark=low → bright=high).
    Density,
    /// Color based on pressure (cool=low → hot=high).
    Pressure,
}

/// Convert a scalar value [0,1] to a color gradient (blue → cyan → green → yellow → red).
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

/// Generate XZ-plane quads for SPH particles.
/// Each particle becomes a small axis-aligned quad positioned at its world coordinates.
/// `particle_size`: world-space size of each particle quad.
#[cfg(feature = "fluids")]
pub fn particles_to_quads(
    particles: &[pravash::common::FluidParticle],
    particle_size: f32,
    color_mode: FluidColorMode,
    base_color: Color,
    max_velocity: f32,
    max_density: f64,
    max_pressure: f64,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let count = particles.len();
    let mut vertices = Vec::with_capacity(count * 4);
    let mut indices = Vec::with_capacity(count * 6);
    let hs = particle_size * 0.5;

    for (i, p) in particles.iter().enumerate() {
        let pos = [
            p.position[0] as f32,
            p.position[1] as f32,
            p.position[2] as f32,
        ];

        let color = match color_mode {
            FluidColorMode::Solid => base_color,
            FluidColorMode::Velocity => {
                let speed = p.speed() as f32;
                let t = (speed / max_velocity.max(0.001)).clamp(0.0, 1.0);
                heat_map(t)
            }
            FluidColorMode::Density => {
                let t = (p.density / max_density.max(0.001)) as f32;
                heat_map(t.clamp(0.0, 1.0))
            }
            FluidColorMode::Pressure => {
                let t = (p.pressure / max_pressure.max(0.001)) as f32;
                heat_map(t.clamp(0.0, 1.0))
            }
        };
        let c = color.to_array();

        // Simple XZ-plane quad (for top-down 2D fluid view)
        let base = (i * 4) as u32;
        vertices.push(Vertex3D {
            position: [pos[0] - hs, pos[1], pos[2] - hs],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 0.0],
            color: c,
        });
        vertices.push(Vertex3D {
            position: [pos[0] + hs, pos[1], pos[2] - hs],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [1.0, 0.0],
            color: c,
        });
        vertices.push(Vertex3D {
            position: [pos[0] + hs, pos[1], pos[2] + hs],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [1.0, 1.0],
            color: c,
        });
        vertices.push(Vertex3D {
            position: [pos[0] - hs, pos[1], pos[2] + hs],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 1.0],
            color: c,
        });

        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base + 2);
        indices.push(base + 3);
        indices.push(base);
    }

    (vertices, indices)
}

/// Generate a mesh from a pravash ShallowWater height field.
/// The mesh is an XZ-plane grid with Y = water height.
/// Normals are computed from neighboring heights.
#[cfg(feature = "fluids")]
pub fn shallow_water_to_mesh(
    water: &pravash::shallow::ShallowWater,
    y_scale: f32,
    color: Color,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let nx = water.nx;
    let ny = water.ny;
    let dx = water.dx as f32;
    let c = color.to_array();

    let cols = nx;
    let rows = ny;
    let mut vertices = Vec::with_capacity(cols * rows);
    let mut indices = Vec::with_capacity((cols - 1) * (rows - 1) * 6);

    // Generate vertices
    for z in 0..rows {
        for x in 0..cols {
            let idx = z * cols + x;
            let h = water.height[idx] as f32 * y_scale;
            let px = x as f32 * dx - (cols as f32 * dx * 0.5);
            let pz = z as f32 * dx - (rows as f32 * dx * 0.5);

            // Compute normal from neighbors
            let hl = if x > 0 {
                water.height[z * cols + x - 1] as f32 * y_scale
            } else {
                h
            };
            let hr = if x < cols - 1 {
                water.height[z * cols + x + 1] as f32 * y_scale
            } else {
                h
            };
            let hd = if z > 0 {
                water.height[(z - 1) * cols + x] as f32 * y_scale
            } else {
                h
            };
            let hu = if z < rows - 1 {
                water.height[(z + 1) * cols + x] as f32 * y_scale
            } else {
                h
            };

            let ndx = (hl - hr) / (2.0 * dx);
            let ndz = (hd - hu) / (2.0 * dx);
            let len = (ndx * ndx + 1.0 + ndz * ndz).sqrt();

            vertices.push(Vertex3D {
                position: [px, h, pz],
                normal: [ndx / len, 1.0 / len, ndz / len],
                tex_coords: [
                    x as f32 / (cols - 1).max(1) as f32,
                    z as f32 / (rows - 1).max(1) as f32,
                ],
                color: c,
            });
        }
    }

    // Generate indices
    for z in 0..(rows - 1) {
        for x in 0..(cols - 1) {
            let tl = (z * cols + x) as u32;
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

/// Parameters for particle rendering (color mapping + normalization ranges).
pub struct ParticleColorParams {
    pub color_mode: FluidColorMode,
    pub base_color: Color,
    pub max_velocity: f32,
    pub max_density: f64,
    pub max_pressure: f64,
}

impl Default for ParticleColorParams {
    fn default() -> Self {
        Self {
            color_mode: FluidColorMode::Solid,
            base_color: Color::WHITE,
            max_velocity: 10.0,
            max_density: 1000.0,
            max_pressure: 1000.0,
        }
    }
}

/// Generate camera-facing billboard quads for particles.
/// Unlike `particles_to_quads` (XZ-plane), these orient toward the camera.
/// `camera_right`/`camera_up`: camera basis vectors in world space.
#[cfg(feature = "fluids")]
pub fn particles_to_billboards(
    particles: &[pravash::common::FluidParticle],
    particle_size: f32,
    camera_right: [f32; 3],
    camera_up: [f32; 3],
    params: &ParticleColorParams,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let count = particles.len();
    let mut vertices = Vec::with_capacity(count * 4);
    let mut indices = Vec::with_capacity(count * 6);
    let hs = particle_size * 0.5;

    let r = [
        camera_right[0] * hs,
        camera_right[1] * hs,
        camera_right[2] * hs,
    ];
    let u = [camera_up[0] * hs, camera_up[1] * hs, camera_up[2] * hs];

    // Billboard normal = cross(right, up)
    let n = [
        camera_right[1] * camera_up[2] - camera_right[2] * camera_up[1],
        camera_right[2] * camera_up[0] - camera_right[0] * camera_up[2],
        camera_right[0] * camera_up[1] - camera_right[1] * camera_up[0],
    ];

    for (i, p) in particles.iter().enumerate() {
        let pos = [
            p.position[0] as f32,
            p.position[1] as f32,
            p.position[2] as f32,
        ];

        let color = match params.color_mode {
            FluidColorMode::Solid => params.base_color,
            FluidColorMode::Velocity => {
                let speed = p.speed() as f32;
                heat_map((speed / params.max_velocity.max(0.001)).clamp(0.0, 1.0))
            }
            FluidColorMode::Density => {
                heat_map(((p.density / params.max_density.max(0.001)) as f32).clamp(0.0, 1.0))
            }
            FluidColorMode::Pressure => {
                heat_map(((p.pressure / params.max_pressure.max(0.001)) as f32).clamp(0.0, 1.0))
            }
        };
        let c = color.to_array();
        let base = (i * 4) as u32;

        // Billboard corners: center ± right ± up
        vertices.push(Vertex3D {
            position: [
                pos[0] - r[0] - u[0],
                pos[1] - r[1] - u[1],
                pos[2] - r[2] - u[2],
            ],
            normal: n,
            tex_coords: [0.0, 0.0],
            color: c,
        });
        vertices.push(Vertex3D {
            position: [
                pos[0] + r[0] - u[0],
                pos[1] + r[1] - u[1],
                pos[2] + r[2] - u[2],
            ],
            normal: n,
            tex_coords: [1.0, 0.0],
            color: c,
        });
        vertices.push(Vertex3D {
            position: [
                pos[0] + r[0] + u[0],
                pos[1] + r[1] + u[1],
                pos[2] + r[2] + u[2],
            ],
            normal: n,
            tex_coords: [1.0, 1.0],
            color: c,
        });
        vertices.push(Vertex3D {
            position: [
                pos[0] - r[0] + u[0],
                pos[1] - r[1] + u[1],
                pos[2] - r[2] + u[2],
            ],
            normal: n,
            tex_coords: [0.0, 1.0],
            color: c,
        });

        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base + 2);
        indices.push(base + 3);
        indices.push(base);
    }

    (vertices, indices)
}

// ── CPU-only helpers (no feature gate) ──────────────────────────────────────

/// Heat map color for visualization (exposed for general use).
pub fn visualization_heat_map(t: f32) -> Color {
    heat_map(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heat_map_endpoints() {
        let cold = heat_map(0.0);
        assert_eq!(cold.b, 1.0); // blue
        let hot = heat_map(1.0);
        assert_eq!(hot.r, 1.0); // red
        assert_eq!(hot.g, 0.0);
    }

    #[test]
    fn heat_map_midpoint() {
        let mid = heat_map(0.5);
        assert_eq!(mid.g, 1.0); // green
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
        // Should produce valid colors at all points
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let c = heat_map(t);
            assert!(c.r >= 0.0 && c.r <= 1.0);
            assert!(c.g >= 0.0 && c.g <= 1.0);
            assert!(c.b >= 0.0 && c.b <= 1.0);
            assert_eq!(c.a, 1.0);
        }
    }

    #[cfg(feature = "fluids")]
    #[test]
    fn particles_to_quads_empty() {
        let (v, i) =
            particles_to_quads(&[], 0.1, FluidColorMode::Solid, Color::BLUE, 1.0, 1.0, 1.0);
        assert!(v.is_empty());
        assert!(i.is_empty());
    }

    #[cfg(feature = "fluids")]
    #[test]
    fn particles_to_quads_single() {
        let p = pravash::common::FluidParticle::new([1.0, 2.0, 3.0], 1.0);
        let (v, i) = particles_to_quads(
            &[p],
            0.5,
            FluidColorMode::Solid,
            Color::BLUE,
            1.0,
            1000.0,
            1.0,
        );
        assert_eq!(v.len(), 4);
        assert_eq!(i.len(), 6);
    }

    #[cfg(feature = "fluids")]
    #[test]
    fn particles_velocity_color() {
        let mut p = pravash::common::FluidParticle::new([0.0; 3], 1.0);
        p.velocity = [10.0, 0.0, 0.0];
        let (v, _) = particles_to_quads(
            &[p],
            0.1,
            FluidColorMode::Velocity,
            Color::WHITE,
            10.0,
            1.0,
            1.0,
        );
        // Fast particle should be red (heat map at 1.0)
        assert_eq!(v[0].color[0], 1.0); // red
    }

    #[cfg(feature = "fluids")]
    #[test]
    fn shallow_water_mesh() {
        let water = pravash::shallow::ShallowWater::new(4, 4, 1.0, 1.0).unwrap();
        let (v, i) = shallow_water_to_mesh(&water, 1.0, Color::BLUE);
        assert_eq!(v.len(), 16); // 4*4
        assert_eq!(i.len(), 9 * 6); // 3*3 quads * 6 indices
    }

    #[cfg(feature = "fluids")]
    #[test]
    fn billboard_single_particle() {
        let p = pravash::common::FluidParticle::new([0.0, 1.0, 0.0], 1.0);
        let (v, i) = particles_to_billboards(
            &[p],
            1.0,
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            &ParticleColorParams::default(),
        );
        assert_eq!(v.len(), 4);
        assert_eq!(i.len(), 6);
        // Normal should face camera (cross of right × up = forward = [0,0,1])
        assert!((v[0].normal[2] - 1.0).abs() < 0.01);
    }

    #[cfg(feature = "fluids")]
    #[test]
    fn billboard_empty() {
        let (v, i) = particles_to_billboards(
            &[],
            1.0,
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            &ParticleColorParams::default(),
        );
        assert!(v.is_empty());
        assert!(i.is_empty());
    }

    #[cfg(feature = "fluids")]
    #[test]
    fn shallow_water_normals_flat() {
        let water = pravash::shallow::ShallowWater::new(3, 3, 1.0, 5.0).unwrap();
        let (v, _) = shallow_water_to_mesh(&water, 1.0, Color::BLUE);
        // Flat water → normals point up
        for vert in &v {
            assert!((vert.normal[1] - 1.0).abs() < 0.01);
        }
    }
}
