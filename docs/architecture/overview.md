# Soorat Architecture

## Overview

Soorat is a wgpu-based GPU rendering engine for the Kiran game engine and AGNOS ecosystem. It provides 2D sprite rendering, 3D PBR mesh rendering, shadow mapping, skeletal animation, debug wireframes, post-processing, terrain, text, and UI rendering.

## Module Structure

```
src/
├── lib.rs              — crate root, organized re-exports by category
│
├── Core
│   ├── color.rs        — RGBA Color type (hex, lerp, wgpu/prakash conversion)
│   ├── vertex.rs       — Vertex2D, Vertex3D with wgpu buffer layouts
│   ├── error.rs        — RenderError enum (#[non_exhaustive])
│   ├── gpu.rs          — GpuContext (wgpu instance/adapter/device/queue)
│   ├── window.rs       — Window (winit + wgpu surface), WindowConfig, run()
│   ├── profiler.rs     — FrameProfiler (CPU timing, FPS)
│   └── math_util.rs    — shared mat4 multiply, normalize, cross (pub(crate))
│
├── 2D
│   ├── pipeline.rs     — SpritePipeline, SpriteBuffers, batch_to_vertices
│   ├── sprite.rs       — Sprite, SpriteBatch, UvRect
│   ├── sprite.wgsl     — 2D sprite vertex + fragment shader
│   ├── text.rs         — BitmapFont, TextBatch
│   └── ui.rs           — UiPanel, UiLabel, UiBatch
│
├── 3D
│   ├── mesh_pipeline.rs — MeshPipeline (PBR), CameraUniforms, LightUniforms, Mesh, DepthBuffer
│   ├── pbr.wgsl        — PBR fragment shader (Cook-Torrance/GGX/Fresnel + shadows)
│   ├── pbr_material.rs — MaterialUniforms, BRDF LUT generation
│   ├── material.rs     — Material (texture + color factor + bind group)
│   ├── shadow.rs       — ShadowMap, ShadowPipeline, directional_light_matrix
│   ├── shadow.wgsl     — depth-only shadow pass vertex shader
│   ├── lights.rs       — GpuLight (directional/point/spot), LightArrayUniforms
│   ├── animation.rs    — Skeleton, Joint, AnimationClip, JointUniforms
│   └── terrain.rs      — TerrainConfig, generate_terrain (heightmap → mesh)
│
├── Debug
│   ├── debug_draw.rs   — LinePipeline, LineBatch, LineVertex
│   └── line.wgsl       — debug line vertex + fragment shader
│
├── Post-Processing
│   ├── postprocess.rs  — PostProcessPipeline, PostProcessUniforms, ToneMapMode
│   └── postprocess.wgsl — full-screen tone mapping (Reinhard/ACES)
│
├── Loading
│   ├── gltf_loader.rs  — load_model, load_gltf_meshes (zero-copy GLB)
│   └── texture.rs      — Texture, TextureCache, create_default_sampler
│
└── Render Targets
    └── render_target.rs — RenderTarget (offscreen framebuffer, read_pixels)
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
| gltf | 3D model loading | yes |
| pollster | Async runtime for GPU init | yes |
| serde | Serialization (Color, Sprite, WindowConfig) | yes |
| thiserror | Error derive macros | yes |
| tracing | Structured logging | yes |
| prakash | Optics, PBR math, BRDF LUT | optional |
| ranga | Pixel buffer texture loading | optional |
| impetus | Physics collider debug wireframes | optional |

## Shader Pipeline

1. **Shadow pass** (`shadow.wgsl`) — depth-only from light perspective
2. **PBR pass** (`pbr.wgsl`) — Cook-Torrance specular + Lambert diffuse + shadow sampling
3. **Sprite pass** (`sprite.wgsl`) — 2D orthographic projection + texture sampling
4. **Debug pass** (`line.wgsl`) — wireframe lines on top (depth read, no write)
5. **Post-process pass** (`postprocess.wgsl`) — tone mapping (Reinhard/ACES)
