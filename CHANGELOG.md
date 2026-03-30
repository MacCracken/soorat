# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

#### Correctness (Sprint 1)
- **Eliminated all `expect()`/`assert_eq!()` in library code** — 15 panic sites across 8 files converted to `Result` propagation.
- **PBR specular denominator** — clamped entire `4.0 * NdotV * NdotL` denominator to prevent specular spikes at grazing angles.
- **Shadow coord z check** — objects behind shadow light now correctly marked as lit (`coords.z < 0.0`).
- **Animation parent-joint logic** — broken `any(|_| false)` replaced with correct parent search in glTF skeleton loading.
- **`from_ior()` dead parameter** — computed Fresnel reflectance (F0) now stored for shader access.
- **`flatten_mat4` doc** — corrected to reflect column-major flatten (no transpose).
- **Division-by-zero guards** — `UvRect::from_pixel_rect`, `BloomUniforms::new`, `BitmapFont::glyph_uv`, cascade splits, terrain generation.
- **Checked integer casts** — `batch.rs` u16/u32 indices, `mesh_pipeline.rs` index count, `terrain.rs` saturating_add.
- **NaN guard** — `vector_field_to_arrows` now skips non-finite magnitudes.

#### Code Quality (Sprint 2)
- **Extracted shared viz helpers** — `visualization_heat_map()`, `signed_value_color()`, `normal_to_basis()` deduplicated from 4→1 copies.
- **TextureCache triple lookup** — replaced `contains_key`×2 + `get` with single `entry()` call.
- **Error chain preservation** — `From<mabda::GpuError>` now uses `format!("{other:#}")` for full context.
- **gltf_loader label allocation** — `format!()` in loop replaced with reusable `write!` buffer.

### Changed

#### Attributes (Sprint 2)
- `#[must_use]` added to ~50 pure functions across 17 files.
- `#[inline]` added to ~15 hot-path functions.
- `#[non_exhaustive]` added to `AnimationProperty` enum.
- `#[deprecated]` added to `Material` re-export (use `MaterialUniforms` for PBR).

#### Observability (Sprint 2)
- `tracing::debug!` spans added to 22 functions across 12 files (all pipelines, glTF loading, screenshot, DRM, particles, render graph).

### Added

#### Tests (Sprint 3)
- 34 new tests (374 → 408): math edge cases (9), div-by-zero regression (6), error conversion (5), render graph cycles (1), animation crash (3), primitive zero-input (3), feature-gated adversarial (5), additional edge cases (2).

## [1.0.0] - 2026-03-29

### Added

#### Core
- `Window` struct — winit window + wgpu surface, `run()` event loop, resize handling.
- `Window::new_with_gpu()` — multi-window support sharing a `GpuContext`.
- `GpuContext` / `GpuContextBuilder` — GPU device lifecycle delegated to mabda.
- `FrameProfiler` — CPU frame timing with EMA smoothing, FPS counter, per-pass timing.
- `FrameProfiler::with_alpha()` — configurable EMA smoothing factor.
- `GpuTimestamps` — wgpu timestamp queries for per-pass GPU timing (feature-detected).
- `GpuCapabilities` — adapter name, backend, feature/limit queries, `meets_requirements()`.
- `RenderError` enum with `#[non_exhaustive]` — Model, Screenshot variants.
- `Color` type with constants, hex/rgba8 constructors, lerp, wgpu conversion.
- `Vertex2D`, `Vertex3D`, `SkinnedVertex3D` — bytemuck Pod/Zeroable vertex types.
- `math_util` — shared matrix/vector operations (pub(crate)).

#### 2D Sprites
- `SpritePipeline` — WGSL shader, orthographic projection, alpha blending.
- `SpritePipeline::draw_batched()` — multi-texture draw, one call per texture group.
- `SpritePipeline::draw_with_buffers()` — zero per-frame GPU allocation via `SpriteBuffers`.
- `Sprite` with builder API, `SpriteBatch` with z-sort, `UvRect` for atlas regions.
- `FrameStats` (draw_calls, triangles, sprites) returned from all draw methods.
- `batch_to_vertices_u32()` — u32 index path, no sprite count limit.

#### Textures
- `Texture` — `from_bytes` (PNG/JPEG), `from_color`, `from_rgba`, `white_pixel`.
- `TextureCache` — single HashMap with `get_or_load()` lazy loading.
- `RenderTarget` / `RenderTargetBuilder` — offscreen framebuffer with `read_pixels()`.

#### 3D PBR Rendering
- `MeshPipeline` — PBR shader (Cook-Torrance/GGX/Fresnel-Schlick via prakash).
- Multi-light PBR — up to 8 lights (directional, point, spot) per draw call.
- `CameraUniforms` — view_proj, model, camera_pos, inverse-transpose normal matrix.
- `Mesh`, `Material`, `MaterialUniforms` — GPU vertex/index buffers, PBR material helpers.
- `SkinnedVertex3D` + `pbr_skinned.wgsl` — vertex skinning + tangent-space normal mapping.
- BRDF LUT precomputation via prakash (feature: `optics`).

#### glTF Loading
- `gltf_loader::load_model()` — zero-copy buffer borrowing, embedded textures.
- `animation::load_gltf_animations()` — skins, joints, animation channels.

