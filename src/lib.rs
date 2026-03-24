//! Soorat — GPU rendering engine for AGNOS
//!
//! **Soorat** (Arabic/Urdu: صورت — form, image, appearance) is a wgpu-based
//! rendering engine designed for the Kiran game engine and AGNOS ecosystem.
//!
//! # Modules
//!
//! - **2D**: [`pipeline`] (sprites), [`sprite`], [`texture`], [`text`], [`ui`]
//! - **3D**: [`mesh_pipeline`] (PBR), [`shadow`], [`animation`], [`terrain`]
//! - **Debug**: [`debug_draw`] (wireframe lines, shapes, grid)
//! - **Post-processing**: [`postprocess`] (tone mapping, bloom)
//! - **Core**: [`gpu`], [`window`], [`color`], [`vertex`], [`error`]
//! - **Lights**: [`lights`] (directional, point, spot)
//! - **Loading**: [`gltf_loader`], [`texture`]

pub mod animation;
pub mod color;
pub mod debug_draw;
pub mod error;
pub mod gltf_loader;
pub mod gpu;
pub mod hdr;
pub mod lights;
pub mod material;
pub(crate) mod math_util;
pub mod mesh_pipeline;
pub mod pbr_material;
pub mod pipeline;
pub mod postprocess;
pub mod profiler;
pub mod render_target;
pub mod shadow;
pub mod sprite;
pub mod ssao;
pub mod terrain;
pub mod text;
pub mod texture;
pub mod ui;
pub mod vertex;
pub mod window;

// ── Core ────────────────────────────────────────────────────────────────────
pub use color::Color;
pub use error::{RenderError, Result};
pub use gpu::GpuContext;
pub use vertex::{SkinnedVertex3D, Vertex2D, Vertex3D};
pub use window::{Window, WindowConfig};

// ── 2D Sprites ──────────────────────────────────────────────────────────────
pub use pipeline::{
    FrameStats, SpriteBuffers, SpritePipeline, batch_to_vertices, batch_to_vertices_into,
    batch_to_vertices_u32,
};
pub use sprite::{Sprite, SpriteBatch, UvRect};
pub use texture::{Texture, TextureCache, create_default_sampler};

// ── 3D Meshes (PBR) ────────────────────────────────────────────────────────
pub use mesh_pipeline::{
    CameraUniforms, DepthBuffer, LightUniforms, Mesh, MeshPipeline, ShadowPassUniforms,
};
pub use pbr_material::MaterialUniforms;

// ── Lighting ────────────────────────────────────────────────────────────────
pub use lights::{GpuLight, LightArrayUniforms};

// ── Shadows ─────────────────────────────────────────────────────────────────
pub use shadow::{
    CascadeUniforms, CascadedShadowMap, PointShadowMap, ShadowAtlas, ShadowAtlasConfig, ShadowMap,
    ShadowPipeline, ShadowUniforms,
};

// ── Animation ───────────────────────────────────────────────────────────────
pub use animation::{AnimationClip, JointUniforms, Skeleton};

// ── Debug ───────────────────────────────────────────────────────────────────
pub use debug_draw::{LineBatch, LinePipeline, LineVertex};

// ── Post-processing ─────────────────────────────────────────────────────────
pub use hdr::{BloomUniforms, HdrFramebuffer};
pub use postprocess::{PostProcessPipeline, PostProcessUniforms, ToneMapMode};
pub use profiler::FrameProfiler;
pub use ssao::SsaoUniforms;

// ── Render targets ──────────────────────────────────────────────────────────
pub use render_target::RenderTarget;

// ── World ───────────────────────────────────────────────────────────────────
pub use terrain::{TerrainConfig, TerrainData};
pub use text::{BitmapFont, TextBatch};
pub use ui::{UiBatch, UiLabel, UiPanel};

// ── Legacy compat ───────────────────────────────────────────────────────────
pub use material::Material;
