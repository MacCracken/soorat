# Changelog

All notable changes to this project will be documented in this file.

## [0.23.3] - 2026-03-23

### Added
- **Window + Surface** — `Window` struct wrapping winit + wgpu surface, `run()` event loop, resize handling.
- **Sprite Pipeline** — `SpritePipeline` with WGSL shader, orthographic projection, alpha blending.
- **Sprite Rotation** — CPU-side quad rotation around center with trig fast path for rotation=0.
- **Sprite Atlas** — `UvRect` type with `from_pixel_rect()` for texture atlas sub-regions.
- **Batched Rendering** — `SpritePipeline::draw_batched()` groups consecutive sprites by texture_id, one draw call per group.
- **Frame Statistics** — `FrameStats` (draw_calls, triangles, sprites) returned from draw methods.
- **Render-to-Texture** — `RenderTarget` offscreen framebuffer with `read_pixels()` GPU readback.
- **Texture Loading** — `Texture::from_bytes()` (PNG/JPEG), `from_color()`, `from_rgba()`, `white_pixel()`.
- **Texture Cache** — `TextureCache` with single HashMap storing texture + bind group.
- **Overflow Protection** — `MAX_SPRITES_PER_BATCH` (16383) clamp for u16 index safety.
- **3D Mesh Pipeline** — `MeshPipeline` with Vertex3D layout, depth buffer, back-face culling.
- **3D Shader** — `mesh.wgsl` with model/view_proj transform, Lambertian diffuse + ambient lighting.
- **Camera + Lighting** — `CameraUniforms` (view_proj + model), `LightUniforms` (ambient + directional).
- **Depth Buffer** — `DepthBuffer` (Depth32Float) with resize support.
- **Mesh Type** — `Mesh` struct with GPU vertex/index buffers (u32 indices).
- **glTF Loading** — `gltf_loader::load_model()` with zero-copy buffer borrowing, embedded texture extraction.
- **Material** — `Material` struct with base_color texture + color factor + bind group.
- **Debug Lines** — `LinePipeline` (LineList topology, depth-tested), `LineBatch` accumulator.
- **Debug Shapes** — `wire_box()`, `wire_circle()`, `wire_sphere()`, `wire_capsule()`, `grid()`.
- **Impetus Integration** — `LineBatch::collider()` for physics ColliderShape wireframes (feature: `physics-debug`).
- **Ranga Integration** — `Texture::from_pixel_buffer()` for ranga PixelBuffer (feature: `ranga`).
- **Hisab Integration** — Replaced glam with hisab for ecosystem-consistent math.
- **Feature Flags** — `optics`, `ranga`, `physics-debug`, `full`.
- `GpuContext::new_for_surface()` for surface-compatible adapter selection.
- `SpriteBatch::with_capacity()` for pre-allocated batches.
- `batch_to_vertices_into()` for zero-allocation vertex generation in game loops.
- `RenderError::Model` variant for glTF errors.
- `SpriteBuffers` — persistent GPU buffer reuse for zero per-frame allocation.
- `batch_to_vertices_u32()` / `batch_to_vertices_u32_into()` — u32 index path, no sprite count limit.
- `create_default_sampler()` — shared sampler factory to avoid per-texture sampler allocation.
- `Texture::from_rgba_with_sampler()` — texture creation with externally-provided sampler.
- `TextureCache::get_or_load()` — lazy texture loading from bytes.
- `SpritePipeline::draw_with_buffers()` — draw using persistent `SpriteBuffers`.

### Changed
- `Texture::from_color()` now returns `Result` instead of panicking.
- `Texture::white_pixel()` now returns `Result`.
- `SpritePipeline::draw()` now returns `FrameStats`.
- Orthographic projection simplified (constants folded).
- `TextureCache` merged from two HashMaps into single HashMap.
- `Window::new()` delegates to `GpuContext::new()` (no duplicate GPU init).
- `LightUniforms` default direction normalized.

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
