# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

## V0.23.3 — Foundation

- Color types (RGBA, hex, lerp, wgpu conversion, prakash optics bridge)
- Vertex types (2D + 3D with bytemuck Pod/Zeroable and wgpu buffer layouts)
- Window + Surface — winit window + wgpu surface, resize handling, event loop
- GpuContext with surface-compatible adapter selection
- SpritePipeline — WGSL shader, orthographic projection, alpha blending
- Sprite rotation, UvRect atlas regions, batched multi-texture draw
- FrameStats, SpriteBuffers (persistent GPU buffer reuse), u16 + u32 index paths
- Texture — from_bytes, from_color, from_rgba, shared sampler, TextureCache with get_or_load
- RenderTarget — offscreen framebuffer with read_pixels GPU readback

## V0.24 — PBR Rendering

- MeshPipeline with PBR shader (Cook-Torrance/GGX/Fresnel-Schlick from prakash)
- CameraUniforms (view_proj + model + camera_pos), LightUniforms, MaterialUniforms
- BRDF LUT precomputation via prakash::pbr::integrate_brdf_lut
- DepthBuffer (Depth32Float), Mesh (GPU vertex/index buffers), Material
- glTF loading — zero-copy buffer borrowing, embedded textures

## V0.25 — Shadows, Lights, Animation, Post-Processing

- ShadowMap + ShadowPipeline — depth-only pass, front-face cull, depth bias, PCF 3x3
- PBR shader shadow integration — light_view_proj, shadow_coords, comparison sampling
- Multi-light system — GpuLight (directional/point/spot), LightArrayUniforms (8 max)
- Skeletal animation — Skeleton, Joint (TRS + inverse bind), AnimationClip, keyframe interpolation
- glTF animation loading — skins, joints, channels
- PostProcessPipeline — full-screen triangle, Reinhard + ACES filmic tone mapping
- math_util — shared mul_mat4, normalize3, cross (deduplicated from shadow + animation)

## V0.26 — World, Text, UI

- Terrain — heightmap mesh generation with computed normals, centered origin, UV mapping
- Text — BitmapFont glyph atlas, TextBatch (positioned sprite quads)
- UI — UiPanel, UiLabel, UiBatch (screen-space overlay via SpritePipeline)

## V1.0 — Production

- API stabilization — organized re-exports, module docs, math_util made pub(crate)
- FrameProfiler — CPU frame timing, EMA smoothing, FPS counter
- Multi-window — Window::new_with_gpu() shares GpuContext across windows
- Debug draw — LinePipeline, wire_box/circle/sphere/capsule/grid, impetus colliders

## AGNOS Ecosystem Integration

- hisab — math foundation (re-exports glam)
- prakash — spectral optics, PBR math, BRDF LUT (feature: optics)
- ranga — PixelBuffer texture loading (feature: ranga)
- impetus — ColliderShape debug wireframes (feature: physics-debug)

## Dependency Map

```
soorat (rendering engine)
  ├── wgpu         — GPU abstraction (Vulkan/Metal/DX12/WebGPU)
  ├── winit        — window management + event loop
  ├── hisab        — math (vectors, matrices, transforms)
  ├── bytemuck     — vertex type zero-copy casting
  ├── image        — texture loading (png, jpeg)
  ├── gltf         — 3D model loading
  ├── prakash      — optics, PBR math                          [optional]
  ├── ranga        — image processing (pixel buffers)           [optional]
  └── impetus      — physics (collider debug wireframes)        [optional]
```

## Stats

188 tests, 21 benchmarks, 25 modules, clippy clean, fmt clean.

## Context for Agent

Soorat is consumed by:
- **kiran** (`src/gpu.rs`) — `SooratRenderer` implements kiran's `Renderer` trait
- **salai** (`src/viewport.rs`) — needs soorat 3D viewport for the editor
