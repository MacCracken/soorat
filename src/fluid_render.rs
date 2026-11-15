//! Fluid rendering — SPH particle visualization and shallow water surface meshes.
//!
//! Requires feature: `fluids` (dep: pravash).

use crate::color::Color;
#[cfg(feature = "fluids")]
use crate::vertex::Vertex3D;
#[cfg(feature = "fluids")]
use crate::viz::{HeightField, MAX_QUADS, XZ_QUAD_INDICES, grid_is_backed, heightfield_mesh};

/// Color mapping mode for fluid particles.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidColorMode {
    /// Uniform color for all particles.
    Solid,
    /// Color based on velocity magnitude (blue=slow → red=fast).
    Velocity,
    /// Color based on density (heat map: blue=low → red=high).
    Density,
    /// Color based on pressure (cool=low → hot=high).
    Pressure,
}

use crate::color::visualization_heat_map;

/// Color for a particle under `params` (heat-mapped speed/density/pressure
/// relative to the configured maxima, or the solid base color).
#[cfg(feature = "fluids")]
#[must_use]
#[inline]
fn particle_color(p: &pravash::common::FluidParticle, params: &ParticleColorParams) -> Color {
    match params.color_mode {
        FluidColorMode::Solid => params.base_color,
        FluidColorMode::Velocity => {
            visualization_heat_map(p.speed() as f32 / params.max_velocity.max(0.001))
        }
        FluidColorMode::Density => {
            visualization_heat_map((p.density / params.max_density.max(0.001)) as f32)
        }
        FluidColorMode::Pressure => {
            visualization_heat_map((p.pressure / params.max_pressure.max(0.001)) as f32)
        }
    }
}

