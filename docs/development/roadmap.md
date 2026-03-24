# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

## V0.23.3 (current)

### Core Rendering
- Color types (RGBA, hex, lerp, wgpu conversion, prakash optics bridge)
- Vertex types (2D + 3D with bytemuck Pod/Zeroable and wgpu buffer layouts)
- Window + Surface — winit window + wgpu surface, resize handling, event loop
- GpuContext with surface-compatible adapter selection

### 2D Sprite Pipeline
- SpritePipeline — WGSL shader, orthographic projection, alpha blending
- Sprite rotation — CPU-side sin/cos around center, fast path for rotation=0
- UvRect — sprite atlas UV regions with from_pixel_rect()
- Batched rendering — multi-texture draw with consecutive texture grouping
- FrameStats — draw_calls, triangles, sprites counters
- SpriteBuffers — persistent GPU buffer reuse (zero per-frame allocation)
- u16 index path (MAX_SPRITES_PER_BATCH=16383) + u32 unlimited path
- batch_to_vertices_into() for zero-alloc vertex generation

### 3D Mesh Pipeline
- MeshPipeline — Vertex3D layout, depth buffer (Depth32Float), back-face culling
- mesh.wgsl — model/view_proj transform, Lambertian diffuse + ambient lighting
- CameraUniforms + LightUniforms (bytemuck Pod)
- Mesh struct with GPU vertex/index buffers (u32 indices)
- glTF loading — zero-copy buffer borrowing, embedded texture extraction
- Material — base_color texture + color factor + bind group

### Debug Rendering
- LinePipeline — LineList topology, depth-tested, renders on top
- LineBatch — wire_box, wire_circle, wire_sphere, wire_capsule, grid
- Optimized trig caching (50% wire_sphere speedup)

### Textures
- Texture — from_bytes (PNG/JPEG), from_color, from_rgba, white_pixel (all return Result)
- from_rgba_with_sampler() for shared sampler reuse
- create_default_sampler() — shared sampler factory
- TextureCache with get_or_load() lazy loading
- RenderTarget — offscreen framebuffer with read_pixels() GPU readback

### AGNOS Ecosystem
- hisab — math foundation (re-exports glam)
- prakash — spectral optics, color temperature (feature: optics)
- ranga — PixelBuffer texture loading (feature: ranga)
- impetus — ColliderShape debug wireframes: Box, Ball, Capsule, Segment (feature: physics-debug)

## V0.24 — PBR + Shadows

- [ ] Material uniform buffer (base_color_factor sent to shader)
- [ ] PBR materials (metallic-roughness workflow, Cook-Torrance from prakash)
- [ ] Shadow mapping (directional light)
- [ ] Use hisab transform types in pipeline code (replace hand-rolled ortho, raw [f32;16])

## V0.25 — Post-Processing + Animation

- [ ] Post-processing pipeline (bloom, tone mapping, SSAO)
- [ ] Skeletal animation (glTF skinned meshes)
- [ ] Point + spot light shadow mapping

## V0.26 — World Rendering

- [ ] Terrain rendering (heightmap or procedural)
- [ ] UI rendering (in-game HUD, menus)
- [ ] Text rendering (glyph atlas, monospace font)

## V1.0 — Production

- [ ] API stabilization + documentation pass
- [ ] Performance profiling + GPU timing queries
- [ ] Multi-window support
- [ ] WebGPU target validation

## Dependency Map

```
soorat (rendering engine)
  ├── wgpu         — GPU abstraction (Vulkan/Metal/DX12/WebGPU)
  ├── winit        — window management + event loop
  ├── hisab        — math (vectors, matrices, transforms)
  ├── bytemuck     — vertex type zero-copy casting
  ├── image        — texture loading (png, jpeg)
  ├── gltf         — 3D model loading
  ├── prakash      — optics (spectral color, PBR math)         [optional]
  ├── ranga        — image processing (pixel buffers)           [optional]
  └── impetus      — physics (collider debug wireframes)        [optional]
```

## Context for Agent

Soorat is consumed by:
- **kiran** (`src/gpu.rs`) — `SooratRenderer` implements kiran's `Renderer` trait
- **salai** (`src/viewport.rs`) — needs soorat 3D viewport for the editor

Current stats: 120 tests (full features), 21 benchmarks, 14 modules.
