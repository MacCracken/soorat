//! Environment maps — cubemap textures for IBL (Image-Based Lighting).

use crate::error::{RenderError, Result};
use crate::texture::mip_level_count;

// ── CPU mip-chain generation ────────────────────────────────────────────────
//
// wgpu does not generate mipmaps, and the PBR shader samples the
// environment maps at LOD > 0 (prefiltered: `roughness * 4`; irradiance:
// implicit LOD under minification). Unwritten mips are zero-initialized, i.e.
// black, so every level is filled here with a 2×2 box filter in linear space.

/// Encoding of an environment-map face, for CPU mip generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FaceEncoding {
    /// `Rgba8UnormSrgb` — 4 bytes per texel, sRGB-encoded color.
    Srgb8,
    /// `Rgba16Float` — 8 bytes per texel, linear half floats.
    F16,
}

impl FaceEncoding {
    fn of(format: wgpu::TextureFormat) -> Option<Self> {
        match format {
            wgpu::TextureFormat::Rgba8UnormSrgb => Some(Self::Srgb8),
            wgpu::TextureFormat::Rgba16Float => Some(Self::F16),
            _ => None,
        }
    }

    pub(crate) fn bytes_per_texel(self) -> usize {
        match self {
            Self::Srgb8 => 4,
            Self::F16 => 8,
        }
    }

    /// Decode a face into linear RGBA f32 texels.
    pub(crate) fn decode(self, bytes: &[u8]) -> Vec<[f32; 4]> {
        match self {
            Self::Srgb8 => {
                let lut = srgb8_decode_table();
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|px| {
                        [
                            lut[px[0] as usize],
                            lut[px[1] as usize],
                            lut[px[2] as usize],
                            f32::from(px[3]) / 255.0,
                        ]
                    })
                    .collect()
            }
            Self::F16 => bytes
                .as_chunks::<8>()
                .0
                .iter()
                .map(|px| {
                    let h = |i: usize| f16_to_f32(u16::from_le_bytes([px[i], px[i + 1]]));
                    [h(0), h(2), h(4), h(6)]
                })
                .collect(),
        }
    }

    /// Encode linear RGBA f32 texels into this face encoding.
    pub(crate) fn encode(self, texels: &[[f32; 4]]) -> Vec<u8> {
        let mut out = Vec::with_capacity(texels.len() * self.bytes_per_texel());
        for px in texels {
            match self {
                Self::Srgb8 => {
                    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                    out.extend_from_slice(&[
                        q(linear_to_srgb(px[0])),
                        q(linear_to_srgb(px[1])),
                        q(linear_to_srgb(px[2])),
                        q(px[3]),
                    ]);
                }
                Self::F16 => {
                    for &c in px {
                        out.extend_from_slice(&f32_to_f16(c).to_le_bytes());
                    }
                }
            }
        }
        out
    }
}

/// sRGB transfer function, decode (IEC 61966-2-1).
#[must_use]
#[inline]
pub(crate) fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// sRGB transfer function, encode (IEC 61966-2-1).
#[must_use]
#[inline]
pub(crate) fn linear_to_srgb(l: f32) -> f32 {
    if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    }
}

/// Linear value of each sRGB-encoded byte: the decode table for `Srgb8` faces.
#[must_use]
pub(crate) fn srgb8_decode_table() -> [f32; 256] {
    std::array::from_fn(|i| srgb_to_linear(i as f32 / 255.0))
}

/// IEEE 754 binary16 → binary32.
#[must_use]
#[inline]
pub(crate) fn f16_to_f32(h: u16) -> f32 {
    let sign = u32::from(h >> 15) << 31;
    let exp = u32::from((h >> 10) & 0x1f);
    let mant = u32::from(h & 0x3ff);
    match exp {
        0 => {
            // Zero or subnormal: mant × 2⁻²⁴
            let v = mant as f32 * (1.0 / 16_777_216.0);
            if sign != 0 { -v } else { v }
        }
        31 => f32::from_bits(sign | 0x7f80_0000 | (mant << 13)),
        _ => f32::from_bits(sign | ((exp + 112) << 23) | (mant << 13)),
    }
}

