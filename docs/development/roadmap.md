# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

## V0.1 — Scaffold (done, 2026-03-23)

- Color types (RGBA, hex, lerp, wgpu conversion, prakash optics bridge)
- Vertex types (2D + 3D with bytemuck Pod/Zeroable and wgpu buffer layouts)
- Sprite instances with builder + SpriteBatch with z-sort
- GpuContext (wgpu adapter/device/queue)
- WindowConfig (winit integration, vsync, present mode)
- RenderError with `#[non_exhaustive]`
- prakash optics integration (color temperature, wavelength → Color)
- 42 tests, 7 benchmarks, clippy/fmt clean

## P0 — Rendering Pipeline (blocks kiran visual output)

These items must be done before kiran can render anything to screen. They are the critical path.

### Window + Surface (do first)

- [ ] `Window` struct wrapping winit `Window` + wgpu `Surface`
- [ ] `Window::new(config: &WindowConfig) -> Result<Window>` — creates OS window + wgpu surface
- [ ] Surface configuration (format, present mode, alpha mode)
- [ ] Resize handling — reconfigure surface on window resize event
- [ ] `Window::request_redraw()` / `Window::present()` flow
- [ ] Basic event loop: `run(callback)` that drives winit `EventLoop` and calls user code per frame

### Sprite Render Pipeline (do second)

- [ ] WGSL sprite shader (`sprite.wgsl`) — vertex + fragment, takes position/uv/color
- [ ] `SpritePipeline` struct — holds wgpu `RenderPipeline`, bind group layouts
- [ ] `SpritePipeline::new(device, surface_format) -> Result<SpritePipeline>`
- [ ] Projection uniform buffer (orthographic 2D camera)
- [ ] Vertex buffer upload from `SpriteBatch` → GPU `wgpu::Buffer`
- [ ] Index buffer (quad indices: 0,1,2, 2,3,0 pattern)
- [ ] `SpritePipeline::draw(encoder, view, batch)` — encode a render pass with all sprites
- [ ] Clear color as first render pass operation

### Texture Loading (do third)

- [ ] `Texture` struct wrapping `wgpu::Texture` + `TextureView` + `Sampler`
- [ ] `Texture::from_bytes(device, queue, bytes, label) -> Result<Texture>` — load PNG/JPEG via `image` crate
- [ ] `Texture::from_color(device, queue, color) -> Texture` — 1x1 solid color texture (default/white pixel)
- [ ] Bind group for texture sampling in sprite shader
- [ ] `TextureCache` — HashMap<u64, Texture> keyed by texture_id from Sprite
- [ ] `TextureCache::get_or_load()` for lazy loading

### Integration Test

- [ ] End-to-end test: create window → load texture → draw 100 sprites → present frame → close
- [ ] Headless test: create GpuContext without surface → render to texture → read back pixels → verify

## V0.2 — Complete 2D Rendering

- [ ] Sprite rotation (transform matrix in vertex shader)
- [ ] Sprite atlas / spritesheet (UV region per sprite)
- [ ] Batch rendering — single draw call for sprites sharing a texture
- [ ] Alpha blending (transparent sprites)
- [ ] Render-to-texture (offscreen framebuffer)
- [ ] Frame statistics (draw calls, triangles, GPU time)

## V0.3 — 3D Mesh Rendering

- [ ] `MeshPipeline` struct (vertex + index buffers, 3D vertex shader)
- [ ] Camera uniform buffer (view + projection matrices from kiran Camera)
- [ ] Depth buffer (z-testing)
- [ ] Basic lighting (ambient + single directional light)
- [ ] glTF model loading (positions, normals, UVs, indices)
- [ ] Material binding (base color texture + color factor)

## V0.4 — Debug Rendering

- [ ] `LinePipeline` — draw colored line segments (wireframe)
- [ ] Debug shape rendering from kiran `DebugShape` (circle → line segments, box → 12 lines, capsule → lines + arcs)
- [ ] Grid overlay (configurable spacing, fade at distance)
- [ ] Text rendering (basic glyph atlas, monospace font)

## V0.5 — AGNOS Shared Crate Integration

### hisab (math)
- [ ] Replace glam with hisab vector/matrix types throughout
- [ ] Use hisab transforms for camera/projection
- [ ] hisab geometry for frustum culling, AABB, ray-plane intersection

### ranga (image processing)
- [ ] Texture loading via ranga pixel buffers (replace `image` crate)
- [ ] GPU texture upload from ranga color spaces
- [ ] Mipmap generation via ranga filters

### impetus (physics) — via kiran bridge
- [ ] Debug wireframe rendering of impetus collider shapes
- [ ] Particle rendering from impetus particle system

### Future science crate integration
- [ ] Optics (prakash ray tracing) → realistic lighting, caustics
- [ ] Fluid dynamics (SPH) → water/smoke/fire particle rendering
- [ ] Electromagnetism → field visualization, force lines

## V1.0 — Production

- [ ] PBR materials (metallic-roughness workflow, Cook-Torrance from prakash)
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
  ├── winit        — window management + event loop
  ├── prakash      — optics (spectral color, PBR math)
  ├── hisab        — math (vectors, matrices, transforms)  [planned]
  ├── ranga        — image processing (textures, filters)   [planned]
  ├── bytemuck     — vertex type zero-copy casting
  └── image        — texture loading (png, jpeg)
```

## Context for Agent

Soorat is consumed by:
- **kiran** (`src/gpu.rs`) — `SooratRenderer` implements kiran's `Renderer` trait, translates `DrawCommand::Sprite/Mesh/Clear/SetCamera` to soorat calls
- **salai** (`src/viewport.rs`) — needs soorat 3D viewport for the editor

The P0 items are ordered: Window+Surface first (need a surface to render to), then sprite pipeline (need shaders to draw), then textures (need images on sprites). Each builds on the previous.

Key types already defined that the pipeline must use:
- `Color` (src/color.rs) — RGBA f32, has `to_wgpu()`
- `Vertex2D` (src/vertex.rs) — position, tex_coords, color with `layout()` for wgpu
- `Sprite` / `SpriteBatch` (src/sprite.rs) — sprite instances with z-sort
- `GpuContext` (src/gpu.rs) — adapter, device, queue
- `WindowConfig` (src/window.rs) — size, vsync, present mode
- `RenderError` (src/error.rs) — error variants for all failure modes

Tests: 42 existing. Benchmarks: 7 existing. All passing.
