# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.30.0] - 2026-03-29

### Added

#### Electromagnetism Visualization (bijli integration)
- `em` feature — optional dep on bijli for electromagnetic field visualization.
- `EmColorMode` enum (Magnitude, Signed) with `#[non_exhaustive]`.
- `EmVisParams` — color mode, base color, alpha configuration.
- `field_slice_2d_to_mesh()` — render `FieldSlice2D` (FDTD 2D) as colored quad heatmap on XZ plane.
- `field_slice_3d_to_mesh()` — render `FieldSlice3D` (FDTD 3D) Z-slice as colored quad heatmap.
- `field_lines_to_lines()` — draw `FieldLineVisualization` as magnitude-colored polylines.
- `charges_to_lines()` — draw `ChargeVisualization` as wireframe spheres (red=positive, blue=negative, sized by magnitude).
- `radiation_pattern_to_mesh()` — render `RadiationPattern` as 3D polar balloon with gain-based radius deformation.
- `vector_field_to_arrows()` — render `VectorFieldSample` as arrow glyphs with 4-line arrowheads.
- `em_heat_map()` — public heat map color helper for EM data.

#### Thermodynamics Visualization (ushma integration)
- `thermo` feature — optional dep on ushma for thermal visualization.
- `ThermalColorMode` enum (Temperature, AlphaBlend) with `#[non_exhaustive]`.
- `ThermalVisParams` — color mode, base color, alpha configuration.
- `CycleDiagramParams` — origin, scale, T-s/P-v colors.
- `thermal_grid_to_mesh()` — render `ThermalGridVisualization` as colored quad heatmap with temperature or alpha-blend coloring.
- `temperature_profile_to_lines()` — draw `TemperatureProfile` as heat-mapped line strip along a 3D direction.
- `cycle_diagram_to_lines()` — draw `CycleDiagramData` T-s and P-v diagrams as normalized colored line plots.
- `thermal_network_to_lines()` — draw `ThermalNetworkVisualization` as circular node-link diagram with temperature-colored nodes.
- `heat_flux_to_arrows()` — draw `HeatFluxField` as arrow glyphs on the XZ plane.
- `thermal_heat_map()` — public heat map color helper for thermal data.

### Changed

#### Dependencies
- `bijli` 1 added as optional dep (em feature, soorat-compat).
- `ushma` 1 added as optional dep (thermo feature, soorat-compat).
- `full` feature now includes `em` and `thermo`.

## [0.24.3] - 2026-03-25

### Added

#### Screenshot (selah integration)
- `screenshot` feature — optional dep on selah for capture, annotation, and redaction.
- `ScreenshotFormat` enum (Png, Jpeg, Bmp) with `#[non_exhaustive]`.
- `encode_pixels()` — encode raw RGBA8 GPU readback to image format (JPEG strips alpha to RGB).
- `capture_render_target()` — blocking GPU readback + encode in one call.
- `save_to_file()` — write encoded bytes to disk.
- `capture_screenshot()` — full pipeline to `selah::Screenshot` with metadata.
- `capture_screenshot_region()` — sub-rectangle capture with pixel-coordinate cropping.
- `annotate_capture()` — bridge to `selah::annotate_image()`.
- `redact_capture()` — bridge to `selah::redact_image()` with PII detection.
- `copy_to_clipboard()` — clipboard via selah (Wayland/X11).
- `to_selah_format()` — format conversion helper.
- `RenderError::Screenshot` variant.

#### Error Handling
- `RenderError::Screenshot` variant for screenshot/encoding failures.

### Changed

#### Dependencies
- `prakash` bumped from 0.23 to **1.0** (stable API).
- `pravash` moved from path dep to **crates.io 0.24** (publish unblocked).
- `selah` 0.24 added as optional dep (screenshot feature).
- `uuid` 1 + `chrono` 0.4 added as optional deps (screenshot feature).

#### Audit Hardening
- **Eliminated all `unwrap()`/`panic!()` in library code** — 7 critical sites fixed:
  - window.rs: `App::resumed()`, surface format/alpha mode selection.
  - profiler.rs: GPU timestamp channel send/recv.
  - animation.rs: keyframe last() access.
  - texture.rs: `TextureCache::get_or_load()`.
  - gpu_particles.rs: particle readback channel send/recv.
- **Cycle detection** in render graph topological sort — prevents infinite recursion on circular pass dependencies.
- **Division-by-zero guards** in `perspective_90()`, `directional_light_matrix()`, `generate_terrain()`.
- **Safe glTF attribute indexing** — `normals[i]`/`tex_coords[i]` use `.get(i)` with fallback defaults.
- **Integer overflow protection** — `checked_mul()`/`saturating_mul()` in buffer size calculations across batch, pipeline, render_target, screenshot, and fluid_render modules.
- **Zero-size validation** — textures reject 0×0 with error, render targets clamp to 1×1 with warning, projection skips zero dimensions.
- **Texture buffer validation** — `from_rgba()` checks `rgba.len() == width * height * 4`.
- **Region bounds overflow** — `capture_screenshot_region()` uses `checked_add()`.

#### Attributes Sweep
- `#[must_use]` added to ~40 pure functions across color, math_util, sprite, texture, batch, pipeline, mesh_pipeline, pbr_material, lights, shadow, hdr, ssao, animation.
- `#[inline]` added to ~25 hot-path functions across color, math_util, sprite, texture, batch, pipeline, lights.
- `#[non_exhaustive]` added to `LightType`, `ToneMapMode`, `PassType`, `FluidColorMode` enums.

#### Performance
- `Color::new()` adds `debug_assert!` for finite components (NaN/Infinity caught in debug builds).
- `InstanceBuffer::update()` uses 1.5× exponential growth instead of exact-fit (fewer GPU reallocations).
- `GpuCapabilities` backend field uses static string match instead of `format!("{:?}")`.
- `interpolate_keyframes()` returns `Cow<[f32]>` — zero-alloc on boundary cases (before first, after last, single keyframe). Also guards against zero-length keyframe intervals.

#### Observability
- `tracing::debug!` spans added to: texture loading, texture cache insert/hit, mesh creation, render target creation, zero-size resize skip.
- `tracing::warn` on: degenerate perspective/shadow matrices, zero-size terrain, render graph cycles, zero-dimension render targets.
- GPU timestamp wraparound documented (64-bit, won't wrap in practice).
- `ComputePipeline::new()` documents buffer 0 read-write / buffers 1+ read-only convention.
- `Window` struct documents thread safety constraints (not `Send`/`Sync`, must use on event-loop thread).

#### Configuration
- `deny.toml` migrated to cargo-deny 0.19 format.
- `RUSTSEC-2024-0436` (paste, unmaintained) ignored — transitive via wgpu/image.
- `CC0-1.0` and `AGPL-3.0` licenses added to allow list.

### Added
- `FrameProfiler::with_alpha()` — configurable EMA smoothing factor (default 0.05, range 0.001–1.0).

### Fixed
- JPEG encoding now strips alpha channel (RGBA→RGB) instead of failing with unsupported color type error.
- `from_rgba8()` roundtrip precision verified with boundary value tests.

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
