//! Terrain rendering — heightmap-based mesh generation.

use crate::mesh_pipeline::Mesh;
use crate::vertex::Vertex3D;

/// Terrain configuration.
#[derive(Debug, Clone)]
pub struct TerrainConfig {
    /// Number of grid cells along X.
    pub width: u32,
    /// Number of grid cells along Z.
    pub depth: u32,
    /// World-space size of the terrain along X.
    pub world_width: f32,
    /// World-space size of the terrain along Z.
    pub world_depth: f32,
    /// Vertical scale applied to height values.
    pub height_scale: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            width: 64,
            depth: 64,
            world_width: 64.0,
            world_depth: 64.0,
            height_scale: 10.0,
        }
    }
}

/// Generated terrain mesh data (CPU-side, before GPU upload).
pub struct TerrainData {
    pub vertices: Vec<Vertex3D>,
    pub indices: Vec<u32>,
}

/// Generate terrain mesh data from a heightmap.
/// `heights`: row-major grid of height values, size = (config.width+1) * (config.depth+1).
/// Returns vertices and indices suitable for MeshPipeline.
pub fn generate_terrain(config: &TerrainConfig, heights: &[f32]) -> TerrainData {
    let cols = config.width + 1;
    let rows = config.depth + 1;
    let expected = (cols * rows) as usize;

    let cell_w = config.world_width / config.width as f32;
    let cell_d = config.world_depth / config.depth as f32;

    let mut vertices = Vec::with_capacity(expected);

    // Generate vertices
    for z in 0..rows {
        for x in 0..cols {
            let idx = (z * cols + x) as usize;
            let h = if idx < heights.len() {
                heights[idx] * config.height_scale
            } else {
                0.0
            };

            let pos = [
                x as f32 * cell_w - config.world_width * 0.5,
                h,
                z as f32 * cell_d - config.world_depth * 0.5,
            ];

            let u = x as f32 / config.width as f32;
            let v = z as f32 / config.depth as f32;

            // Compute normal from neighboring heights
            let normal = compute_normal(
                heights,
                x,
                z,
                cols,
                rows,
                cell_w,
                cell_d,
                config.height_scale,
            );

            vertices.push(Vertex3D {
                position: pos,
                normal,
                tex_coords: [u, v],
                color: [1.0, 1.0, 1.0, 1.0],
            });
        }
    }

    // Generate indices (two triangles per grid cell)
    let quad_count = (config.width * config.depth) as usize;
    let mut indices = Vec::with_capacity(quad_count * 6);

    for z in 0..config.depth {
        for x in 0..config.width {
            let tl = z * cols + x;
            let tr = tl + 1;
            let bl = tl + cols;
            let br = bl + 1;

            // Triangle 1
            indices.push(tl);
            indices.push(bl);
            indices.push(tr);
            // Triangle 2
            indices.push(tr);
            indices.push(bl);
            indices.push(br);
        }
    }

    TerrainData { vertices, indices }
}

/// Upload terrain data to GPU as a Mesh.
pub fn create_terrain_mesh(device: &wgpu::Device, config: &TerrainConfig, heights: &[f32]) -> Mesh {
    let data = generate_terrain(config, heights);
    Mesh::new(device, &data.vertices, &data.indices)
}

/// Generate a flat heightmap (all zeros).
pub fn flat_heightmap(width: u32, depth: u32) -> Vec<f32> {
    vec![0.0; ((width + 1) * (depth + 1)) as usize]
}

