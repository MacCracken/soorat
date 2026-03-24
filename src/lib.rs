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
pub mod debug_draw;
pub mod error;
pub mod gltf_loader;
pub mod gpu;
pub mod material;
pub mod mesh_pipeline;
pub mod pbr_material;
pub mod pipeline;
pub mod render_target;
pub mod sprite;
pub mod texture;
pub mod vertex;
pub mod window;

pub use debug_draw::{LineBatch, LinePipeline, LineVertex};
pub use error::{RenderError, Result};
pub use gpu::GpuContext;
pub use material::Material;
pub use mesh_pipeline::{CameraUniforms, DepthBuffer, LightUniforms, Mesh, MeshPipeline};
pub use pbr_material::MaterialUniforms;
pub use pipeline::{FrameStats, SpriteBuffers, SpritePipeline};
pub use render_target::RenderTarget;
pub use texture::{Texture, TextureCache};
pub use window::{Window, WindowConfig};
