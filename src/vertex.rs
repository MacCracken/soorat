//! Vertex types and buffer layouts.

use serde::{Deserialize, Serialize};

/// A 2D vertex with position, texture coordinates, and color.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Serialize, Deserialize, bytemuck::Pod, bytemuck::Zeroable,
)]
pub struct Vertex2D {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex2D {
    /// wgpu vertex buffer layout for Vertex2D.
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // tex_coords
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // color
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// A 3D vertex with position, normal, texture coordinates, and color.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Serialize, Deserialize, bytemuck::Pod, bytemuck::Zeroable,
)]
pub struct Vertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex3D {
    /// wgpu vertex buffer layout for Vertex3D.
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // normal
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // tex_coords
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // color
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2 + std::mem::size_of::<[f32; 2]>())
                        as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex2d_size() {
        // 2 + 2 + 4 = 8 floats = 32 bytes
        assert_eq!(std::mem::size_of::<Vertex2D>(), 32);
    }

    #[test]
    fn vertex3d_size() {
        // 3 + 3 + 2 + 4 = 12 floats = 48 bytes
        assert_eq!(std::mem::size_of::<Vertex3D>(), 48);
    }

    #[test]
    fn vertex2d_layout_stride() {
        let layout = Vertex2D::layout();
        assert_eq!(layout.array_stride, 32);
        assert_eq!(layout.attributes.len(), 3);
    }

    #[test]
    fn vertex3d_layout_stride() {
        let layout = Vertex3D::layout();
        assert_eq!(layout.array_stride, 48);
        assert_eq!(layout.attributes.len(), 4);
    }

    #[test]
    fn vertex2d_serde() {
        let v = Vertex2D {
            position: [1.0, 2.0],
            tex_coords: [0.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
        };
        let json = serde_json::to_string(&v).unwrap();
        let decoded: Vertex2D = serde_json::from_str(&json).unwrap();
        assert_eq!(v, decoded);
    }
}
