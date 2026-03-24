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
│   └── math_util.rs      — mul_mat4, normalize3, cross, look_at, perspective_90 (pub(crate))
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
| hisab | Math (re-exports glam, transforms, projections) | yes |
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

## Shader Pipeline

1. **Shadow pass** (`shadow.wgsl`) — depth-only from light perspective (per cascade/face)
2. **PBR pass** (`pbr.wgsl` / `pbr_skinned.wgsl`) — multi-light Cook-Torrance + shadow sampling
3. **Sprite pass** (`sprite.wgsl`) — 2D orthographic projection + texture sampling
4. **Debug pass** (`line.wgsl`) — wireframe lines on top (depth read, no write)
5. **SSAO pass** (`ssao.wgsl`) — depth-based ambient occlusion
6. **Bloom pass** (`bloom.wgsl`) — threshold → horizontal blur → vertical blur
7. **Post-process pass** (`postprocess.wgsl`) — bloom composite + tone mapping (Reinhard/ACES)

## Stats

236 tests, 28 benchmarks, 31 modules, 8 WGSL shaders.
