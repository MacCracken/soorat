//! Level of Detail — distance-based mesh/terrain selection.

/// A single LOD level with a mesh index and maximum distance.
#[derive(Debug, Clone, Copy)]
pub struct LodLevel {
    /// Index into an external mesh array.
    pub mesh_index: usize,
    /// Maximum distance from camera for this LOD (squared, for fast comparison).
    pub max_distance_sq: f32,
}

/// LOD chain — ordered list of detail levels from highest to lowest.
pub struct LodChain {
    pub levels: Vec<LodLevel>,
}

impl LodChain {
    /// Create a LOD chain from distances (sorted nearest → farthest).
    /// Each distance is the max range for that LOD level.
    #[must_use]
    pub fn new(distances: &[f32]) -> Self {
        let levels = distances
            .iter()
            .enumerate()
            .map(|(i, &d)| LodLevel {
                mesh_index: i,
                max_distance_sq: d * d,
            })
            .collect();
        Self { levels }
    }

    /// Select the appropriate LOD level for a given squared distance from camera.
    /// Returns the mesh_index for the best LOD, or the last (lowest detail) if beyond all ranges.
    #[must_use]
    #[inline]
    pub fn select(&self, distance_sq: f32) -> usize {
        for level in &self.levels {
            if distance_sq <= level.max_distance_sq {
                return level.mesh_index;
            }
        }
        // Beyond all LODs — use lowest detail
        self.levels.last().map(|l| l.mesh_index).unwrap_or(0)
    }

    /// Select LOD from camera and object positions (computes distance internally).
    #[must_use]
    #[inline]
    pub fn select_from_positions(&self, camera_pos: [f32; 3], object_pos: [f32; 3]) -> usize {
        let dx = camera_pos[0] - object_pos[0];
        let dy = camera_pos[1] - object_pos[1];
        let dz = camera_pos[2] - object_pos[2];
        self.select(dx * dx + dy * dy + dz * dz)
    }

    /// Number of LOD levels.
    #[must_use]
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }
}

/// Terrain LOD — selects grid resolution based on distance from camera.
pub struct TerrainLod {
    /// Grid resolutions from highest to lowest (e.g., [64, 32, 16, 8]).
    pub resolutions: Vec<u32>,
    /// Distance thresholds matching resolutions.
    pub distances: Vec<f32>,
}

impl TerrainLod {
    pub fn new(resolutions: Vec<u32>, distances: Vec<f32>) -> crate::error::Result<Self> {
        if resolutions.len() != distances.len() {
            return Err(crate::error::RenderError::Pipeline(format!(
                "TerrainLod: resolutions length ({}) != distances length ({})",
                resolutions.len(),
                distances.len(),
            )));
        }
        Ok(Self {
            resolutions,
            distances,
        })
    }

    /// Select terrain resolution for a given distance from camera.
    #[must_use]
    #[inline]
    pub fn select_resolution(&self, distance: f32) -> u32 {
        for (i, &d) in self.distances.iter().enumerate() {
            if distance <= d {
                return self.resolutions[i];
            }
        }
        *self.resolutions.last().unwrap_or(&8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lod_chain_select() {
        let chain = LodChain::new(&[10.0, 30.0, 100.0]);
        assert_eq!(chain.select(5.0 * 5.0), 0); // close = high detail
        assert_eq!(chain.select(20.0 * 20.0), 1); // mid
        assert_eq!(chain.select(50.0 * 50.0), 2); // far = low detail
    }

    #[test]
    fn lod_chain_beyond_range() {
        let chain = LodChain::new(&[10.0, 30.0]);
        assert_eq!(chain.select(999.0 * 999.0), 1); // beyond all = last
    }

    #[test]
    fn lod_chain_from_positions() {
        let chain = LodChain::new(&[10.0, 50.0, 200.0]);
        assert_eq!(
            chain.select_from_positions([0.0, 0.0, 0.0], [5.0, 0.0, 0.0]),
            0
        );
        assert_eq!(
            chain.select_from_positions([0.0, 0.0, 0.0], [100.0, 0.0, 0.0]),
            2
        );
    }

    #[test]
    fn lod_chain_single_level() {
        let chain = LodChain::new(&[100.0]);
        assert_eq!(chain.select(0.0), 0);
        assert_eq!(chain.level_count(), 1);
    }

    #[test]
    fn terrain_lod_select() {
        let lod = TerrainLod::new(vec![64, 32, 16, 8], vec![50.0, 100.0, 200.0, 500.0]).unwrap();
        assert_eq!(lod.select_resolution(30.0), 64);
        assert_eq!(lod.select_resolution(75.0), 32);
        assert_eq!(lod.select_resolution(150.0), 16);
        assert_eq!(lod.select_resolution(300.0), 8);
    }

    #[test]
    fn terrain_lod_beyond() {
        let lod = TerrainLod::new(vec![64, 8], vec![50.0, 200.0]).unwrap();
        assert_eq!(lod.select_resolution(999.0), 8);
    }

    #[test]
    fn terrain_lod_mismatched_lengths() {
        // Adversarial: different-length vecs must return Err, not panic
        let result = TerrainLod::new(vec![64, 32, 16], vec![50.0, 100.0]);
        assert!(result.is_err());
        let result2 = TerrainLod::new(vec![64], vec![50.0, 100.0, 200.0]);
        assert!(result2.is_err());
        // Equal lengths should succeed
        let result3 = TerrainLod::new(vec![64, 32], vec![50.0, 100.0]);
        assert!(result3.is_ok());
    }
}
