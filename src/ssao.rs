//! Screen-Space Ambient Occlusion (SSAO).

/// SSAO uniforms.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SsaoUniforms {
    /// x = radius, y = bias, z = intensity, w = sample_count
    pub params: [f32; 4],
    /// Camera projection matrix (for reprojection).
    pub projection: [f32; 16],
    /// Inverse projection matrix (for depth → view-space).
    pub inv_projection: [f32; 16],
}

impl Default for SsaoUniforms {
    fn default() -> Self {
        Self {
            params: [0.5, 0.025, 1.0, 16.0],
            projection: crate::math_util::IDENTITY_MAT4,
            inv_projection: crate::math_util::IDENTITY_MAT4,
        }
    }
}

impl SsaoUniforms {
    pub fn new(radius: f32, bias: f32, intensity: f32, sample_count: u32) -> Self {
        Self {
            params: [radius, bias, intensity, sample_count as f32],
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssao_uniforms_size() {
        // vec4 + 2 * mat4 = 16 + 128 = 144
        assert_eq!(std::mem::size_of::<SsaoUniforms>(), 144);
    }

    #[test]
    fn ssao_uniforms_default() {
        let u = SsaoUniforms::default();
        assert_eq!(u.params[0], 0.5); // radius
        assert_eq!(u.params[3], 16.0); // sample_count
    }

    #[test]
    fn ssao_uniforms_new() {
        let u = SsaoUniforms::new(1.0, 0.01, 2.0, 32);
        assert_eq!(u.params[0], 1.0);
        assert_eq!(u.params[3], 32.0);
    }

    #[test]
    fn ssao_uniforms_bytemuck() {
        let u = SsaoUniforms::default();
        let bytes = bytemuck::bytes_of(&u);
        assert_eq!(bytes.len(), 144);
    }
}
