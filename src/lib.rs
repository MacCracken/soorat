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

pub mod animation;
pub mod color;
pub mod debug_draw;
pub mod error;
pub mod gltf_loader;
pub mod gpu;
pub mod lights;
pub mod material;
pub mod math_util;
pub mod mesh_pipeline;
pub mod pbr_material;
pub mod pipeline;
pub mod postprocess;
pub mod render_target;
pub mod shadow;
pub mod sprite;
pub mod terrain;
pub mod text;
pub mod texture;
pub mod ui;
pub mod vertex;
pub mod window;

pub use animation::{AnimationClip, JointUniforms, Skeleton};
pub use debug_draw::{LineBatch, LinePipeline, LineVertex};
pub use error::{RenderError, Result};
pub use gpu::GpuContext;
pub use lights::{GpuLight, LightArrayUniforms};
pub use material::Material;
pub use mesh_pipeline::{CameraUniforms, DepthBuffer, LightUniforms, Mesh, MeshPipeline};
pub use pbr_material::MaterialUniforms;
pub use pipeline::{FrameStats, SpriteBuffers, SpritePipeline};
pub use postprocess::{PostProcessPipeline, PostProcessUniforms, ToneMapMode};
pub use render_target::RenderTarget;
pub use shadow::{ShadowMap, ShadowPipeline, ShadowUniforms};
pub use terrain::TerrainConfig;
pub use text::{BitmapFont, TextBatch};
pub use texture::{Texture, TextureCache};
pub use ui::{UiBatch, UiLabel, UiPanel};
pub use window::{Window, WindowConfig};
