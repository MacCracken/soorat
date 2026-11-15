//! Compute shader pipeline — general-purpose GPU compute.
//!
//! Re-exports [`mabda::compute`] types which provide the full compute pipeline
//! abstraction: single/multi bind group layouts, dispatch helpers, ping-pong
//! buffers, and workgroup utilities.
//!
//! Soorat-specific compute usage (e.g. [`crate::gpu_particles`]) builds on
//! raw wgpu directly for tighter control.

pub use mabda::compute::{
    ComputePipeline, PingPongBuffer, validate_dispatch, workgroups_1d, workgroups_2d,
};

/// Helper to create a GPU storage buffer.
pub fn create_storage_buffer(
    device: &wgpu::Device,
    data: &[u8],
    label: &str,
    read_only: bool,
) -> wgpu::Buffer {
    mabda::buffer::create_storage_buffer(device, data, label, read_only)
}

/// Helper to create an empty GPU storage buffer with a given size.
pub fn create_storage_buffer_empty(
    device: &wgpu::Device,
    size: u64,
    label: &str,
    read_only: bool,
) -> wgpu::Buffer {
    mabda::buffer::create_storage_buffer_empty(device, size, label, read_only)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_pipeline_types() {
        let _size = std::mem::size_of::<ComputePipeline>();
    }

    #[test]
    fn workgroups_1d_exact() {
        assert_eq!(workgroups_1d(256, 256), 1);
        assert_eq!(workgroups_1d(512, 256), 2);
    }

    #[test]
    fn workgroups_1d_remainder() {
        assert_eq!(workgroups_1d(257, 256), 2);
        assert_eq!(workgroups_1d(1, 256), 1);
    }

    #[test]
    fn workgroups_2d_exact() {
        assert_eq!(workgroups_2d(32, 32, 16, 16), (2, 2));
    }

    #[test]
    fn workgroups_2d_remainder() {
        assert_eq!(workgroups_2d(33, 17, 16, 16), (3, 2));
    }

    #[test]
    fn workgroups_1d_single() {
        assert_eq!(workgroups_1d(1, 64), 1);
        assert_eq!(workgroups_1d(0, 64), 0);
    }

    #[test]
    fn workgroups_2d_single() {
        assert_eq!(workgroups_2d(1, 1, 8, 8), (1, 1));
        assert_eq!(workgroups_2d(0, 0, 8, 8), (0, 0));
    }

    #[test]
    fn workgroups_1d_large() {
        assert_eq!(workgroups_1d(1_000_000, 256), 3907);
        assert_eq!(workgroups_1d(u32::MAX, 256), 16_777_216);
    }

    #[test]
    fn validate_dispatch_within_limits() {
        let limits = wgpu::Limits {
            max_compute_workgroups_per_dimension: 65535,
            ..Default::default()
        };
        assert!(validate_dispatch(&limits, 100, 100, 1).is_ok());
        assert!(validate_dispatch(&limits, 65535, 65535, 65535).is_ok());
    }

    #[test]
    fn validate_dispatch_exceeds_limits() {
        let limits = wgpu::Limits {
            max_compute_workgroups_per_dimension: 65535,
            ..Default::default()
        };
        assert!(validate_dispatch(&limits, 65536, 1, 1).is_err());
        assert!(validate_dispatch(&limits, 1, 65536, 1).is_err());
        assert!(validate_dispatch(&limits, 1, 1, 65536).is_err());
    }

    #[test]
    fn ping_pong_types() {
        let _size = std::mem::size_of::<PingPongBuffer>();
    }

    // NOTE: `workgroups_1d(n, 0)` will panic due to division by zero in mabda's
    // implementation. This is a known mabda issue — callers must ensure
    // workgroup_size > 0. No test here because it would panic unconditionally.
}
