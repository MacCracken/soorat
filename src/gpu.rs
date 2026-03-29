//! GPU device and surface management.

use crate::error::{RenderError, Result};

/// Holds the wgpu device, queue, and instance.
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuContext {
    /// Request a GPU context (adapter + device + queue) without a surface.
    /// The adapter may not support presentation — use `new_for_surface` when rendering to a window.
    pub async fn new() -> Result<Self> {
        Self::new_inner(None).await
    }

    /// Request a GPU context compatible with the given surface.
    /// Ensures the adapter can present to this surface.
    pub async fn new_for_surface(surface: &wgpu::Surface<'_>) -> Result<Self> {
        Self::new_inner(Some(surface)).await
    }

    async fn new_inner(compatible_surface: Option<&wgpu::Surface<'_>>) -> Result<Self> {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
        desc.backends = wgpu::Backends::all();
        let instance = wgpu::Instance::new(desc);

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| RenderError::AdapterNotFound)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|e: wgpu::RequestDeviceError| RenderError::DeviceRequest(e.to_string()))?;

        tracing::info!(
            adapter = adapter.get_info().name,
            backend = ?adapter.get_info().backend,
            "GPU context initialized"
        );

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    /// Get adapter info.
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }

    /// Get device limits.
    pub fn limits(&self) -> wgpu::Limits {
        self.device.limits()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn gpu_context_types() {
        // GpuContext requires async + GPU — test the type exists
        let _size = std::mem::size_of::<super::GpuContext>();
    }
}