/// IEEE 754 binary32 → binary16, round-to-nearest-even; overflow → ±inf.
#[must_use]
#[inline]
pub(crate) fn f32_to_f16(v: f32) -> u16 {
    let x = v.to_bits();
    let sign = ((x >> 16) & 0x8000) as u16;
    let exp = ((x >> 23) & 0xff) as i32;
    let mant = x & 0x7f_ffff;
    if exp == 0xff {
        return sign | 0x7c00 | if mant != 0 { 0x200 } else { 0 };
    }
    let e = exp - 127 + 15;
    if e >= 0x1f {
        return sign | 0x7c00;
    }
    let round = |value: u32, shift: u32| -> u32 {
        let half = 1u32 << (shift - 1);
        let rem = value & ((1u32 << shift) - 1);
        let r = value >> shift;
        if rem > half || (rem == half && r & 1 == 1) {
            r + 1
        } else {
            r
        }
    };
    if e <= 0 {
        if e < -10 {
            return sign; // underflows to ±0
        }
        // Subnormal: (1.m × 2^(exp-127)) / 2⁻²⁴ = m_full >> (14 - e)
        return sign | round(mant | 0x80_0000, (14 - e) as u32) as u16;
    }
    // Carry out of the mantissa correctly bumps the exponent (up to inf).
    sign | round(((e as u32) << 23) | mant, 13) as u16
}

/// 2×2 box downsample of a square `size × size` linear image to
/// `max(size/2, 1)` (odd edges clamp to the last texel).
pub(crate) fn downsample(src: &[[f32; 4]], size: usize) -> Vec<[f32; 4]> {
    let Some(last) = size.checked_sub(1) else {
        return Vec::new();
    };
    if src.len() < size.saturating_mul(size) {
        return Vec::new();
    }
    let dst = (size / 2).max(1);
    let mut out = Vec::with_capacity(dst * dst);
    for y in 0..dst {
        let (y0, y1) = ((2 * y).min(last), (2 * y + 1).min(last));
        for x in 0..dst {
            let (x0, x1) = ((2 * x).min(last), (2 * x + 1).min(last));
            let taps = [
                src[y0 * size + x0],
                src[y0 * size + x1],
                src[y1 * size + x0],
                src[y1 * size + x1],
            ];
            out.push(std::array::from_fn(|c| {
                0.25 * (taps[0][c] + taps[1][c] + taps[2][c] + taps[3][c])
            }));
        }
    }
    out
}

