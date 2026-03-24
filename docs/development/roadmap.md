# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

## Completed

### V0.1 — Scaffold (2026-03-23)

- Color types (RGBA, hex, lerp, wgpu conversion, prakash optics bridge)
- Vertex types (2D + 3D with bytemuck Pod/Zeroable and wgpu buffer layouts)
- Sprite instances with builder + SpriteBatch with z-sort + UvRect
- GpuContext (wgpu adapter/device/queue, surface-compatible variant)
- WindowConfig (winit integration, vsync, present mode)
- RenderError with `#[non_exhaustive]`
- prakash optics integration (color temperature, wavelength → Color)

### P0 — Rendering Pipeline (2026-03-23)

- `Window` struct — winit window + wgpu surface, resize handling, event loop via `run()`
- `SpritePipeline` — WGSL shader, orthographic projection, alpha blending
- `SpritePipeline::draw()` — single-texture draw call
- `SpritePipeline::draw_batched()` — multi-texture draw with consecutive grouping + `FrameStats`
- `batch_to_vertices()` / `batch_to_vertices_into()` — sprite → vertex/index generation with rotation
- `Texture` — from_bytes (PNG/JPEG), from_color (1x1 solid), from_rgba, white_pixel
- `TextureCache` — HashMap<u64, Texture+BindGroup> keyed by texture_id
- `UvRect` — sprite atlas UV regions with `from_pixel_rect()`
- 16K sprite limit with overflow protection (u16 indices, `MAX_SPRITES_PER_BATCH`)
- 74+ tests, 17 benchmarks

## Remaining

### V0.2 — Complete 2D Rendering

- [ ] Render-to-texture (offscreen framebuffer)
- [ ] Frame statistics integration (expose FrameStats from draw calls)

### V0.3 — 3D Mesh Rendering

- [ ] `MeshPipeline` struct (vertex + index buffers, 3D vertex shader)
- [ ] Camera uniform buffer (view + projection matrices from kiran Camera)
- [ ] Depth buffer (z-testing)
- [ ] Basic lighting (ambient + single directional light)
- [ ] glTF model loading (positions, normals, UVs, indices)
- [ ] Material binding (base color texture + color factor)

### V0.4 — Debug Rendering

- [ ] `LinePipeline` — draw colored line segments (wireframe)
- [ ] Debug shape rendering from kiran `DebugShape` (circle → line segments, box → 12 lines, capsule → lines + arcs)
- [ ] Grid overlay (configurable spacing, fade at distance)
- [ ] Text rendering (basic glyph atlas, monospace font)

### V0.5 — AGNOS Shared Crate Integration

- [ ] Replace glam with hisab vector/matrix types throughout
- [ ] Texture loading via ranga pixel buffers (replace `image` crate)
- [ ] GPU texture upload from ranga color spaces
- [ ] Debug wireframe rendering of impetus collider shapes
- [ ] Optics (prakash ray tracing) → realistic lighting, caustics

### V1.0 — Production

- [ ] PBR materials (metallic-roughness workflow, Cook-Torrance from prakash)
- [ ] Shadow mapping (directional, point, spot)
- [ ] Post-processing pipeline (bloom, tone mapping, SSAO)
- [ ] Skeletal animation (glTF skinned meshes)
- [ ] Terrain rendering (heightmap or procedural)
- [ ] UI rendering (in-game HUD, menus)
- [ ] Publish to crates.io

## Engineering Backlog

### Performance

- [ ] **Buffer reuse** — `draw()` and `draw_batched()` allocate new vertex/index GPU buffers every frame via `create_buffer_init()`. Should pre-allocate and use `queue.write_buffer()` on persistent buffers. ~2 GPU allocations per frame at 60fps.
- [ ] **Shared sampler** — every `Texture` creates its own `wgpu::Sampler`. Sprites typically share one nearest-neighbor sampler. Create a global sampler and share it.
- [ ] **u32 index path** — `batch_to_vertices` uses u16 indices (limit: 16383 sprites). Add a u32 index variant for large batches or split into multiple draw calls automatically.

### API

- [ ] **`Texture::from_color` should return Result** — currently uses `.expect()` which panics. Public API should not panic.
- [ ] **`TextureCache::get_or_load()`** — lazy loading path (currently insert-only)

## Dependency Map

```
soorat (rendering engine)
  ├── wgpu         — GPU abstraction (Vulkan/Metal/DX12/WebGPU)
  ├── winit        — window management + event loop
  ├── prakash      — optics (spectral color, PBR math)
  ├── hisab        — math (vectors, matrices, transforms)  [planned]
  ├── ranga        — image processing (textures, filters)   [planned]
  ├── bytemuck     — vertex type zero-copy casting
  └── image        — texture loading (png, jpeg)
```
