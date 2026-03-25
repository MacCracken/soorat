//! Window management via winit.

use crate::error::{RenderError, Result};
use crate::gpu::GpuContext;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Window configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub vsync: bool,
    pub fullscreen: bool,
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            title: "Soorat".into(),
            vsync: true,
            fullscreen: false,
            resizable: true,
        }
    }
}

impl WindowConfig {
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            title: title.into(),
            width,
            height,
            ..Default::default()
        }
    }

    /// Aspect ratio as f32. Returns 1.0 if height is 0.
    pub fn aspect_ratio(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    /// Present mode based on vsync setting.
    pub fn present_mode(&self) -> wgpu::PresentMode {
        if self.vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        }
    }
}

/// A window with an attached wgpu surface for rendering.
pub struct Window {
    pub gpu: GpuContext,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub winit_window: Arc<winit::window::Window>,
}

impl Window {
    /// Create a new window and GPU context from a winit window.
    /// The winit window must already be created (within an ApplicationHandler).
    pub async fn new(
        winit_window: Arc<winit::window::Window>,
        config: &WindowConfig,
    ) -> Result<Self> {
        let gpu = GpuContext::new().await?;

        let surface = gpu
            .instance
            .create_surface(winit_window.clone())
            .map_err(|e| RenderError::SurfaceConfig(e.to_string()))?;

        let surface_caps = surface.get_capabilities(&gpu.adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .or(surface_caps.formats.first())
            .copied()
            .ok_or_else(|| RenderError::SurfaceConfig("no supported surface formats".into()))?;

        let alpha_mode = surface_caps
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| RenderError::SurfaceConfig("no supported alpha modes".into()))?;

        let size = winit_window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: config.present_mode(),
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&gpu.device, &surface_config);

        tracing::info!(
            format = ?surface_format,
            "Window surface configured"
        );

        Ok(Self {
            gpu,
            surface,
            surface_config,
            winit_window,
        })
    }

    /// Create a window using an existing GPU context (for multi-window setups).
    /// The GpuContext is shared across windows; each window owns its own surface.
    pub fn new_with_gpu(
        gpu: GpuContext,
        winit_window: Arc<winit::window::Window>,
        config: &WindowConfig,
    ) -> Result<Self> {
        let surface = gpu
            .instance
            .create_surface(winit_window.clone())
            .map_err(|e| RenderError::SurfaceConfig(e.to_string()))?;

        let surface_caps = surface.get_capabilities(&gpu.adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .or(surface_caps.formats.first())
            .copied()
            .ok_or_else(|| RenderError::SurfaceConfig("no supported surface formats".into()))?;

        let alpha_mode = surface_caps
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| RenderError::SurfaceConfig("no supported alpha modes".into()))?;

        let size = winit_window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: config.present_mode(),
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&gpu.device, &surface_config);

        Ok(Self {
            gpu,
            surface,
            surface_config,
            winit_window,
        })
    }

    /// Reconfigure the surface after a resize.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface
            .configure(&self.gpu.device, &self.surface_config);
    }

    /// Get the current surface texture for rendering.
    pub fn current_texture(&self) -> Result<wgpu::SurfaceTexture> {
        self.surface
            .get_current_texture()
            .map_err(|e| RenderError::SurfaceTexture(e.to_string()))
    }

    /// Surface format.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    /// Current surface dimensions.
    pub fn size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    /// Request a redraw from the windowing system.
    pub fn request_redraw(&self) {
        self.winit_window.request_redraw();
    }
}

/// Run the event loop with a callback-based application handler.
///
/// `init` is called once when the window is created, receiving the Window.
/// `frame` is called each frame with the Window, returning false to exit.
pub fn run(
    config: WindowConfig,
    init: impl FnOnce(&mut Window) + 'static,
    mut frame: impl FnMut(&mut Window) -> bool + 'static,
) -> Result<()> {
    let event_loop =
        winit::event_loop::EventLoop::new().map_err(|e| RenderError::Window(e.to_string()))?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut app = App {
        config,
        window: None,
        init: Some(Box::new(init)),
        frame: Box::new(move |w| frame(w)),
    };

    event_loop
        .run_app(&mut app)
        .map_err(|e| RenderError::Window(e.to_string()))
}

type InitCallback = Box<dyn FnOnce(&mut Window)>;
type FrameCallback = Box<dyn FnMut(&mut Window) -> bool>;

struct App {
    config: WindowConfig,
    window: Option<Window>,
    init: Option<InitCallback>,
    frame: FrameCallback,
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = winit::window::WindowAttributes::default()
            .with_title(&self.config.title)
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.config.width,
                self.config.height,
            ))
            .with_resizable(self.config.resizable);

        let winit_window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!("Failed to create window: {e}");
                event_loop.exit();
                return;
            }
        };

        let window = match pollster::block_on(Window::new(winit_window, &self.config)) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("Failed to init GPU: {e}");
                event_loop.exit();
                return;
            }
        };

        self.window = Some(window);

        if let Some(init) = self.init.take()
            && let Some(w) = self.window.as_mut()
        {
            init(w);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(window) = self.window.as_mut() else {
            return;
        };

        match event {
            winit::event::WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            winit::event::WindowEvent::Resized(size) => {
                window.resize(size.width, size.height);
            }
            winit::event::WindowEvent::RedrawRequested => {
                if !(self.frame)(window) {
                    event_loop.exit();
                }
                window.request_redraw();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_config_default() {
        let cfg = WindowConfig::default();
        assert_eq!(cfg.width, 1280);
        assert_eq!(cfg.height, 720);
        assert_eq!(cfg.title, "Soorat");
        assert!(cfg.vsync);
        assert!(!cfg.fullscreen);
        assert!(cfg.resizable);
    }

    #[test]
    fn window_config_new() {
        let cfg = WindowConfig::new("My Game", 1920, 1080);
        assert_eq!(cfg.title, "My Game");
        assert_eq!(cfg.width, 1920);
    }

    #[test]
    fn aspect_ratio() {
        let cfg = WindowConfig::default();
        assert!((cfg.aspect_ratio() - 16.0 / 9.0).abs() < 0.01);

        let square = WindowConfig::new("Square", 100, 100);
        assert!((square.aspect_ratio() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn present_mode() {
        let mut cfg = WindowConfig::default();
        assert_eq!(cfg.present_mode(), wgpu::PresentMode::AutoVsync);

        cfg.vsync = false;
        assert_eq!(cfg.present_mode(), wgpu::PresentMode::AutoNoVsync);
    }

    #[test]
    fn window_config_serde() {
        let cfg = WindowConfig::new("Test", 800, 600);
        let json = serde_json::to_string(&cfg).unwrap();
        let decoded: WindowConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, decoded);
    }

    #[test]
    fn aspect_ratio_zero_height() {
        let cfg = WindowConfig::new("Zero", 100, 0);
        assert_eq!(cfg.aspect_ratio(), 1.0);
    }

    #[test]
    fn window_config_fullscreen_resizable() {
        let mut cfg = WindowConfig::default();
        assert!(!cfg.fullscreen);
        assert!(cfg.resizable);
        cfg.fullscreen = true;
        cfg.resizable = false;
        assert!(cfg.fullscreen);
        assert!(!cfg.resizable);
    }

    #[test]
    fn window_config_clone_eq() {
        let a = WindowConfig::new("Test", 800, 600);
        let b = a.clone();
        assert_eq!(a, b);
    }
}
