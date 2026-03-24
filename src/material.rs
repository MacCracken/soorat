//! Material types for 3D mesh rendering.

use crate::color::Color;
use crate::texture::Texture;

/// A material defining the surface appearance of a mesh.
pub struct Material {
    pub base_color_texture: Texture,
    pub base_color_factor: Color,
    pub bind_group: wgpu::BindGroup,
}

impl Material {
    /// Create a material from a texture and color factor.
    pub fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        texture: Texture,
        base_color_factor: Color,
    ) -> Self {
        let bind_group = texture.bind_group(device, layout);
        Self {
            base_color_texture: texture,
            base_color_factor,
            bind_group,
        }
    }

    /// Create a default material (white pixel texture, white color).
    pub fn default_material(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let texture = Texture::white_pixel(device, queue);
        Self::new(device, layout, texture, Color::WHITE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_types_exist() {
        let _size = std::mem::size_of::<Material>();
    }
}
