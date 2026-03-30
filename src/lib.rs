//! Soorat — GPU rendering engine for AGNOS
//!
//! **Soorat** (Arabic/Urdu: صورت — form, image, appearance) is a wgpu-based
//! rendering engine designed for the Kiran game engine and AGNOS ecosystem.
//!
//! # Modules
//!
//! - **Core**: [`gpu`], [`window`], [`color`], [`vertex`], [`error`], [`profiler`], [`capabilities`]
//! - **2D**: [`pipeline`] (sprites), [`sprite`], [`texture`], [`text`], [`ui`]
//! - **3D**: [`mesh_pipeline`] (PBR), [`shadow`], [`animation`], [`terrain`], [`primitives`]
//! - **Lighting**: [`lights`], [`environment`] (IBL)
//! - **Post-processing**: [`postprocess`], [`hdr`] (bloom), [`ssao`]
//! - **Debug**: [`debug_draw`] (wireframe lines, shapes, grid)
//! - **Rendering**: [`instancing`], [`lod`], [`compute`], [`gpu_particles`], [`render_graph`]
//! - **Fluids**: [`fluid_render`] (pravash integration)
//! - **Acoustics**: [`acoustic_render`] (goonj integration — ray paths, pressure maps, directivity)
//! - **Screenshot**: [`screenshot`] (selah integration — capture, annotate, redact)
//! - **Loading**: [`gltf_loader`], [`texture`]
//! - **Integration**: [`egui_bridge`] (salai editor)

pub mod acoustic_render;
pub mod animation;
pub mod batch;
pub mod capabilities;
pub mod color;
pub mod compute;
pub mod debug_draw;
pub mod drm;
pub mod egui_bridge;
pub mod environment;
pub mod error;
pub mod fluid_render;
pub mod gltf_loader;
pub mod gpu;
pub mod gpu_particles;
pub mod hdr;
pub mod instancing;
pub mod lights;
pub mod lod;
pub mod material;
pub(crate) mod math_util;
pub mod mesh_pipeline;
pub mod pbr_material;
pub mod pipeline;
pub mod postprocess;
pub mod primitives;
pub mod profiler;
pub mod render_graph;
pub mod render_target;
pub mod screenshot;
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
pub use capabilities::{GpuCapabilities, SooratCapabilities};
pub use color::Color;
pub use error::{RenderError, Result};
pub use gpu::{GpuContext, GpuContextBuilder};
pub use profiler::{FrameProfiler, GpuTimestamps, PassTiming, ProfileScope};
pub use vertex::{SkinnedVertex3D, Vertex2D, Vertex3D};
pub use window::{Window, WindowConfig};

// ── Rendering ───────────────────────────────────────────────────────────────
pub use compute::{
    ComputePipeline, PingPongBuffer, validate_dispatch, workgroups_1d, workgroups_2d,
};
pub use gpu_particles::{GpuParticle, GpuParticleSystem, SimParams};
pub use instancing::{InstanceBuffer, InstanceData};
pub use lod::{LodChain, TerrainLod};
pub use render_graph::{PassType, RenderGraph};

// ── Lighting + Environment ──────────────────────────────────────────────────
pub use environment::{EnvironmentMap, IblBindGroup};
pub use fluid_render::{FluidColorMode, ParticleColorParams};

// ── Acoustics (goonj integration) ──────────────────────────────────────────
pub use acoustic_render::{AcousticColorMode, AcousticVisParams, DecayCurveParams};

// ── Integration ─────────────────────────────────────────────────────────────
pub use egui_bridge::ViewportSize;

// ── 2D Sprites ──────────────────────────────────────────────────────────────
pub use pipeline::{
    FrameStats, SpriteBuffers, SpritePipeline, batch_to_vertices, batch_to_vertices_into,
    batch_to_vertices_u32,
};
pub use sprite::{Sprite, SpriteBatch, UvRect};
pub use texture::{
    CubemapTexture, Texture, TextureCache, copy_texture_to_texture, create_default_sampler,
    mip_level_count, validate_dimensions,
};

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
pub use hdr::{BloomPipeline, BloomUniforms, HdrFramebuffer};
pub use postprocess::{PostProcessPipeline, PostProcessUniforms, ToneMapMode};
pub use ssao::{SsaoPipeline, SsaoUniforms};

// ── Render targets ──────────────────────────────────────────────────────────
pub use render_target::{RenderTarget, RenderTargetBuilder};

// ── Screenshot (selah integration) ─────────────────────────────────────────
pub use screenshot::{ScreenshotFormat, capture_render_target, encode_pixels, save_to_file};

// ── World ───────────────────────────────────────────────────────────────────
pub use terrain::{TerrainConfig, TerrainData};
pub use text::{BitmapFont, TextBatch};
pub use ui::{UiBatch, UiLabel, UiPanel};

// ── Materials ───────────────────────────────────────────────────────────────
/// Deprecated: use `MaterialUniforms` for PBR. Retained for simple texture+bind group use.
pub use material::Material;
