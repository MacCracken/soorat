//! GPU device and surface management.
//!
//! Re-exports [`mabda::GpuContext`] as the shared GPU foundation.
//! All soorat subsystems share a single context for device, queue, and adapter access.

pub use mabda::context::{GpuContext, GpuContextBuilder};

#[cfg(test)]
mod tests {
    #[test]
    fn gpu_context_types() {
        let _size = std::mem::size_of::<super::GpuContext>();
    }
}
