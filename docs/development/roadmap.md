# Soorat Roadmap

> GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

## V0.1 — Scaffold (done, 2026-03-23)

- Color types (RGBA, hex, lerp, wgpu conversion)
- Vertex types (2D + 3D with wgpu layouts)
- Sprite instances with builder + SpriteBatch with z-sort
- GpuContext (wgpu adapter/device/queue)
- WindowConfig (winit integration)
- RenderError with non_exhaustive
- Project scaffold (CI, benchmarks, deny, Makefile)

## V0.2 — Sprite Rendering Pipeline

- [ ] Sprite render pipeline (shader + pipeline + bind groups)
- [ ] Texture loading and management (TextureAtlas)
- [ ] Sprite batch GPU upload and draw
- [ ] Clear color / background
- [ ] Window event loop integration
- [ ] Resize handling

## V0.3 — 3D Mesh Rendering

- [ ] Mesh pipeline (vertex + index buffers)
- [ ] Camera uniform buffer
- [ ] Basic lighting (ambient + directional)
- [ ] glTF model loading
- [ ] Depth buffer

## V0.4 — Debug Rendering

- [ ] Wireframe line rendering
- [ ] Debug shapes (circles, boxes, capsules) from kiran DebugShape
- [ ] Grid overlay
- [ ] Text rendering (basic glyph atlas)

## V0.5 — AGNOS Shared Crate Integration

### hisab (math)
- [ ] Replace glam with hisab vector/matrix types
- [ ] Use hisab transforms for camera/projection
- [ ] hisab geometry for frustum culling, AABB, ray-plane intersection

### ranga (image processing)
- [ ] Texture loading via ranga pixel buffers
- [ ] GPU texture upload from ranga color spaces
- [ ] Mipmap generation via ranga filters

### impetus (physics) — via kiran bridge
- [ ] Debug wireframe rendering of impetus collider shapes
- [ ] Particle rendering from impetus particle system

### Future science crate integration
- [ ] Optics (ray tracing, refraction) → realistic lighting, caustics
- [ ] Fluid dynamics (SPH) → water/smoke/fire particle rendering
- [ ] Electromagnetism → field visualization, force lines

## V1.0 — Production

- [ ] PBR materials (metallic-roughness workflow)
- [ ] Shadow mapping (directional, point, spot)
- [ ] Post-processing pipeline (bloom, tone mapping, SSAO)
- [ ] Skeletal animation (glTF skinned meshes)
- [ ] Terrain rendering (heightmap or procedural)
- [ ] UI rendering (in-game HUD, menus)
- [ ] Publish to crates.io

## Dependency Map

```
soorat (rendering engine)
  ├── wgpu         — GPU abstraction (Vulkan/Metal/DX12/WebGPU)
  ├── winit        — window management
  ├── hisab        — math (vectors, matrices, transforms)  [planned]
  ├── ranga        — image processing (textures, filters)  [planned]
  ├── bytemuck     — vertex type casting
  └── image        — texture loading (png, jpeg)
```
