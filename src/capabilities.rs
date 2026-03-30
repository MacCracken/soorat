//! GPU capability reporting — feature detection and limit queries.
//!
//! Re-exported from [`mabda`] — the shared GPU foundation.
//! Soorat adds [`SooratCapabilities`] for renderer-specific requirement checks.

pub use mabda::capabilities::{GpuCapabilities, webgpu};

/// Soorat-specific capability extensions.
pub trait SooratCapabilities {
    /// Check if the device meets minimum requirements for soorat rendering.
    fn meets_soorat_requirements(&self) -> bool;
}

impl SooratCapabilities for GpuCapabilities {
    fn meets_soorat_requirements(&self) -> bool {
        self.max_texture_dimension_2d >= 2048
            && self.max_uniform_buffer_size >= 16384
            && self.max_bind_groups >= 4
    }
}

/// Soorat-specific WebGPU compatibility checks.
pub mod soorat_webgpu {
    /// Check if soorat's current uniform sizes are WebGPU-compatible.
    pub fn check_soorat_uniforms() -> Vec<(&'static str, u32, bool)> {
        let checks = vec![
            (
                "CameraUniforms",
                std::mem::size_of::<crate::mesh_pipeline::CameraUniforms>() as u32,
            ),
            (
                "LightArrayUniforms",
                std::mem::size_of::<crate::lights::LightArrayUniforms>() as u32,
            ),
            (
                "MaterialUniforms",
                std::mem::size_of::<crate::pbr_material::MaterialUniforms>() as u32,
            ),
            (
                "ShadowPassUniforms",
                std::mem::size_of::<crate::mesh_pipeline::ShadowPassUniforms>() as u32,
            ),
            (
                "CascadeUniforms",
                std::mem::size_of::<crate::shadow::CascadeUniforms>() as u32,
            ),
            (
                "PostProcessUniforms",
                std::mem::size_of::<crate::postprocess::PostProcessUniforms>() as u32,
            ),
            (
                "BloomUniforms",
                std::mem::size_of::<crate::hdr::BloomUniforms>() as u32,
            ),
            (
                "SsaoUniforms",
                std::mem::size_of::<crate::ssao::SsaoUniforms>() as u32,
            ),
        ];

        checks
            .into_iter()
            .map(|(name, size)| (name, size, mabda::capabilities::webgpu::uniform_fits(size)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webgpu_uniform_fits() {
        assert!(webgpu::uniform_fits(1024));
        assert!(webgpu::uniform_fits(65536));
        assert!(!webgpu::uniform_fits(65537));
    }

    #[test]
    fn webgpu_constants() {
        assert_eq!(webgpu::MAX_BIND_GROUPS, 4);
        assert_eq!(webgpu::MAX_UNIFORM_BUFFER, 65536);
    }

    #[test]
    fn soorat_uniforms_webgpu_compatible() {
        let checks = soorat_webgpu::check_soorat_uniforms();
        for (name, size, fits) in &checks {
            assert!(*fits, "{name} ({size} bytes) exceeds WebGPU uniform limit");
        }
    }

    #[test]
    fn gpu_capabilities_types() {
        let _size = std::mem::size_of::<GpuCapabilities>();
    }
}
