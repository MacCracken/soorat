//! Terrain rendering — heightmap-based mesh generation.

use crate::mesh_pipeline::Mesh;
use crate::vertex::Vertex3D;
use crate::viz::{HeightField, heightfield_mesh};

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
/// Missing (short `heights`) or non-finite samples are treated as 0.
/// Returns vertices and indices suitable for MeshPipeline (triangles face +Y).
///
/// Returns empty data if `width`/`depth` is 0 or the vertex count would not fit
/// `u32` indices.
#[must_use]
pub fn generate_terrain(config: &TerrainConfig, heights: &[f32]) -> TerrainData {
    let empty = || TerrainData {
        vertices: Vec::new(),
        indices: Vec::new(),
    };
    if config.width == 0 || config.depth == 0 {
        tracing::warn!(
            width = config.width,
            depth = config.depth,
            "generate_terrain: width or depth is 0 — returning empty terrain"
        );
        return empty();
    }

    let cols = config.width as usize + 1;
    let rows = config.depth as usize + 1;
    let Some(expected) = cols.checked_mul(rows) else {
        tracing::warn!(cols, rows, "generate_terrain: vertex count overflows");
        return empty();
    };
    if heights.len() < expected {
        tracing::debug!(
            expected,
            actual = heights.len(),
            "generate_terrain: heightmap shorter than grid — missing samples are 0"
        );
    }

    let field = HeightField {
        cols,
        rows,
        origin_x: -config.world_width * 0.5,
        origin_z: -config.world_depth * 0.5,
        dx: config.world_width / config.width as f32,
        dz: config.world_depth / config.depth as f32,
    };
    let scale = config.height_scale;
    let (vertices, indices) = heightfield_mesh(
        &field,
        |ix, iz| heights.get(iz * cols + ix).map_or(0.0, |&h| h * scale),
        |_, _, _| [1.0, 1.0, 1.0, 1.0],
    );

    TerrainData { vertices, indices }
}

/// Upload terrain data to GPU as a Mesh.
#[must_use]
pub fn create_terrain_mesh(device: &wgpu::Device, config: &TerrainConfig, heights: &[f32]) -> Mesh {
    let data = generate_terrain(config, heights);
    Mesh::new(device, &data.vertices, &data.indices)
}

/// Generate a flat heightmap (all zeros) sized `(width+1) * (depth+1)`.
/// Returns an empty vector if that size overflows `usize`.
#[must_use]
pub fn flat_heightmap(width: u32, depth: u32) -> Vec<f32> {
    let len = (width as usize + 1).checked_mul(depth as usize + 1);
    vec![0.0; len.unwrap_or(0)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_winds_upward() {
        let cfg = TerrainConfig {
            width: 4,
            depth: 3,
            ..Default::default()
        };
        let heights: Vec<f32> = (0..20).map(|i| (i as f32 * 0.37).sin()).collect();
        let data = generate_terrain(&cfg, &heights);
        crate::viz::assert_winding_matches_normals(&data.vertices, &data.indices);
    }

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

    #[test]
    fn generate_terrain_zero_size() {
        // Division-by-zero regression: zero width/depth must not panic
        let cfg = TerrainConfig {
            width: 0,
            depth: 0,
            ..Default::default()
        };
        let data = generate_terrain(&cfg, &[]);
        assert!(data.vertices.is_empty());
        assert!(data.indices.is_empty());

        // Also test one dimension zero
        let cfg_w0 = TerrainConfig {
            width: 0,
            depth: 4,
            ..Default::default()
        };
        let data_w0 = generate_terrain(&cfg_w0, &[]);
        assert!(data_w0.vertices.is_empty());
        assert!(data_w0.indices.is_empty());
    }
}