#[allow(clippy::too_many_arguments)]
fn compute_normal(
    heights: &[f32],
    x: u32,
    z: u32,
    cols: u32,
    rows: u32,
    cell_w: f32,
    cell_d: f32,
    height_scale: f32,
) -> [f32; 3] {
    let h = |ix: u32, iz: u32| -> f32 {
        let idx = (iz * cols + ix) as usize;
        if idx < heights.len() {
            heights[idx] * height_scale
        } else {
            0.0
        }
    };

    let hc = h(x, z);
    let hl = if x > 0 { h(x - 1, z) } else { hc };
    let hr = if x < cols - 1 { h(x + 1, z) } else { hc };
    let hd = if z > 0 { h(x, z - 1) } else { hc };
    let hu = if z < rows - 1 { h(x, z + 1) } else { hc };

    let dx = (hl - hr) / (2.0 * cell_w);
    let dz = (hd - hu) / (2.0 * cell_d);

    // Normal = normalize(dx, 1, dz)
    let len = (dx * dx + 1.0 + dz * dz).sqrt();
    [dx / len, 1.0 / len, dz / len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_config_default() {
        let cfg = TerrainConfig::default();
        assert_eq!(cfg.width, 64);
        assert_eq!(cfg.depth, 64);
    }

    #[test]
    fn flat_terrain_generates_correct_counts() {
        let cfg = TerrainConfig {
            width: 4,
            depth: 4,
            ..Default::default()
        };
        let heights = flat_heightmap(4, 4);
        let data = generate_terrain(&cfg, &heights);
        assert_eq!(data.vertices.len(), 5 * 5); // (4+1)*(4+1) = 25
        assert_eq!(data.indices.len(), 4 * 4 * 6); // 16 quads * 6 indices
    }

    #[test]
    fn flat_terrain_normals_point_up() {
        let cfg = TerrainConfig {
            width: 2,
            depth: 2,
            ..Default::default()
        };
        let heights = flat_heightmap(2, 2);
        let data = generate_terrain(&cfg, &heights);
        for v in &data.vertices {
            assert!((v.normal[1] - 1.0).abs() < 0.01, "Normal should be up");
        }
    }

    #[test]
    fn flat_terrain_height_zero() {
        let cfg = TerrainConfig {
            width: 2,
            depth: 2,
            ..Default::default()
        };
        let heights = flat_heightmap(2, 2);
        let data = generate_terrain(&cfg, &heights);
        for v in &data.vertices {
            assert_eq!(v.position[1], 0.0);
        }
    }

    #[test]
    fn terrain_uv_corners() {
        let cfg = TerrainConfig {
            width: 1,
            depth: 1,
            ..Default::default()
        };
        let heights = flat_heightmap(1, 1);
        let data = generate_terrain(&cfg, &heights);
        assert_eq!(data.vertices[0].tex_coords, [0.0, 0.0]);
        assert_eq!(data.vertices[1].tex_coords, [1.0, 0.0]);
        assert_eq!(data.vertices[2].tex_coords, [0.0, 1.0]);
        assert_eq!(data.vertices[3].tex_coords, [1.0, 1.0]);
    }

    #[test]
    fn terrain_centered_at_origin() {
        let cfg = TerrainConfig {
            width: 2,
            depth: 2,
            world_width: 10.0,
            world_depth: 10.0,
            ..Default::default()
        };
        let heights = flat_heightmap(2, 2);
        let data = generate_terrain(&cfg, &heights);
        // First vertex should be at (-5, 0, -5)
        assert_eq!(data.vertices[0].position[0], -5.0);
        assert_eq!(data.vertices[0].position[2], -5.0);
    }

    #[test]
    fn terrain_height_scale() {
        let cfg = TerrainConfig {
            width: 1,
            depth: 1,
            height_scale: 5.0,
            ..Default::default()
        };
        let heights = vec![1.0; 4];
        let data = generate_terrain(&cfg, &heights);
        assert_eq!(data.vertices[0].position[1], 5.0);
    }

    #[test]
    fn flat_heightmap_size() {
        let h = flat_heightmap(10, 20);
        assert_eq!(h.len(), 11 * 21);
    }

    #[test]
    fn terrain_indices_valid() {
        let cfg = TerrainConfig {
            width: 3,
            depth: 3,
            ..Default::default()
        };
        let heights = flat_heightmap(3, 3);
        let data = generate_terrain(&cfg, &heights);
        let max_vertex = data.vertices.len() as u32;
        for &idx in &data.indices {
            assert!(idx < max_vertex);
        }
    }
}
