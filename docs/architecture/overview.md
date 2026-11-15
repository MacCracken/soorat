# Soorat Architecture

## Overview

Soorat is a wgpu-based GPU rendering engine for the Kiran game engine and AGNOS ecosystem. It provides 2D sprites, 3D PBR meshes with multi-light support, shadow mapping (single/cascaded/point), skeletal animation with vertex skinning, debug wireframes, post-processing (HDR/bloom/SSAO/tone mapping), terrain, text, UI, fluid rendering, and GPU profiling.

## Module Structure

```
src/
├── lib.rs                — crate root, organized re-exports by category
│
├── Core
│   ├── color.rs          — RGBA Color type (hex, lerp, wgpu/prakash conversion)
│   ├── vertex.rs         — Vertex2D, Vertex3D, SkinnedVertex3D with wgpu layouts
│   ├── error.rs          — RenderError enum (#[non_exhaustive])
│   ├── gpu.rs            — GpuContext (wgpu instance/adapter/device/queue)
│   ├── window.rs         — Window (winit + wgpu surface), WindowConfig, run(), multi-window
│   ├── profiler.rs       — FrameProfiler (CPU timing), GpuTimestamps (GPU queries), PassTiming
│   ├── capabilities.rs   — GpuCapabilities, WebGPU compliance checks
│   ├── math_util.rs      — mul_mat4, normalize3, cross, look_at, perspective_90 (pub(crate))
│   ├── viz.rs            — shared viz helpers (crate-private module): NaN-safe normalization, data-backed grid
│   │                       checks, +Y heatmap quads, height-field meshes, outward lattice indices
│   └── oracle.rs         — hidden `oracle` feature only: read-only hooks for the Cyrius port's golden-value
│                           generator (not public API; see docs/development/soorat-port-plan.md, D1)
│
├── 2D
│   ├── pipeline.rs       — SpritePipeline, SpriteBuffers, batch_to_vertices (u16/u32)
│   ├── sprite.rs         — Sprite, SpriteBatch, UvRect
│   ├── sprite.wgsl       — 2D sprite vertex + fragment shader
│   ├── text.rs           — BitmapFont, TextBatch
│   └── ui.rs             — UiPanel, UiLabel, UiBatch
│
├── 3D
│   ├── mesh_pipeline.rs  — MeshPipeline (PBR), CameraUniforms, LightUniforms, Mesh, DepthBuffer
│   ├── pbr.wgsl          — multi-light PBR (Cook-Torrance/GGX/Fresnel) + shadow sampling
│   ├── pbr_skinned.wgsl  — PBR + vertex skinning + normal mapping
│   ├── pbr_material.rs   — MaterialUniforms, BRDF LUT generation
│   ├── material.rs       — Material (texture + color factor + bind group)
│   ├── shadow.rs         — ShadowMap, ShadowPipeline, CascadedShadowMap, ShadowAtlas, PointShadowMap
│   ├── shadow.wgsl       — depth-only shadow pass vertex shader
│   ├── lights.rs         — GpuLight (directional/point/spot), LightArrayUniforms
│   ├── animation.rs      — Skeleton, Joint, AnimationClip, JointUniforms, glTF animation loading
│   ├── terrain.rs        — TerrainConfig, generate_terrain (heightmap → mesh)
│   └── fluid_render.rs   — SPH particle quads, shallow water mesh, heat map (feature: fluids)
│
├── Debug
│   ├── debug_draw.rs     — LinePipeline, LineBatch, LineVertex, collider wireframes
│   └── line.wgsl         — debug line vertex + fragment shader
│
├── Post-Processing
│   ├── postprocess.rs    — PostProcessPipeline, PostProcessUniforms, ToneMapMode
│   ├── postprocess.wgsl  — tone mapping (Reinhard/ACES) + bloom composite
│   ├── hdr.rs            — HdrFramebuffer (Rgba16Float), BloomUniforms
│   ├── bloom.wgsl        — threshold extraction + separable Gaussian blur
│   ├── ssao.rs           — SsaoUniforms
│   └── ssao.wgsl         — screen-space ambient occlusion
│
├── Loading
│   ├── gltf_loader.rs    — load_model, load_gltf_meshes (zero-copy GLB)
│   └── texture.rs        — Texture, TextureCache, create_default_sampler
│
└── Render Targets
    └── render_target.rs  — RenderTarget (offscreen framebuffer, read_pixels)
```

