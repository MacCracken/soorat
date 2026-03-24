//! Texture loading and management.

use crate::error::{RenderError, Result};
use std::collections::HashMap;

/// A GPU texture with view and sampler, ready for binding.
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    /// Load a texture from PNG/JPEG bytes.
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)
            .map_err(|e| RenderError::Texture(e.to_string()))?
            .to_rgba8();

        let (width, height) = img.dimensions();

        Self::from_rgba(device, queue, &img, width, height, label)
    }

    /// Create a 1x1 solid color texture.
    pub fn from_color(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color: crate::color::Color,
    ) -> Self {
        let rgba = [
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        ];

        Self::from_rgba(device, queue, &rgba, 1, 1, "solid_color")
            .expect("1x1 texture creation should not fail")
    }

    /// Create a 1x1 white pixel texture (default texture for untextured sprites).
    pub fn white_pixel(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self::from_color(device, queue, crate::color::Color::WHITE)
    }

    /// Create a texture from raw RGBA8 pixel data.
    pub fn from_rgba(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba: &[u8],
        width: u32,
        height: u32,
        label: &str,
    ) -> Result<Self> {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    /// Create a wgpu bind group for this texture, compatible with SpritePipeline.
    pub fn bind_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Create a texture from a ranga PixelBuffer.
    /// Only Rgba8 format is supported; convert other formats via ranga before calling.
    #[cfg(feature = "ranga")]
    pub fn from_pixel_buffer(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffer: &ranga::pixel::PixelBuffer,
        label: &str,
    ) -> Result<Self> {
        let rgba_data = match buffer.format {
            ranga::pixel::PixelFormat::Rgba8 => &buffer.data,
            _ => {
                return Err(RenderError::Texture(format!(
                    "Unsupported pixel format {:?}, expected Rgba8",
                    buffer.format
                )));
            }
        };
        Self::from_rgba(device, queue, rgba_data, buffer.width, buffer.height, label)
    }

    /// Texture dimensions.
    pub fn size(&self) -> (u32, u32) {
        let s = self.texture.size();
        (s.width, s.height)
    }
}

/// A cached texture entry: texture + its bind group.
struct CachedTexture {
    texture: Texture,
    bind_group: wgpu::BindGroup,
}

/// Cache of loaded textures, keyed by texture ID.
pub struct TextureCache {
    entries: HashMap<u64, CachedTexture>,
}

impl TextureCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Insert a texture, creating its bind group.
    pub fn insert(
        &mut self,
        id: u64,
        texture: Texture,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) {
        let bind_group = texture.bind_group(device, layout);
        self.entries.insert(
            id,
            CachedTexture {
                texture,
                bind_group,
            },
        );
    }

    /// Get the bind group for a texture ID.
    pub fn get_bind_group(&self, id: u64) -> Option<&wgpu::BindGroup> {
        self.entries.get(&id).map(|e| &e.bind_group)
    }

    /// Check if a texture ID exists.
    pub fn contains(&self, id: u64) -> bool {
        self.entries.contains_key(&id)
    }

    /// Get a texture by ID.
    pub fn get(&self, id: u64) -> Option<&Texture> {
        self.entries.get(&id).map(|e| &e.texture)
    }

    /// Number of cached textures.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for TextureCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_cache_empty() {
        let cache = TextureCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert!(!cache.contains(0));
        assert!(cache.get(0).is_none());
        assert!(cache.get_bind_group(0).is_none());
    }

    #[test]
    fn texture_cache_default() {
        let cache = TextureCache::default();
        assert!(cache.is_empty());
    }
}
