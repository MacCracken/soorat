//! HDR framebuffer and post-processing chain.

/// HDR render target — Rgba16Float for linear HDR rendering.
/// Scene renders to this, then post-processing reads from it.
pub struct HdrFramebuffer {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
}

impl HdrFramebuffer {
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hdr_framebuffer"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("hdr_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            width,
            height,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        *self = Self::new(device, width, height);
    }
}

/// Bloom uniforms for the bloom shader passes.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BloomUniforms {
    /// x = threshold, y = soft_threshold, z = intensity, w = unused
    pub params: [f32; 4],
    /// x = texel_width, y = texel_height
    pub texel_size: [f32; 4],
}

impl BloomUniforms {
    pub fn new(
        threshold: f32,
        soft_threshold: f32,
        intensity: f32,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            params: [threshold, soft_threshold, intensity, 0.0],
            texel_size: [1.0 / width as f32, 1.0 / height as f32, 0.0, 0.0],
        }
    }
}

impl Default for BloomUniforms {
    fn default() -> Self {
        Self::new(1.0, 0.5, 0.3, 1920, 1080)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdr_format() {
        assert_eq!(HdrFramebuffer::FORMAT, wgpu::TextureFormat::Rgba16Float);
    }

    #[test]
    fn bloom_uniforms_size() {
        assert_eq!(std::mem::size_of::<BloomUniforms>(), 32);
    }

    #[test]
    fn bloom_uniforms_default() {
        let u = BloomUniforms::default();
        assert_eq!(u.params[0], 1.0); // threshold
        assert_eq!(u.params[2], 0.3); // intensity
    }

    #[test]
    fn bloom_uniforms_texel_size() {
        let u = BloomUniforms::new(1.0, 0.5, 0.3, 1920, 1080);
        assert!((u.texel_size[0] - 1.0 / 1920.0).abs() < 0.0001);
        assert!((u.texel_size[1] - 1.0 / 1080.0).abs() < 0.0001);
    }

    #[test]
    fn bloom_uniforms_bytemuck() {
        let u = BloomUniforms::default();
        let bytes = bytemuck::bytes_of(&u);
        assert_eq!(bytes.len(), 32);
    }
}
