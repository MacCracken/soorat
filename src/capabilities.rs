//! GPU capability reporting — feature detection and limit queries.

/// GPU capabilities report — what the current device supports.
#[derive(Debug, Clone)]
pub struct GpuCapabilities {
    /// Adapter name (e.g., "NVIDIA GeForce RTX 4090").
    pub adapter_name: String,
    /// Backend (Vulkan, Metal, DX12, etc.).
    pub backend: String,
    /// Whether timestamp queries are supported (for GPU profiling).
    pub timestamp_query: bool,
    /// Whether compute shaders are supported.
    pub compute_shaders: bool,
    /// Maximum texture dimension (1D/2D).
    pub max_texture_dimension_2d: u32,
    /// Maximum uniform buffer binding size.
    pub max_uniform_buffer_size: u32,
    /// Maximum storage buffer binding size.
    pub max_storage_buffer_size: u32,
    /// Maximum number of bind groups.
    pub max_bind_groups: u32,
    /// Maximum vertex buffers per pipeline.
    pub max_vertex_buffers: u32,
    /// Maximum push constant size.
    pub max_push_constant_size: u32,
    /// Whether the device supports multi-draw indirect.
    pub multi_draw_indirect: bool,
}

impl GpuCapabilities {
    /// Query capabilities from a GpuContext.
    pub fn from_context(ctx: &crate::gpu::GpuContext) -> Self {
        let info = ctx.adapter.get_info();
        let features = ctx.device.features();
        let limits = ctx.device.limits();

        Self {
            adapter_name: info.name.clone(),
            backend: match info.backend {
                wgpu::Backend::Vulkan => "Vulkan",
                wgpu::Backend::Metal => "Metal",
                wgpu::Backend::Dx12 => "DX12",
                wgpu::Backend::Gl => "GL",
                wgpu::Backend::BrowserWebGpu => "WebGPU",
                _ => "Unknown",
            }
            .to_string(),
            timestamp_query: features.contains(wgpu::Features::TIMESTAMP_QUERY),
            compute_shaders: true, // wgpu always supports compute
            max_texture_dimension_2d: limits.max_texture_dimension_2d,
            max_uniform_buffer_size: limits.max_uniform_buffer_binding_size,
            max_storage_buffer_size: limits.max_storage_buffer_binding_size,
            max_bind_groups: limits.max_bind_groups,
            max_vertex_buffers: limits.max_vertex_buffers,
            max_push_constant_size: limits.max_push_constant_size,
            multi_draw_indirect: features.contains(wgpu::Features::MULTI_DRAW_INDIRECT),
        }
    }

    /// Check if the device meets minimum requirements for soorat.
    pub fn meets_requirements(&self) -> bool {
        self.max_texture_dimension_2d >= 2048
            && self.max_uniform_buffer_size >= 16384
            && self.max_bind_groups >= 4
    }

    /// Format as a human-readable report.
    pub fn report(&self) -> String {
        format!(
            "GPU: {} ({})\n\
             Timestamp queries: {}\n\
             Max texture 2D: {}\n\
             Max uniform buffer: {} bytes\n\
             Max storage buffer: {} bytes\n\
             Max bind groups: {}\n\
             Max vertex buffers: {}\n\
             Multi-draw indirect: {}\n\
             Meets soorat requirements: {}",
            self.adapter_name,
            self.backend,
            self.timestamp_query,
            self.max_texture_dimension_2d,
            self.max_uniform_buffer_size,
            self.max_storage_buffer_size,
            self.max_bind_groups,
            self.max_vertex_buffers,
            self.multi_draw_indirect,
            self.meets_requirements(),
        )
    }
}

/// WebGPU compatibility notes.
/// These are compile-time hints, not runtime checks.
pub mod webgpu {
    /// WebGPU maximum uniform buffer size (64KB).
    pub const MAX_UNIFORM_BUFFER: u32 = 65536;

    /// WebGPU maximum storage buffer size (128MB typical).
    pub const MAX_STORAGE_BUFFER: u32 = 134_217_728;

    /// WebGPU maximum bind groups (4).
    pub const MAX_BIND_GROUPS: u32 = 4;

    /// WebGPU maximum texture dimension (8192 typical).
    pub const MAX_TEXTURE_2D: u32 = 8192;

    /// Check if a uniform buffer size is WebGPU-compatible.
    pub fn uniform_fits(size: u32) -> bool {
        size <= MAX_UNIFORM_BUFFER
    }

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
            .map(|(name, size)| (name, size, uniform_fits(size)))
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
        let checks = webgpu::check_soorat_uniforms();
        for (name, size, fits) in &checks {
            assert!(*fits, "{name} ({size} bytes) exceeds WebGPU uniform limit");
        }
    }

    #[test]
    fn gpu_capabilities_types() {
        let _size = std::mem::size_of::<GpuCapabilities>();
    }
}