/// Generate XZ-plane quads for SPH particles.
/// Each particle becomes a small axis-aligned, +Y-facing quad positioned at
/// its world coordinates.
/// `particle_size`: world-space size of each particle quad.
#[must_use]
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
    let params = ParticleColorParams {
        color_mode,
        base_color,
        max_velocity,
        max_density,
        max_pressure,
    };
    let count = particles.len().min(MAX_QUADS);
    tracing::debug!(count, "building particle quads");
    let mut vertices = Vec::with_capacity(count * 4);
    let mut indices = Vec::with_capacity(count * 6);
    let hs = particle_size * 0.5;
    let up = [0.0, 1.0, 0.0];

    for p in particles.iter().take(count) {
        let [x, y, z] = [
            p.position[0] as f32,
            p.position[1] as f32,
            p.position[2] as f32,
        ];
        let color = particle_color(p, &params).to_array();

        // count <= MAX_QUADS, so base + 3 always fits in u32.
        let base = vertices.len() as u32;
        for (position, tex_coords) in [
            ([x - hs, y, z - hs], [0.0, 0.0]),
            ([x + hs, y, z - hs], [1.0, 0.0]),
            ([x + hs, y, z + hs], [1.0, 1.0]),
            ([x - hs, y, z + hs], [0.0, 1.0]),
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

    (vertices, indices)
}

/// Generate a mesh from a pravash ShallowWater height field.
/// The mesh is an XZ-plane grid centred on the origin with Y = water height ×
/// `y_scale`; normals come from central differences (one-sided at borders).
///
/// pravash's `height` is the free-surface elevation η, not the depth (depth
/// is `height - ground`), so the mesh is the water surface over any
/// bathymetry; `ground` is not added. Dry cells sit at the bed (η = ground).
///
/// Returns an empty mesh if `nx`/`ny` < 2, `dx` is not positive and finite, or
/// `height` holds fewer than `nx * ny` samples. Non-finite heights render as 0.
#[must_use]
#[cfg(feature = "fluids")]
pub fn shallow_water_to_mesh(
    water: &pravash::shallow::ShallowWater,
    y_scale: f32,
    color: Color,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let (cols, rows) = (water.nx, water.ny);
    let dx = water.dx as f32;
    if cols < 2
        || rows < 2
        || !(dx.is_finite() && dx > 0.0)
        || !grid_is_backed(&[cols, rows], water.height.len(), "shallow_water_to_mesh")
    {
        return (Vec::new(), Vec::new());
    }
    tracing::debug!(cols, rows, "building shallow water mesh");

    // Centre the (cols-1)·dx × (rows-1)·dx extent on the origin.
    let field = HeightField {
        cols,
        rows,
        origin_x: -((cols - 1) as f32) * dx * 0.5,
        origin_z: -((rows - 1) as f32) * dx * 0.5,
        dx,
        dz: dx,
    };
    let c = color.to_array();
    heightfield_mesh(
        &field,
        |ix, iz| water.height[iz * cols + ix] as f32 * y_scale,
        |_, _, _| c,
    )
}

/// Parameters for particle rendering (color mapping + normalization ranges).
pub struct ParticleColorParams {
    /// How particles are colored.
    pub color_mode: FluidColorMode,
    /// Color used by [`FluidColorMode::Solid`].
    pub base_color: Color,
    /// Speed mapped to the top of the heat map (floored at 0.001).
    pub max_velocity: f32,
    /// Density mapped to the top of the heat map (floored at 0.001).
    pub max_density: f64,
    /// Pressure mapped to the top of the heat map (floored at 0.001).
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
#[must_use]
#[cfg(feature = "fluids")]
pub fn particles_to_billboards(
    particles: &[pravash::common::FluidParticle],
    particle_size: f32,
    camera_right: [f32; 3],
    camera_up: [f32; 3],
    params: &ParticleColorParams,
) -> (Vec<Vertex3D>, Vec<u32>) {
    let count = particles.len().min(MAX_QUADS);
    tracing::debug!(count, "building particle billboards");
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

    for p in particles.iter().take(count) {
        let pos = [
            p.position[0] as f32,
            p.position[1] as f32,
            p.position[2] as f32,
        ];

        let c = particle_color(p, params).to_array();
        // count <= MAX_QUADS, so base + 3 always fits in u32.
        let base = vertices.len() as u32;

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

/// Fluid heat map exposed for general use.
#[must_use]
pub fn fluid_heat_map(t: f32) -> Color {
    visualization_heat_map(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "fluids")]
    #[test]
    fn particle_quads_face_up() {
        let p = pravash::common::FluidParticle::new([1.0, 2.0, 3.0].into(), 1.0);
        let (v, i) =
            particles_to_quads(&[p], 0.5, FluidColorMode::Solid, Color::BLUE, 1.0, 1.0, 1.0);
        crate::viz::assert_winding_matches_normals(&v, &i);
    }

    #[cfg(feature = "fluids")]
    #[test]
    fn shallow_water_centred_and_upward() {
        let mut water = pravash::shallow::ShallowWater::new(4, 3, 0.5, 1.0).unwrap();
        for (k, h) in water.height.iter_mut().enumerate() {
            *h = 1.0 + 0.1 * (k as f64).sin();
        }
        let (v, i) = shallow_water_to_mesh(&water, 1.0, Color::BLUE);
        assert_eq!(v.len(), 12);
        crate::viz::assert_winding_matches_normals(&v, &i);
        // Extent (cols-1)·dx = 1.5 centred on 0 → x ∈ [-0.75, 0.75].
        assert!((v[0].position[0] + 0.75).abs() < 1e-6);
        assert!((v[3].position[0] - 0.75).abs() < 1e-6);
    }

    #[cfg(feature = "fluids")]
    #[test]
    fn shallow_water_renders_the_free_surface_over_bathymetry() {
        // pravash stores `height` as the surface elevation η (depth is
        // `height - ground`), so the mesh follows `height` and must not add
        // `ground` again.
        let mut water = pravash::shallow::ShallowWater::new(3, 3, 1.0, 1.0).unwrap();
        for k in 0..9 {
            water.ground[k] = 0.25 * k as f64; // a sloping bed
            water.height[k] = water.ground[k] + 0.5; // uniform depth 0.5
        }
        let (v, _) = shallow_water_to_mesh(&water, 2.0, Color::BLUE);
        for (k, vertex) in v.iter().enumerate() {
            let want = (water.height[k] * 2.0) as f32;
            assert!(
                (vertex.position[1] - want).abs() < 1e-6,
                "vertex {k}: y {} vs η·scale {want}",
                vertex.position[1]
            );
        }
    }

    #[cfg(feature = "fluids")]
    #[test]
    fn shallow_water_rejects_inconsistent_fields() {
        let water = pravash::shallow::ShallowWater::new(3, 3, 1.0, 1.0).unwrap();
        let mut bad = water.clone();
        bad.dx = 0.0;
        assert!(shallow_water_to_mesh(&bad, 1.0, Color::BLUE).0.is_empty());
        let mut bad = water.clone();
        bad.height.truncate(4);
        assert!(shallow_water_to_mesh(&bad, 1.0, Color::BLUE).0.is_empty());
        let mut bad = water;
        bad.nx = 0;
        assert!(shallow_water_to_mesh(&bad, 1.0, Color::BLUE).0.is_empty());
    }

    #[test]
    fn heat_map_endpoints() {
        let cold = visualization_heat_map(0.0);
        assert_eq!(cold.b, 1.0); // blue
        let hot = visualization_heat_map(1.0);
        assert_eq!(hot.r, 1.0); // red
        assert_eq!(hot.g, 0.0);
    }

    #[test]
    fn heat_map_midpoint() {
        let mid = visualization_heat_map(0.5);
        assert_eq!(mid.g, 1.0); // green
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
        // Should produce valid colors at all points
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let c = visualization_heat_map(t);
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
        let p = pravash::common::FluidParticle::new([1.0, 2.0, 3.0].into(), 1.0);
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
        let mut p = pravash::common::FluidParticle::new([0.0; 3].into(), 1.0);
        p.velocity = [10.0, 0.0, 0.0].into();
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
        let p = pravash::common::FluidParticle::new([0.0, 1.0, 0.0].into(), 1.0);
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
