//! Texture loading and management.
//!
//! Core [`Texture`] struct lives in soorat because its construction methods
//! reference [`crate::color::Color`] (soorat's own color type). Standalone
//! utilities (`mip_level_count`, `validate_dimensions`, `copy_texture_to_texture`)
//! and [`CubemapTexture`] are re-exported from [`mabda::texture`].

use crate::error::{RenderError, Result};
use std::collections::HashMap;
use std::collections::hash_map::Entry;

// ── Re-exports from mabda ──────────────────────────────────────────────────
pub use mabda::texture::{
    CubemapTexture, copy_texture_to_texture, create_default_sampler, mip_level_count,
    validate_dimensions,
};

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
        tracing::debug!(label, "loading texture from bytes");
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
    ) -> Result<Self> {
        let rgba = [
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        ];

        Self::from_rgba(device, queue, &rgba, 1, 1, "solid_color")
    }

    /// Create a 1x1 white pixel texture (default texture for untextured sprites).
    pub fn white_pixel(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self> {
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
        let sampler = create_default_sampler(device);
        Self::from_rgba_with_sampler(device, queue, rgba, width, height, label, sampler)
    }

    /// Create a texture from raw RGBA8 pixel data with an externally-provided sampler.
    pub fn from_rgba_with_sampler(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rgba: &[u8],
        width: u32,
        height: u32,
        label: &str,
        sampler: wgpu::Sampler,
    ) -> Result<Self> {
        if width == 0 || height == 0 {
            return Err(RenderError::Texture("zero-size texture".into()));
        }

        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|v| v.checked_mul(4))
            .ok_or_else(|| RenderError::Texture("texture dimensions overflow".into()))?;
        if rgba.len() != expected {
            return Err(RenderError::Texture(format!(
                "RGBA buffer size mismatch: expected {width}x{height}x4={expected}, got {}",
                rgba.len()
            )));
        }

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
    #[must_use]
    #[inline]
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
        tracing::debug!(id, "texture cache insert");
        let bind_group = texture.bind_group(device, layout);
        self.entries.insert(
            id,
            CachedTexture {
                texture,
                bind_group,
            },
        );
    }

    /// Get the bind group for a texture ID, or load from bytes if not cached.
    pub fn get_or_load(
        &mut self,
        id: u64,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        bytes: &[u8],
        label: &str,
    ) -> Result<&wgpu::BindGroup> {
        match self.entries.entry(id) {
            Entry::Occupied(e) => {
                tracing::debug!(id, "texture cache hit");
                Ok(&e.into_mut().bind_group)
            }
            Entry::Vacant(e) => {
                let texture = Texture::from_bytes(device, queue, bytes, label)?;
                let bind_group = texture.bind_group(device, layout);
                let cached = e.insert(CachedTexture {
                    texture,
                    bind_group,
                });
                Ok(&cached.bind_group)
            }
        }
    }

    /// Get the bind group for a texture ID.
    #[must_use]
    pub fn get_bind_group(&self, id: u64) -> Option<&wgpu::BindGroup> {
        self.entries.get(&id).map(|e| &e.bind_group)
    }

    /// Check if a texture ID exists.
    #[must_use]
    #[inline]
    pub fn contains(&self, id: u64) -> bool {
        self.entries.contains_key(&id)
    }

    /// Get a texture by ID.
    #[must_use]
    pub fn get(&self, id: u64) -> Option<&Texture> {
        self.entries.get(&id).map(|e| &e.texture)
    }

    /// Number of cached textures.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    #[inline]
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

    #[test]
    fn mip_level_count_values() {
        assert_eq!(mip_level_count(1, 1), 1);
        assert_eq!(mip_level_count(2, 2), 2);
        assert_eq!(mip_level_count(4, 4), 3);
        assert_eq!(mip_level_count(256, 256), 9);
        assert_eq!(mip_level_count(1024, 512), 11);
        assert_eq!(mip_level_count(1, 512), 10);
        assert_eq!(mip_level_count(0, 0), 1);
    }

    #[test]
    fn validate_dimensions_within_limits() {
        let limits = wgpu::Limits {
            max_texture_dimension_2d: 8192,
            ..Default::default()
        };
        assert!(validate_dimensions(1024, 1024, &limits).is_ok());
        assert!(validate_dimensions(8192, 8192, &limits).is_ok());
    }

    #[test]
    fn validate_dimensions_exceeds_limits() {
        let limits = wgpu::Limits {
            max_texture_dimension_2d: 8192,
            ..Default::default()
        };
        assert!(validate_dimensions(8193, 1024, &limits).is_err());
        assert!(validate_dimensions(1024, 8193, &limits).is_err());
    }
}