/// The CPU mip chain of one face: levels `1..mip_count`, each as
/// `(edge size, encoded bytes)`. Level 0 is the face itself; each further
/// level is a 2×2 box filter of the previous one in linear space.
#[must_use]
pub(crate) fn face_mip_chain(
    encoding: FaceEncoding,
    face: &[u8],
    size: usize,
    mip_count: u32,
) -> Vec<(usize, Vec<u8>)> {
    let mut levels = Vec::with_capacity(mip_count.saturating_sub(1) as usize);
    if mip_count <= 1 {
        return levels;
    }
    let mut level = encoding.decode(face);
    let mut level_size = size;
    for _ in 1..mip_count {
        level = downsample(&level, level_size);
        level_size = (level_size / 2).max(1);
        levels.push((level_size, encoding.encode(&level)));
    }
    levels
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

    /// Create a cubemap from 6 face images with a specified format, filling
    /// the full mip chain (2×2 box filter in linear space).
    ///
    /// Inputs are validated before any GPU allocation: `size` must be non-zero
    /// and within the device's `max_texture_dimension_2d`, and every face must
    /// hold exactly `size² × bytes_per_texel` bytes.
    fn from_faces_with_format(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        faces: [&[u8]; 6],
        size: u32,
        format: wgpu::TextureFormat,
    ) -> Result<Self> {
        let encoding = FaceEncoding::of(format).ok_or_else(|| {
            RenderError::Texture(format!("unsupported environment map format {format:?}"))
        })?;
        if size == 0 {
            return Err(RenderError::Texture("zero-size environment map".into()));
        }
        let max = device.limits().max_texture_dimension_2d;
        if size > max {
            return Err(RenderError::Texture(format!(
                "environment map size {size} exceeds device limit {max}"
            )));
        }
        let side = size as usize;
        let expected_face_size = side
            .checked_mul(side)
            .and_then(|t| t.checked_mul(encoding.bytes_per_texel()))
            .ok_or_else(|| RenderError::Texture("environment map size overflows".into()))?;
        for (i, face_data) in faces.iter().enumerate() {
            if face_data.len() != expected_face_size {
                return Err(RenderError::Texture(format!(
                    "Face {i} size mismatch: expected {expected_face_size} bytes, got {}",
                    face_data.len()
                )));
            }
        }

        let mip_count = mip_level_count(size, size);
        tracing::debug!(size, mip_count, ?format, "creating environment map");
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("environment_map"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 6,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let bytes_per_texel = encoding.bytes_per_texel() as u32;
        let write_level = |face: u32, mip: u32, level_size: u32, data: &[u8]| {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: mip,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: face,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_texel * level_size),
                    rows_per_image: Some(level_size),
                },
                wgpu::Extent3d {
                    width: level_size,
                    height: level_size,
                    depth_or_array_layers: 1,
                },
            );
        };

        for (i, face_data) in faces.iter().enumerate() {
            let face = i as u32;
            write_level(face, 0, size, face_data);
            let chain = face_mip_chain(encoding, face_data, side, mip_count);
            for (mip, (level_size, data)) in (1..).zip(&chain) {
                write_level(face, mip, *level_size as u32, data);
            }
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
    fn f16_roundtrip_exact_values() {
        for v in [
            0.0f32,
            -0.0,
            1.0,
            -2.5,
            0.5,
            65504.0,
            6.103_515_6e-5, // smallest normal
            5.960_464_5e-8, // smallest subnormal
            0.333_251_95,   // 0x3555
        ] {
            let h = f32_to_f16(v);
            assert_eq!(f16_to_f32(h), v, "{v} → {h:#06x}");
        }
        assert_eq!(f32_to_f16(1.0), 0x3c00);
        assert_eq!(f32_to_f16(-2.0), 0xc000);
    }

    #[test]
    fn f16_special_values() {
        assert_eq!(f32_to_f16(f32::INFINITY), 0x7c00);
        assert_eq!(f32_to_f16(1.0e6), 0x7c00); // overflow → inf
        assert_eq!(f32_to_f16(-1.0e6), 0xfc00);
        assert!(f16_to_f32(f32_to_f16(f32::NAN)).is_nan());
        assert_eq!(f32_to_f16(1.0e-10), 0); // underflow → 0
        assert_eq!(f16_to_f32(0x7c00), f32::INFINITY);
    }

    #[test]
    fn f16_rounds_to_nearest_even() {
        // 1 + 2⁻¹¹ is exactly halfway between 1.0 and the next f16 → even (1.0).
        assert_eq!(f32_to_f16(1.0 + 2f32.powi(-11)), 0x3c00);
        // 1 + 3·2⁻¹¹ is halfway between 0x3c01 and 0x3c02 → even (0x3c02).
        assert_eq!(f32_to_f16(1.0 + 3.0 * 2f32.powi(-11)), 0x3c02);
        // Rounding carry into the exponent: just below 2.0 rounds to 2.0.
        assert_eq!(f32_to_f16(2.0 - 2f32.powi(-13)), 0x4000);
    }

    #[test]
    fn srgb_transfer_roundtrip() {
        for i in 0..=255u8 {
            let c = f32::from(i) / 255.0;
            let back = linear_to_srgb(srgb_to_linear(c));
            assert!((back - c).abs() < 1e-5, "{i}: {back} vs {c}");
        }
    }

    #[test]
    fn downsample_box_filters_and_handles_odd_sizes() {
        let src = vec![
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [1.0, 1.0, 0.0, 1.0],
        ];
        assert_eq!(downsample(&src, 2), vec![[0.5, 0.5, 0.0, 1.0]]);
        // 3×3 → 1×1 using the clamped top-left 2×2 block.
        let src3: Vec<[f32; 4]> = (0..9).map(|i| [i as f32, 0.0, 0.0, 1.0]).collect();
        assert_eq!(downsample(&src3, 3), vec![[2.0, 0.0, 0.0, 1.0]]);
        assert_eq!(downsample(&[[7.0; 4]], 1), vec![[7.0; 4]]);
    }

    #[test]
    fn mip_chain_preserves_average_color() {
        // A 4×4 checkerboard averages to mid-gray (in linear space) at every level.
        for enc in [FaceEncoding::Srgb8, FaceEncoding::F16] {
            let texels: Vec<[f32; 4]> = (0..16)
                .map(|i| {
                    let on = ((i % 4) + (i / 4)) % 2 == 0;
                    let v = if on { 1.0 } else { 0.0 };
                    [v, v, v, 1.0]
                })
                .collect();
            let bytes = enc.encode(&texels);
            assert_eq!(bytes.len(), 16 * enc.bytes_per_texel());
            let mut level = enc.decode(&bytes);
            let mut size = 4;
            while size > 1 {
                level = downsample(&level, size);
                size /= 2;
            }
            let px = level[0];
            assert!((px[0] - 0.5).abs() < 1e-3, "{enc:?}: {px:?}");
            assert!((px[3] - 1.0).abs() < 1e-6);
        }
    }

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
