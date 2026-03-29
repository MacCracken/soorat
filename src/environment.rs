//! Environment maps — cubemap textures for IBL (Image-Based Lighting).

use crate::error::{RenderError, Result};

/// Compute the number of mip levels for a given texture size.
fn max_mip_levels(size: u32) -> u32 {
    (size as f32).log2().floor() as u32 + 1
}

/// A cubemap texture for environment lighting.
/// 6 faces: +X, -X, +Y, -Y, +Z, -Z.
pub struct EnvironmentMap {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: u32,
}

impl EnvironmentMap {
    /// Create a cubemap from 6 RGBA8 face images (sRGB).
    /// `faces`: array of 6 byte slices, each `size * size * 4` bytes.
    /// Order: +X, -X, +Y, -Y, +Z, -Z.
    /// For HDR/linear IBL data, use `from_faces_hdr` with Rgba16Float format.
    pub fn from_faces(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        faces: [&[u8]; 6],
        size: u32,
    ) -> Result<Self> {
        Self::from_faces_with_format(
            device,
            queue,
            faces,
            size,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )
    }

    /// Create a cubemap from 6 face images with a specified format.
    fn from_faces_with_format(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        faces: [&[u8]; 6],
        size: u32,
        format: wgpu::TextureFormat,
    ) -> Result<Self> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("environment_map"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 6,
            },
            mip_level_count: max_mip_levels(size),
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let bytes_per_pixel = format.block_copy_size(None).unwrap_or(4);
        let expected_face_size = (size * size * bytes_per_pixel) as usize;

        for (i, face_data) in faces.iter().enumerate() {
            if face_data.len() != expected_face_size {
                return Err(RenderError::Texture(format!(
                    "Face {i} size mismatch: expected {expected_face_size} bytes, got {}",
                    face_data.len()
                )));
            }

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                face_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_pixel * size),
                    rows_per_image: Some(size),
                },
                wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("environment_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
            size,
        })
    }

    /// Create a cubemap from 6 Rgba16Float face images (linear HDR).
    /// Each face is `size * size * 8` bytes (4 channels × f16).
    pub fn from_faces_hdr(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        faces: [&[u8]; 6],
        size: u32,
    ) -> Result<Self> {
        Self::from_faces_with_format(device, queue, faces, size, wgpu::TextureFormat::Rgba16Float)
    }

    /// Create a solid-color cubemap (uniform ambient).
    pub fn solid_color(device: &wgpu::Device, queue: &wgpu::Queue, color: [u8; 4]) -> Result<Self> {
        let size = 1u32;
        let face_data = vec![color[0], color[1], color[2], color[3]];
        Self::from_faces(
            device,
            queue,
            [
                &face_data, &face_data, &face_data, &face_data, &face_data, &face_data,
            ],
            size,
        )
    }
}

/// IBL bind group data — holds references to irradiance map, pre-filtered specular map, and BRDF LUT.
pub struct IblBindGroup {
    pub bind_group: wgpu::BindGroup,
}

impl IblBindGroup {
    /// Create an IBL bind group from environment maps and BRDF LUT.
    pub fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        irradiance_map: &EnvironmentMap,
        prefiltered_map: &EnvironmentMap,
        brdf_lut: &crate::texture::Texture,
    ) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ibl_bind_group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&irradiance_map.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&irradiance_map.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&prefiltered_map.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&prefiltered_map.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&brdf_lut.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&brdf_lut.sampler),
                },
            ],
        });

        Self { bind_group }
    }

    /// Create the bind group layout for IBL.
    pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ibl_layout"),
            entries: &[
                // irradiance cubemap
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // pre-filtered specular cubemap
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // BRDF LUT (2D)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ibl_bind_group_layout_types() {
        // Just verify types compile
        let _size = std::mem::size_of::<IblBindGroup>();
    }

    #[test]
    fn environment_map_types() {
        let _size = std::mem::size_of::<EnvironmentMap>();
    }
}
