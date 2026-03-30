//! Vertex types and buffer layouts.
//!
//! Re-exported from [`mabda`] — the shared GPU foundation.

pub use mabda::vertex::{SkinnedVertex3D, Vertex2D, Vertex3D};

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    #[test]
    fn vertex2d_size() {
        assert_eq!(std::mem::size_of::<Vertex2D>(), 32);
    }

    #[test]
    fn vertex3d_size() {
        assert_eq!(std::mem::size_of::<Vertex3D>(), 48);
    }

    #[test]
    fn skinned_vertex3d_size() {
        assert_eq!(std::mem::size_of::<SkinnedVertex3D>(), 96);
    }

    #[test]
    fn vertex2d_layout_stride() {
        let layout = Vertex2D::layout();
        assert_eq!(layout.array_stride, 32);
    }

    #[test]
    fn vertex3d_layout_stride() {
        let layout = Vertex3D::layout();
        assert_eq!(layout.array_stride, 48);
    }

    #[test]
    fn vertex2d_zeroed() {
        let v = Vertex2D::zeroed();
        assert_eq!(v.position, [0.0, 0.0]);
    }

    #[test]
    fn vertex3d_zeroed() {
        let v = Vertex3D::zeroed();
        assert_eq!(v.position, [0.0, 0.0, 0.0]);
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
    }
}
