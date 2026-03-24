# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.23.3] - 2026-03-23

### Added

#### Core
- `Window` struct — winit window + wgpu surface, `run()` event loop, resize handling.
- `Window::new_with_gpu()` — multi-window support sharing a `GpuContext`.
- `GpuContext::new_for_surface()` — surface-compatible adapter selection.
- `FrameProfiler` — CPU frame timing with EMA smoothing, FPS counter, per-pass timing.
- `GpuTimestamps` — wgpu timestamp queries for per-pass GPU timing (feature-detected).
- `PassTiming` — labeled duration for individual render passes.
- `GpuCapabilities` — adapter name, backend, feature/limit queries, `meets_requirements()`.
- `capabilities::webgpu` — WebGPU limit constants, `check_soorat_uniforms()` compliance test.
- `RenderError::Model` variant for glTF/model loading errors.
- `math_util` — shared `mul_mat4`, `normalize3`, `cross`, `look_at`, `perspective_90`, `IDENTITY_MAT4`.
- `Color`, `Vertex2D`, `Vertex3D`, `SkinnedVertex3D` re-exported from lib root.

#### 2D Sprites
- `SpritePipeline` — WGSL shader, orthographic projection, alpha blending.
- `SpritePipeline::draw_batched()` — multi-texture draw, one call per texture group.
- `SpritePipeline::draw_with_buffers()` — zero per-frame GPU allocation via `SpriteBuffers`.
- Sprite rotation — CPU-side sin/cos around center, fast path for rotation=0.
- `UvRect` — sprite atlas UV regions with `from_pixel_rect()`.
- `FrameStats` (draw_calls, triangles, sprites) returned from all draw methods.
- `batch_to_vertices_into()` — zero-allocation vertex generation for game loops.
- `batch_to_vertices_u32()` / `batch_to_vertices_u32_into()` — u32 index path, no sprite count limit.
- `MAX_SPRITES_PER_BATCH` (16383) — u16 overflow protection.

#### Textures
- `Texture` — `from_bytes` (PNG/JPEG), `from_color`, `from_rgba`, `white_pixel`.
- `Texture::from_rgba_with_sampler()` — shared sampler support.
- `create_default_sampler()` — sampler factory for reuse across textures.
- `TextureCache` — single HashMap with `get_or_load()` lazy loading.
- `RenderTarget` — offscreen framebuffer with `read_pixels()` GPU readback.

#### 3D PBR Rendering
- `MeshPipeline` — PBR shader (Cook-Torrance/GGX/Fresnel-Schlick ported from prakash).
- Multi-light PBR shader loop — up to 8 lights (directional, point, spot) per draw call.
- Point light range-based attenuation, spot light cone falloff.
- `CameraUniforms` — view_proj, model, camera_pos, inverse-transpose normal matrix.
- `CameraUniforms::set_model()` — auto-computes normal matrix for non-uniform scale.
- `LightUniforms`, `MaterialUniforms`, `ShadowPassUniforms`.
- `DepthBuffer` (Depth32Float) with resize support.
- `Mesh` struct — GPU vertex/index buffers (u32 indices).
- `Material` — base_color texture + color factor + bind group.
- BRDF LUT precomputation via `prakash::pbr::integrate_brdf_lut` (feature: `optics`).
- `MaterialUniforms::dielectric()`, `::metal()`, `::from_ior()` helpers.

#### Skinned Meshes + Normal Mapping
- `SkinnedVertex3D` — 96 bytes: position, normal, tex_coords, color, tangent (xyzw), joints (4×u32), weights (4×f32).
- `pbr_skinned.wgsl` — vertex skinning (4 joint weights from storage buffer) + tangent-space normal mapping via TBN matrix.

#### glTF Loading
- `gltf_loader::load_model()` — zero-copy buffer borrowing, embedded textures.
- `animation::load_gltf_animations()` — skins, joints, animation channels.

