//! Egui integration helpers — bridge soorat render targets to egui textures.
//!
//! These helpers allow soorat's offscreen render targets to be displayed
//! inside egui panels (for the salai editor viewport).
//!
//! Usage with egui-wgpu:
//! ```ignore
//! let sampler = soorat::egui_bridge::create_egui_sampler(&device);
//! let tex_id = egui_renderer.register_native_texture(&device, &render_target.view, &sampler);
//! ui.image(tex_id, [width, height]);
//! ```

/// Create a linear sampler suitable for displaying soorat textures in egui.
pub fn create_egui_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("egui_bridge_sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    })
}

/// Viewport dimensions for egui render callbacks.
#[derive(Debug, Clone, Copy)]
pub struct ViewportSize {
    pub width: u32,
    pub height: u32,
}

impl ViewportSize {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    /// Check if the viewport needs a resize (dimensions changed).
    pub fn needs_resize(&self, new_width: u32, new_height: u32) -> bool {
        self.width != new_width || self.height != new_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_size() {
        let v = ViewportSize::new(1280, 720);
        assert!((v.aspect_ratio() - 16.0 / 9.0).abs() < 0.01);
    }

    #[test]
    fn viewport_needs_resize() {
        let v = ViewportSize::new(800, 600);
        assert!(!v.needs_resize(800, 600));
        assert!(v.needs_resize(1024, 768));
    }

    #[test]
    fn viewport_zero_height() {
        let v = ViewportSize::new(100, 0);
        assert_eq!(v.aspect_ratio(), 1.0);
    }
}
