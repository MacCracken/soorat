//! Window management via winit.

use serde::{Deserialize, Serialize};

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
