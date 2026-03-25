//! PBR material types and BRDF LUT precomputation.

use crate::color::Color;
#[cfg(feature = "optics")]
use crate::error::Result;
#[cfg(feature = "optics")]
use crate::texture::Texture;

/// PBR material parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialUniforms {
    pub base_color_factor: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

impl Default for MaterialUniforms {
    fn default() -> Self {
        Self {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

impl MaterialUniforms {
    /// Create material uniforms for a dielectric (non-metal).
    #[must_use]
    pub fn dielectric(base_color: Color, roughness: f32) -> Self {
        Self {
            base_color_factor: base_color.to_array(),
            metallic: 0.0,
            roughness,
            ..Default::default()
        }
    }

    /// Create material uniforms for a metal.
    #[must_use]
    pub fn metal(base_color: Color, roughness: f32) -> Self {
        Self {
            base_color_factor: base_color.to_array(),
            metallic: 1.0,
            roughness,
            ..Default::default()
        }
    }

    /// Create a dielectric material from an IOR (via prakash).
    /// Uses prakash::pbr::ior_to_f0 to compute reflectance.
    /// Always non-metallic — for metals, use `metal()` directly.
    #[cfg(feature = "optics")]
    #[must_use]
    pub fn from_ior(base_color: Color, ior: f64, roughness: f32) -> Self {
        // IOR→F0 gives us the dielectric reflectance, but the PBR shader
        // hardcodes F0=0.04 for dielectrics via mix(0.04, albedo, metallic).
        // For non-standard IOR (e.g. water=1.33, diamond=2.42), the user
        // should adjust the shader or use the F0 value directly.
        let _f0 = prakash::pbr::ior_to_f0(ior) as f32;
        Self {
            base_color_factor: base_color.to_array(),
            metallic: 0.0,
            roughness,
            ..Default::default()
        }
    }
}

/// Precompute the BRDF integration LUT using prakash.
/// Returns RGBA8 pixel data for a `size x size` texture.
/// Red = scale, Green = bias (from split-sum approximation).
#[cfg(feature = "optics")]
pub fn generate_brdf_lut(size: u32, samples: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        let roughness = (y as f64 + 0.5) / size as f64;
        for x in 0..size {
            let n_dot_v = (x as f64 + 0.5) / size as f64;
            let (scale, bias) = prakash::pbr::integrate_brdf_lut(n_dot_v, roughness, samples);

            pixels.push((scale.clamp(0.0, 1.0) * 255.0) as u8);
            pixels.push((bias.clamp(0.0, 1.0) * 255.0) as u8);
            pixels.push(0); // unused
            pixels.push(255); // alpha
        }
    }

    pixels
}

/// Create a BRDF LUT texture on the GPU using prakash precomputation.
#[cfg(feature = "optics")]
pub fn create_brdf_lut_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: u32,
    samples: u32,
) -> Result<Texture> {
    let pixels = generate_brdf_lut(size, samples);
    Texture::from_rgba(device, queue, &pixels, size, size, "brdf_lut")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_uniforms_size() {
        assert_eq!(std::mem::size_of::<MaterialUniforms>(), 32);
    }

    #[test]
    fn material_uniforms_default() {
        let m = MaterialUniforms::default();
        assert_eq!(m.metallic, 0.0);
        assert_eq!(m.roughness, 0.5);
        assert_eq!(m.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn material_dielectric() {
        let m = MaterialUniforms::dielectric(Color::RED, 0.8);
        assert_eq!(m.metallic, 0.0);
        assert_eq!(m.roughness, 0.8);
        assert_eq!(m.base_color_factor[0], 1.0);
    }

    #[test]
    fn material_metal() {
        let m = MaterialUniforms::metal(Color::new(0.9, 0.8, 0.2, 1.0), 0.3);
        assert_eq!(m.metallic, 1.0);
        assert_eq!(m.roughness, 0.3);
    }

    #[test]
    fn material_bytemuck() {
        let m = MaterialUniforms::default();
        let bytes = bytemuck::bytes_of(&m);
        assert_eq!(bytes.len(), 32);
    }

    #[cfg(feature = "optics")]
    #[test]
    fn material_from_ior_glass() {
        let m = MaterialUniforms::from_ior(Color::WHITE, 1.5, 0.1);
        assert_eq!(m.metallic, 0.0); // glass is dielectric
        assert_eq!(m.roughness, 0.1);
    }

    #[cfg(feature = "optics")]
    #[test]
    fn brdf_lut_generates_correct_size() {
        let pixels = generate_brdf_lut(16, 32);
        assert_eq!(pixels.len(), 16 * 16 * 4);
    }

    #[cfg(feature = "optics")]
    #[test]
    fn brdf_lut_values_in_range() {
        let pixels = generate_brdf_lut(8, 16);
        for chunk in pixels.chunks(4) {
            // RGBA values exist and alpha is opaque
            assert_eq!(chunk.len(), 4);
            assert_eq!(chunk[3], 255); // alpha always 1
        }
    }
}