## Consumers

- **kiran** — game engine, `SooratRenderer` implements kiran's `Renderer` trait
- **salai** — game editor, uses soorat for 3D viewport rendering

## Dependencies

| Crate | Role | Required |
|---|---|---|
| wgpu | GPU abstraction (Vulkan/Metal/DX12/WebGPU) | yes |
| winit | Window management + event loop | yes |
| mabda | GPU foundation (context, pipelines, buffers, textures) — 1.0, final Rust release | yes |
| bytemuck | Zero-copy vertex type casting | yes |
| image | PNG/JPEG texture loading | yes |
| gltf | 3D model + animation loading | yes |
| pollster | Async runtime for GPU init | yes |
| serde | Serialization (Color, Sprite, WindowConfig) | yes |
| thiserror | Error derive macros | yes |
| tracing | Structured logging | yes |
| prakash | Optics, PBR math, BRDF LUT | optional (`optics`) |
| ranga | Pixel buffer texture loading | optional (`ranga`) |
| impetus | Physics collider debug wireframes | optional (`physics-debug`) |
| pravash | Fluid dynamics particle/surface rendering | optional (`fluids`) |
| goonj + hisab | Acoustic visualization (hisab only for goonj's `Vec3`) | optional (`acoustics`) |
| bijli | Electromagnetism visualization | optional (`em`) |
| ushma | Thermodynamics visualization | optional (`thermo`) |
| selah, uuid, chrono | Screenshot annotate/redact/clipboard | optional (`screenshot`) |
| agnosys | DRM device info (git tag, Linux) | optional (`drm`) |

wgpu is pinned to 29.x by mabda 1.0. See `docs/development/dependency-watch.md`.

## Conventions

- **Winding**: triangles are counter-clockwise around their normals (front
  faces); pipelines use back-face culling. Every mesh generator is checked by
  `viz::assert_winding_matches_normals` in tests.
- **Matrices**: column-major `[f32; 16]`; normal matrices are uploaded as the
  3 rows of the inverse-transpose (`dot(row, n)` in WGSL).
- **Depth**: wgpu `[0, 1]`; the directional light view is a regular
  (non-mirrored) view looking along the light direction.
- **Untrusted input**: see `docs/development/threat-model.md`.

## Shader Pipeline

1. **Shadow pass** (`shadow.wgsl`) — depth-only from light perspective (per cascade/face)
2. **PBR pass** (`pbr.wgsl` / `pbr_skinned.wgsl`) — multi-light Cook-Torrance + PCF shadow sampling (`textureSampleCompareLevel`, per-fragment perspective divide) + split-sum IBL
3. **Sprite pass** (`sprite.wgsl`) — 2D orthographic projection + texture sampling
4. **Debug pass** (`line.wgsl`) — wireframe lines on top (depth read, no write)
5. **SSAO pass** (`ssao.wgsl`) — depth-based ambient occlusion
6. **Bloom pass** (`bloom.wgsl`) — threshold → horizontal blur → vertical blur
7. **Post-process pass** (`postprocess.wgsl`) — tone mapping (Reinhard/ACES); the bloom-composite entry point exists but is not yet wired (roadmap)

## Testing

- Unit tests in every module; `tests/integration.rs` for cross-module CPU flows
- `tests/shaders.rs` — naga parse + validate of every WGSL file (no GPU needed)
- `tests/gpu_smoke.rs` — builds every pipeline and renders offscreen frames on a
  real adapter (skips when none is available)
- `tests/oracle.rs` (`--features oracle`) — each oracle hook agrees with the public
  path it backs, and calling the hooks changes no public output

## Stats

501 tests (all features, 18 of them oracle-hook tests), 35 benchmarks, 44 modules plus the hidden `oracle`, 9 WGSL shaders.
