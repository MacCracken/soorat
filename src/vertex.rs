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
                // color (after position [f32; 2] + tex_coords [f32; 2])
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 2]>() * 2) as wgpu::BufferAddress,
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

/// A skinned 3D vertex with tangent, joint indices, and joint weights.
/// Used for skeletal animation and normal-mapped meshes.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Serialize, Deserialize, bytemuck::Pod, bytemuck::Zeroable,
)]
pub struct SkinnedVertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
    /// Tangent vector for normal mapping (xyz = tangent, w = handedness ±1).
    pub tangent: [f32; 4],
    /// Joint indices (up to 4 joints per vertex).
    pub joints: [u32; 4],
    /// Joint weights (sum to 1.0).
    pub weights: [f32; 4],
}

impl SkinnedVertex3D {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem::size_of;
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
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
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // tex_coords
                wgpu::VertexAttribute {
                    offset: (size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // color
                wgpu::VertexAttribute {
                    offset: (size_of::<[f32; 3]>() * 2 + size_of::<[f32; 2]>())
                        as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // tangent
                wgpu::VertexAttribute {
                    offset: (size_of::<[f32; 3]>() * 2
                        + size_of::<[f32; 2]>()
                        + size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // joints
                wgpu::VertexAttribute {
                    offset: (size_of::<[f32; 3]>() * 2
                        + size_of::<[f32; 2]>()
                        + size_of::<[f32; 4]>() * 2)
                        as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Uint32x4,
                },
                // weights
                wgpu::VertexAttribute {
                    offset: (size_of::<[f32; 3]>() * 2
                        + size_of::<[f32; 2]>()
                        + size_of::<[f32; 4]>() * 2
                        + size_of::<[u32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

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

    #[test]
    fn vertex3d_serde() {
        let v = Vertex3D {
            position: [1.0, 2.0, 3.0],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [0.5, 0.5],
            color: [1.0, 0.0, 0.0, 1.0],
        };
        let json = serde_json::to_string(&v).unwrap();
        let decoded: Vertex3D = serde_json::from_str(&json).unwrap();
        assert_eq!(v, decoded);
    }

    #[test]
    fn vertex2d_bytemuck_cast() {
        let v = Vertex2D {
            position: [1.0, 2.0],
            tex_coords: [0.5, 0.5],
            color: [1.0, 1.0, 1.0, 1.0],
        };
        let bytes: &[u8] = bytemuck::bytes_of(&v);
        assert_eq!(bytes.len(), 32);
        let back: &Vertex2D = bytemuck::from_bytes(bytes);
        assert_eq!(*back, v);
    }

    #[test]
    fn vertex3d_bytemuck_cast() {
        let v = Vertex3D {
            position: [1.0, 2.0, 3.0],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [0.5, 0.5],
            color: [1.0, 0.0, 0.0, 1.0],
        };
        let bytes: &[u8] = bytemuck::bytes_of(&v);
        assert_eq!(bytes.len(), 48);
        let back: &Vertex3D = bytemuck::from_bytes(bytes);
        assert_eq!(*back, v);
    }

    #[test]
    fn vertex2d_zeroed() {
        let v = Vertex2D::zeroed();
        assert_eq!(v.position, [0.0, 0.0]);
        assert_eq!(v.tex_coords, [0.0, 0.0]);
        assert_eq!(v.color, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn vertex3d_zeroed() {
        let v = Vertex3D::zeroed();
        assert_eq!(v.position, [0.0, 0.0, 0.0]);
        assert_eq!(v.normal, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn vertex2d_layout_offsets() {
        let layout = Vertex2D::layout();
        assert_eq!(layout.attributes[0].offset, 0); // position
        assert_eq!(layout.attributes[1].offset, 8); // tex_coords (2 * f32)
        assert_eq!(layout.attributes[2].offset, 16); // color (4 * f32)
    }

    #[test]
    fn vertex3d_layout_offsets() {
        let layout = Vertex3D::layout();
        assert_eq!(layout.attributes[0].offset, 0); // position
        assert_eq!(layout.attributes[1].offset, 12); // normal (3 * f32)
        assert_eq!(layout.attributes[2].offset, 24); // tex_coords (6 * f32)
        assert_eq!(layout.attributes[3].offset, 32); // color (8 * f32)
    }

    #[test]
    fn vertex2d_batch_bytemuck() {
        let verts = vec![
            Vertex2D {
                position: [0.0, 0.0],
                tex_coords: [0.0, 0.0],
                color: [1.0; 4],
            },
            Vertex2D {
                position: [1.0, 0.0],
                tex_coords: [1.0, 0.0],
                color: [1.0; 4],
            },
            Vertex2D {
                position: [0.0, 1.0],
                tex_coords: [0.0, 1.0],
                color: [1.0; 4],
            },
        ];
        let bytes: &[u8] = bytemuck::cast_slice(&verts);
        assert_eq!(bytes.len(), 32 * 3);
    }

    #[test]
    fn skinned_vertex3d_size() {
        // 3+3+2+4+4+4+4 = 24 floats + 4 u32s = 24*4 + 4*4 = 96 + 16 = 112 bytes
        // Actually: 3+3+2+4 = 12 floats (48) + 4 tangent (16) + 4 u32 joints (16) + 4 weights (16) = 96
        assert_eq!(std::mem::size_of::<SkinnedVertex3D>(), 96);
    }

    #[test]
    fn skinned_vertex3d_layout() {
        let layout = SkinnedVertex3D::layout();
        assert_eq!(layout.array_stride, 96);
        assert_eq!(layout.attributes.len(), 7);
    }

    #[test]
    fn skinned_vertex3d_bytemuck() {
        let v = SkinnedVertex3D {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [0.0; 2],
            color: [1.0; 4],
            tangent: [1.0, 0.0, 0.0, 1.0],
            joints: [0, 1, 0, 0],
            weights: [0.5, 0.5, 0.0, 0.0],
        };
        let bytes = bytemuck::bytes_of(&v);
        assert_eq!(bytes.len(), 96);
    }

    #[test]
    fn skinned_vertex3d_zeroed() {
        let v = SkinnedVertex3D::zeroed();
        assert_eq!(v.position, [0.0; 3]);
        assert_eq!(v.joints, [0; 4]);
        assert_eq!(v.weights, [0.0; 4]);
    }
}