#### Shadows
- `ShadowMap` + `ShadowPipeline` — depth-only pass, PCF 3x3 soft shadows.
- `CascadedShadowMap` — 1–4 cascades with practical split scheme.
- `ShadowAtlas` — single depth texture subdivided into tiles for multiple lights.
- `PointShadowMap` — 6-face cube shadow maps for point lights.

#### Lighting
- `GpuLight` — directional, point, and spot light types.
- `LightArrayUniforms` — up to 8 lights with ambient color.
- `EnvironmentMap` + `IblBindGroup` — image-based lighting.

#### Animation
- `Skeleton`, `Joint`, `AnimationClip`, `JointUniforms` — 128-joint skinned animation.

#### Debug Rendering
- `LinePipeline` + `LineBatch` — wireframe lines, shapes, grid, collider outlines.

#### Post-Processing
- `PostProcessPipeline` — Reinhard + ACES filmic tone mapping.
- `HdrFramebuffer` — Rgba16Float render target for linear HDR.
- `BloomPipeline` — threshold → separable 9-tap Gaussian blur (H/V passes).
- `SsaoPipeline` — screen-space ambient occlusion with hemisphere sampling.

#### World / UI / Text
- `TerrainConfig` + `generate_terrain()` — heightmap → mesh with computed normals.
- `BitmapFont` + `TextBatch` — monospace glyph atlas.
- `UiPanel`, `UiLabel`, `UiBatch` — screen-space overlay rendering.

#### Instanced Rendering + Compute + LOD
- `InstanceBuffer` — GPU instance buffer with 1.5× exponential growth.
- `ComputePipeline` + `PingPongBuffer` — compute dispatch utilities.
- `GpuParticleSystem` — GPU-driven particle simulation.
- `LodChain`, `TerrainLod` — distance-based mesh/resolution selection.
- `RenderGraph` — topological pass ordering with cycle detection.

#### Fluid Rendering (pravash integration)
- `particles_to_quads()`, `particles_to_billboards()`, `shallow_water_to_mesh()`.
- `FluidColorMode` — Solid, Velocity, Density, Pressure coloring.
- Feature flag: `fluids`.

#### Acoustic Visualization (goonj integration)
- `ray_paths_to_lines()` — acoustic ray paths with energy-based color fading.
- `pressure_map_slice()` — XZ-plane pressure heatmaps.
- `mode_pattern_to_mesh()` — standing wave height-field meshes.
- `portal_to_lines()` — wireframe portals with normal arrows.
- `directivity_balloon_to_mesh()` — gain-deformed sphere meshes.
- `coupled_decay_to_lines()` — double-slope energy decay curves.
- Feature flag: `acoustics`.

#### Electromagnetism Visualization (bijli integration)
- `field_slice_2d_to_mesh()` / `field_slice_3d_to_mesh()` — FDTD field heatmaps.
- `field_lines_to_lines()` — magnitude-colored field line polylines.
- `charges_to_lines()` — wireframe charge spheres (red=positive, blue=negative).
- `radiation_pattern_to_mesh()` — far-field polar balloon with gain-based deformation.
- `vector_field_to_arrows()` — arrow glyphs with 4-line arrowheads.
- Feature flag: `em`.

#### Thermodynamics Visualization (ushma integration)
- `thermal_grid_to_mesh()` — temperature heatmaps with temperature or alpha-blend coloring.
- `temperature_profile_to_lines()` — heat-mapped 1D profile line strips.
- `cycle_diagram_to_lines()` — T-s and P-v cycle diagrams.
- `thermal_network_to_lines()` — circular node-link diagrams.
- `heat_flux_to_arrows()` — 2D heat flux arrow glyphs.
- Feature flag: `thermo`.

#### Screenshot (selah integration)
- `capture_screenshot()`, `capture_screenshot_region()` — GPU readback to `selah::Screenshot`.
- `annotate_capture()`, `redact_capture()` — annotation and PII redaction bridges.
- `copy_to_clipboard()` — Wayland/X11 clipboard via selah.
- Feature flag: `screenshot`.

#### Integration
- `egui_bridge` — salai editor viewport integration.
- `Texture::from_pixel_buffer()` — ranga PixelBuffer support (feature: `ranga`).
- `LineBatch::collider()` — impetus ColliderShape wireframes (feature: `physics-debug`).

#### Safety & Correctness
- Zero `unwrap()`/`panic!()` in library code — all fallible paths return `Result`.
- Integer overflow protection via `checked_mul()`/`saturating_mul()` in buffer calculations.
- Zero-size validation on textures, render targets, projections.
- Cycle detection in render graph topological sort.
- Division-by-zero guards on all projection/transform functions.

#### Attributes
- `#[must_use]` on all pure functions.
- `#[inline]` on hot-path functions.
- `#[non_exhaustive]` on all public enums.

#### Observability
- `tracing` spans on all GPU resource creation and cache operations.
- Warnings on degenerate matrices, zero-size resources, graph cycles.

## [0.24.3] - 2026-03-25

Initial tagged release — core rendering, PBR pipeline, shadows, post-processing,
fluid/acoustic visualization, screenshot capture, audit hardening. See git history
for detailed pre-release changes.
