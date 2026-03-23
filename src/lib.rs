//! Soorat — GPU rendering engine for AGNOS
//!
//! **Soorat** (Arabic/Urdu: صورت — form, image, appearance) is a wgpu-based
//! rendering engine designed for the Kiran game engine and AGNOS ecosystem.
//!
//! Provides:
//! - GPU device and surface management
//! - Sprite rendering pipeline (2D)
//! - Mesh rendering pipeline (3D)
//! - Window management via winit
//! - Render pass abstraction

pub mod color;
pub mod error;
pub mod gpu;
pub mod sprite;
pub mod vertex;
pub mod window;

pub use error::{RenderError, Result};
pub use gpu::GpuContext;
pub use window::WindowConfig;
