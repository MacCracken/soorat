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
    /// Request a GPU context (adapter + device + queue).
    /// This is async because wgpu adapter/device requests are async.
    pub async fn new() -> Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RenderError::AdapterNotFound)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .map_err(|e| RenderError::DeviceRequest(e.to_string()))?;

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