#### Shadows
- `ShadowMap` — depth texture + comparison sampler.
- `ShadowPipeline` — depth-only pass, front-face cull, depth bias for acne reduction.
- `directional_light_matrix()` — orthographic projection from light direction.
- PCF 3x3 soft shadows in PBR shader with dynamic texel size.
- `CascadedShadowMap` — 1–4 cascades with practical split scheme (Nvidia GPU Gems 3).
- `CascadeUniforms` — split distances + per-cascade view-projection matrices.
- `compute_practical_splits()` — standalone cascade split computation.
- `ShadowAtlas` — single large depth texture subdivided into tiles for multiple lights.
- `ShadowAtlasConfig`, `tile_viewport()`, `tile_uv()` — CPU-side atlas math.
- `PointShadowMap` — 6-face cube shadow maps for point lights.

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
- `PostProcessUniforms` — exposure, bloom intensity, tone map mode.
- `HdrFramebuffer` — Rgba16Float render target for linear HDR scene rendering.
- `BloomPipeline` — orchestrates threshold → horizontal blur → vertical blur (3 render passes).
- `BloomUniforms` — threshold, soft knee, intensity, texel size.
- `bloom.wgsl` — threshold extraction + separable 9-tap Gaussian blur (H/V passes).
- `SsaoPipeline` — depth + normal input → single-channel occlusion output.
- `SsaoUniforms` — radius, bias, intensity, sample count, projection matrices.
- `ssao.wgsl` — screen-space ambient occlusion with hemisphere sampling + PCF.

#### World / UI / Text
- `TerrainConfig` + `generate_terrain()` — heightmap → mesh with computed normals.
- `BitmapFont` + `TextBatch` — monospace glyph atlas, positioned sprite quads.
- `UiPanel`, `UiLabel`, `UiBatch` — screen-space overlay rendering.

#### Fluid Rendering
- `particles_to_quads()` — pravash `FluidParticle` → Vertex3D XZ-plane quads with color modes (Solid, Velocity, Density, Pressure).
- `particles_to_billboards()` — camera-facing billboard quads using camera right/up vectors.
- `shallow_water_to_mesh()` — pravash `ShallowWater` height field → mesh with computed normals.
- `visualization_heat_map()` — blue→cyan→green→yellow→red gradient for scalar data.
- Feature flag: `fluids` (dep: pravash).

#### LOD
- `LodChain` — distance-based mesh selection with squared distance comparison.
- `TerrainLod` — grid resolution selection by distance from camera.

#### Instanced Rendering
- `InstanceData` — 80 bytes: model matrix + color tint, vertex step mode = Instance.
- `InstanceBuffer` — GPU instance buffer with auto-grow on update.

#### Compute
- `ComputePipeline` — wraps wgpu compute pipeline from WGSL source.
- `create_storage_buffer()` / `create_storage_buffer_empty()` — GPU storage buffer helpers.

#### Ecosystem Integration
- Replaced glam with hisab (re-exports glam, adds transforms/projections).
- `Texture::from_pixel_buffer()` — ranga PixelBuffer support (feature: `ranga`).
- prakash PBR math ported to WGSL (feature: `optics`).
- Feature flags: `optics`, `ranga`, `physics-debug`, `fluids`, `full`.

### Changed
- `Texture::from_color()` and `white_pixel()` now return `Result`.
- `SpritePipeline::draw()` now returns `FrameStats`.
- PBR shader outputs linear HDR (tone mapping moved to PostProcessPipeline).
- `MeshPipeline` now uses `LightArrayUniforms` (multi-light) instead of single `LightUniforms`.
- `MeshPipeline::update_lights()` replaces `update_light()` (takes `LightArrayUniforms`).
- `MeshPipeline::draw()` now requires shadow bind group parameter.
- `CameraUniforms` extended with `camera_pos` + inverse-transpose normal matrix (192 bytes).
- `LightUniforms` extended with `light_view_proj` for shadow mapping (112 bytes).
- `LightUniforms` default direction normalized (`FRAC_1_SQRT_2`).
- `TextureCache` merged from two HashMaps into single HashMap.
- `Window::new()` delegates to `GpuContext::new()`.
- Orthographic projection simplified (constant terms folded).
- `math_util` made `pub(crate)`, `look_at`/`perspective_90` moved there from shadow.rs.
- API re-exports organized by category in lib.rs.

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
