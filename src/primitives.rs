//! Built-in mesh primitives — cube, sphere, plane, cylinder.
//! Generates Vertex3D + index data for common shapes without requiring glTF loading.

use crate::vertex::Vertex3D;

/// Generate a unit cube centered at origin (side length 1.0).
#[must_use]
pub fn cube() -> (Vec<Vertex3D>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    // 6 faces, 4 vertices each, unique normals per face
    type Face = ([f32; 3], [[f32; 3]; 4], [[f32; 2]; 4]);
    let faces: [Face; 6] = [
        // +Z front
        (
            [0.0, 0.0, 1.0],
            [
                [-0.5, -0.5, 0.5],
                [0.5, -0.5, 0.5],
                [0.5, 0.5, 0.5],
                [-0.5, 0.5, 0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
        // -Z back
        (
            [0.0, 0.0, -1.0],
            [
                [0.5, -0.5, -0.5],
                [-0.5, -0.5, -0.5],
                [-0.5, 0.5, -0.5],
                [0.5, 0.5, -0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
        // +X right
        (
            [1.0, 0.0, 0.0],
            [
                [0.5, -0.5, 0.5],
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [0.5, 0.5, 0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
        // -X left
        (
            [-1.0, 0.0, 0.0],
            [
                [-0.5, -0.5, -0.5],
                [-0.5, -0.5, 0.5],
                [-0.5, 0.5, 0.5],
                [-0.5, 0.5, -0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
        // +Y top
        (
            [0.0, 1.0, 0.0],
            [
                [-0.5, 0.5, 0.5],
                [0.5, 0.5, 0.5],
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
        // -Y bottom
        (
            [0.0, -1.0, 0.0],
            [
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, -0.5, 0.5],
                [-0.5, -0.5, 0.5],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        ),
    ];

    for (normal, positions, uvs) in &faces {
        let base = vertices.len() as u32;
        for j in 0..4 {
            vertices.push(Vertex3D {
                position: positions[j],
                normal: *normal,
                tex_coords: uvs[j],
                color: [1.0, 1.0, 1.0, 1.0],
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }

    (vertices, indices)
}

/// Generate a UV sphere centered at origin with radius 0.5.
#[must_use]
pub fn sphere(segments: u32, rings: u32) -> (Vec<Vertex3D>, Vec<u32>) {
    let segments = segments.max(3);
    let rings = rings.max(2);

    let mut vertices = Vec::with_capacity(((rings + 1) * (segments + 1)) as usize);
    let mut indices = Vec::with_capacity((rings * segments * 6) as usize);

    for y in 0..=rings {
        let v = y as f32 / rings as f32;
        let phi = v * std::f32::consts::PI;

        for x in 0..=segments {
            let u = x as f32 / segments as f32;
            let theta = u * std::f32::consts::TAU;

            let nx = phi.sin() * theta.cos();
            let ny = phi.cos();
            let nz = phi.sin() * theta.sin();

            vertices.push(Vertex3D {
                position: [nx * 0.5, ny * 0.5, nz * 0.5],
                normal: [nx, ny, nz],
                tex_coords: [u, v],
                color: [1.0, 1.0, 1.0, 1.0],
            });
        }
    }

    for y in 0..rings {
        for x in 0..segments {
            let a = y * (segments + 1) + x;
            let b = a + segments + 1;
            indices.extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
        }
    }

    (vertices, indices)
}

/// Generate an XZ plane centered at origin (side length 1.0).
#[must_use]
pub fn plane(subdivisions: u32) -> (Vec<Vertex3D>, Vec<u32>) {
    let divs = subdivisions.max(1);
    let cols = divs + 1;
    let rows = divs + 1;

    let mut vertices = Vec::with_capacity((cols * rows) as usize);
    let mut indices = Vec::with_capacity((divs * divs * 6) as usize);

    for z in 0..rows {
        for x in 0..cols {
            let px = x as f32 / divs as f32 - 0.5;
            let pz = z as f32 / divs as f32 - 0.5;
            vertices.push(Vertex3D {
                position: [px, 0.0, pz],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [x as f32 / divs as f32, z as f32 / divs as f32],
                color: [1.0, 1.0, 1.0, 1.0],
            });
        }
    }

    for z in 0..divs {
        for x in 0..divs {
            let tl = z * cols + x;
            let tr = tl + 1;
            let bl = tl + cols;
            let br = bl + 1;
            indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }

    (vertices, indices)
}

/// Generate a cylinder along the Y axis, centered at origin (radius 0.5, height 1.0).
#[must_use]
pub fn cylinder(segments: u32) -> (Vec<Vertex3D>, Vec<u32>) {
    let segments = segments.max(3);
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let step = std::f32::consts::TAU / segments as f32;

    // Side vertices (2 rings)
    for y_idx in 0..=1u32 {
        let y = y_idx as f32 - 0.5;
        for i in 0..=segments {
            let angle = step * i as f32;
            let nx = angle.cos();
            let nz = angle.sin();
            vertices.push(Vertex3D {
                position: [nx * 0.5, y, nz * 0.5],
                normal: [nx, 0.0, nz],
                tex_coords: [i as f32 / segments as f32, y_idx as f32],
                color: [1.0, 1.0, 1.0, 1.0],
            });
        }
    }

    // Side indices
    let ring_size = segments + 1;
    for i in 0..segments {
        let a = i;
        let b = i + ring_size;
        indices.extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
    }

    // Top cap
    let top_center = vertices.len() as u32;
    vertices.push(Vertex3D {
        position: [0.0, 0.5, 0.0],
        normal: [0.0, 1.0, 0.0],
        tex_coords: [0.5, 0.5],
        color: [1.0, 1.0, 1.0, 1.0],
    });
    for i in 0..segments {
        let angle = step * i as f32;
        vertices.push(Vertex3D {
            position: [angle.cos() * 0.5, 0.5, angle.sin() * 0.5],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [angle.cos() * 0.5 + 0.5, angle.sin() * 0.5 + 0.5],
            color: [1.0, 1.0, 1.0, 1.0],
        });
    }
    for i in 0..segments {
        let next = if i + 1 < segments { i + 1 } else { 0 };
        indices.extend_from_slice(&[top_center, top_center + 1 + i, top_center + 1 + next]);
    }

    // Bottom cap
    let bot_center = vertices.len() as u32;
    vertices.push(Vertex3D {
        position: [0.0, -0.5, 0.0],
        normal: [0.0, -1.0, 0.0],
        tex_coords: [0.5, 0.5],
        color: [1.0, 1.0, 1.0, 1.0],
    });
    for i in 0..segments {
        let angle = step * i as f32;
        vertices.push(Vertex3D {
            position: [angle.cos() * 0.5, -0.5, angle.sin() * 0.5],
            normal: [0.0, -1.0, 0.0],
            tex_coords: [angle.cos() * 0.5 + 0.5, angle.sin() * 0.5 + 0.5],
            color: [1.0, 1.0, 1.0, 1.0],
        });
    }
    for i in 0..segments {
        let next = if i + 1 < segments { i + 1 } else { 0 };
        indices.extend_from_slice(&[bot_center, bot_center + 1 + next, bot_center + 1 + i]);
    }

    (vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_geometry() {
        let (v, i) = cube();
        assert_eq!(v.len(), 24); // 6 faces * 4 verts
        assert_eq!(i.len(), 36); // 6 faces * 6 indices
    }

    #[test]
    fn cube_indices_valid() {
        let (v, i) = cube();
        for &idx in &i {
            assert!(idx < v.len() as u32);
        }
    }

    #[test]
    fn sphere_geometry() {
        let (v, i) = sphere(16, 8);
        assert!(!v.is_empty());
        assert!(!i.is_empty());
        for &idx in &i {
            assert!(idx < v.len() as u32);
        }
    }

    #[test]
    fn sphere_normals_unit() {
        let (v, _) = sphere(8, 4);
        for vert in &v {
            let n = vert.normal;
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 0.01, "Normal not unit: len={len}");
        }
    }

    #[test]
    fn plane_geometry() {
        let (v, i) = plane(4);
        assert_eq!(v.len(), 25); // 5*5
        assert_eq!(i.len(), 96); // 4*4*6
    }

    #[test]
    fn plane_flat_normals() {
        let (v, _) = plane(2);
        for vert in &v {
            assert_eq!(vert.normal, [0.0, 1.0, 0.0]);
        }
    }

    #[test]
    fn cylinder_geometry() {
        let (v, i) = cylinder(16);
        assert!(!v.is_empty());
        assert!(!i.is_empty());
        for &idx in &i {
            assert!(idx < v.len() as u32);
        }
    }

    #[test]
    fn sphere_minimum_segments() {
        let (v, i) = sphere(1, 1); // should clamp to 3, 2
        assert!(!v.is_empty());
        assert!(!i.is_empty());
    }

    #[test]
    fn sphere_zero_segments() {
        // segments=0, rings=0 — should clamp to minimums and produce valid data
        let (v, i) = sphere(0, 0);
        assert!(!v.is_empty(), "sphere(0,0) should produce vertices");
        for &idx in &i {
            assert!(idx < v.len() as u32, "index out of bounds in sphere(0,0)");
        }
    }

    #[test]
    fn plane_zero_subdivisions() {
        // subdivisions=0 — should clamp to 1 and produce valid data
        let (v, i) = plane(0);
        assert!(!v.is_empty(), "plane(0) should produce vertices");
        assert!(!i.is_empty(), "plane(0) should produce indices");
        for &idx in &i {
            assert!(idx < v.len() as u32, "index out of bounds in plane(0)");
        }
    }

    #[test]
    fn cylinder_zero_segments() {
        // segments=0 — should clamp to 3 and produce valid data
        let (v, i) = cylinder(0);
        assert!(!v.is_empty(), "cylinder(0) should produce vertices");
        assert!(!i.is_empty(), "cylinder(0) should produce indices");
        for &idx in &i {
            assert!(idx < v.len() as u32, "index out of bounds in cylinder(0)");
        }
    }
}
