# Changelog

All notable changes to this project will be documented in this file.

## [0.23.3] - 2026-03-23

### Added

#### Core
- `Window` struct — winit window + wgpu surface, `run()` event loop, resize handling.
- `Window::new_with_gpu()` — multi-window support sharing a `GpuContext`.
- `GpuContext::new_for_surface()` — surface-compatible adapter selection.
- `FrameProfiler` — CPU frame timing with EMA smoothing and FPS counter.
- `RenderError::Model` variant for glTF/model loading errors.
- `Color`, `Vertex2D`, `Vertex3D` re-exported from lib root.

#### 2D Sprites
- `SpritePipeline` — WGSL shader, orthographic projection, alpha blending.
- `SpritePipeline::draw_batched()` — multi-texture draw, one call per texture group.
- `SpritePipeline::draw_with_buffers()` — zero per-frame GPU allocation via `SpriteBuffers`.
- Sprite rotation — CPU-side sin/cos around center, fast path for rotation=0.
- `UvRect` — sprite atlas UV regions with `from_pixel_rect()`.
- `FrameStats` (draw_calls, triangles, sprites) returned from all draw methods.
- `batch_to_vertices_into()` — zero-allocation vertex generation for game loops.
- `batch_to_vertices_u32()` — u32 index path, no sprite count limit.
- `MAX_SPRITES_PER_BATCH` (16383) — u16 overflow protection.

#### Textures
- `Texture` — `from_bytes` (PNG/JPEG), `from_color`, `from_rgba`, `white_pixel`.
- `Texture::from_rgba_with_sampler()` — shared sampler support.
- `create_default_sampler()` — sampler factory for reuse across textures.
- `TextureCache` — single HashMap with `get_or_load()` lazy loading.
- `RenderTarget` — offscreen framebuffer with `read_pixels()` GPU readback.

#### 3D PBR Rendering
- `MeshPipeline` — PBR shader (Cook-Torrance/GGX/Fresnel-Schlick ported from prakash).
- `CameraUniforms` (view_proj + model + camera_pos), `LightUniforms`, `MaterialUniforms`.
- `DepthBuffer` (Depth32Float) with resize support.
- `Mesh` struct — GPU vertex/index buffers (u32 indices).
- `Material` — base_color texture + color factor + bind group.
- BRDF LUT precomputation via `prakash::pbr::integrate_brdf_lut` (feature: `optics`).
- `MaterialUniforms::dielectric()`, `::metal()`, `::from_ior()` helpers.

#### glTF Loading
- `gltf_loader::load_model()` — zero-copy buffer borrowing, embedded textures.
- `animation::load_gltf_animations()` — skins, joints, animation channels.

#### Shadows
- `ShadowMap` — depth texture + comparison sampler.
- `ShadowPipeline` — depth-only pass, front-face cull, depth bias for acne reduction.
- `directional_light_matrix()` — orthographic projection from light direction.
- PCF 3x3 soft shadows in PBR shader with dynamic texel size.

#### Lighting
- `GpuLight` — directional, point, and spot light types.
- `LightArrayUniforms` — up to 8 lights with ambient color.

#### Animation
- `Skeleton` — joint hierarchy with inverse bind matrices.
- `Joint` — TRS local transform + parent index.
- `AnimationClip` — channels with keyframe interpolation (translation, rotation, scale).
- `JointUniforms` — 128 joints max, bytemuck Pod for GPU upload.

#### Debug Rendering
- `LinePipeline` — LineList topology, depth-tested (LessEqual, no write).
- `LineBatch` — `line()`, `wire_box()`, `wire_circle()`, `wire_sphere()`, `wire_capsule()`, `grid()`.
- `LineBatch::collider()` — impetus ColliderShape wireframes (feature: `physics-debug`).
- Optimized trig caching in circle/sphere (50% speedup).

#### Post-Processing
- `PostProcessPipeline` — full-screen triangle (no vertex buffer needed).
- Reinhard + ACES filmic tone mapping in WGSL.
- `PostProcessUniforms` — exposure, tone map mode.

#### World / UI / Text
- `TerrainConfig` + `generate_terrain()` — heightmap → mesh with computed normals.
- `BitmapFont` + `TextBatch` — monospace glyph atlas, positioned sprite quads.
- `UiPanel`, `UiLabel`, `UiBatch` — screen-space overlay rendering.

#### Ecosystem Integration
- Replaced glam with hisab (re-exports glam, adds transforms/projections).
- `Texture::from_pixel_buffer()` — ranga PixelBuffer support (feature: `ranga`).
- prakash PBR math ported to WGSL (feature: `optics`).
- Feature flags: `optics`, `ranga`, `physics-debug`, `full`.

### Changed
- `Texture::from_color()` and `white_pixel()` now return `Result`.
- `SpritePipeline::draw()` now returns `FrameStats`.
- PBR shader outputs linear HDR (tone mapping moved to PostProcessPipeline).
- `LightUniforms` extended with `light_view_proj` for shadow mapping.
- `CameraUniforms` extended with `camera_pos` for PBR view vector.
- `LightUniforms` default direction normalized (FRAC_1_SQRT_2).
- `TextureCache` merged from two HashMaps into single HashMap.
- `Window::new()` delegates to `GpuContext::new()`.
- Orthographic projection simplified (constant terms folded).
- Shared `math_util` module (deduplicated mul_mat4/normalize3/cross).
- API re-exports organized by category in lib.rs.
- `math_util` made `pub(crate)` (internal only).

## [0.1.0] - 2026-03-23

### Added
- `Color` type with constants, hex/rgba8 constructors, lerp, wgpu conversion.
- `Vertex2D` and `Vertex3D` with bytemuck Pod/Zeroable and wgpu buffer layouts.
- `Sprite` with builder API (position, size, color, rotation, texture, z-order).
- `SpriteBatch` for batched sprite rendering with z-sort.
- `GpuContext` for wgpu adapter/device/queue management.
- `WindowConfig` with winit integration (vsync, fullscreen, present mode).
- `RenderError` enum with `#[non_exhaustive]`.
- Criterion benchmark scaffold.
