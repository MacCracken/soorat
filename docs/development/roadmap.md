# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

## Completed

### V0.23.3 — Core + 2D + 3D + Debug + Textures + Ecosystem

- Color types (RGBA, hex, lerp, wgpu conversion, prakash optics bridge)
- Vertex types (2D + 3D with bytemuck Pod/Zeroable and wgpu buffer layouts)
- Window + Surface — winit window + wgpu surface, resize handling, event loop
- GpuContext with surface-compatible adapter selection
- SpritePipeline — WGSL shader, orthographic projection, alpha blending, rotation, UvRect
- SpriteBuffers — persistent GPU buffer reuse, u16/u32 index paths
- MeshPipeline — Vertex3D, depth buffer, back-face culling, Lambertian lighting
- glTF loading — zero-copy buffer borrowing, embedded texture extraction
- Material — base_color texture + color factor + bind group
- LinePipeline + LineBatch — wire_box, wire_circle, wire_sphere, wire_capsule, grid
- Texture — from_bytes, from_color, from_rgba, white_pixel, shared sampler
- TextureCache with get_or_load(), RenderTarget with read_pixels()
- hisab math, prakash optics, ranga pixel buffers, impetus debug wireframes

### V0.24 — PBR + Shadows

- MaterialUniforms — dielectric/metal/IOR presets, BRDF LUT generation
- ShadowMap + ShadowPipeline — directional light shadow mapping
- ShadowUniforms + directional_light_matrix()
- PBR shader (pbr.wgsl) — Cook-Torrance BRDF
- GpuLight — directional/point/spot with intensity + range/angle
- LightArrayUniforms — multi-light system

### V0.25 — Post-Processing + Animation

- PostProcessPipeline — bloom, tone mapping (Reinhard/ACES/exposure), SSAO
- PostProcessUniforms + ToneMapMode enum
- Skeletal animation — Skeleton, Joint, AnimationClip, AnimationChannel, Keyframe
- JointUniforms — GPU joint matrix upload

## Remaining

### V0.26 — World Rendering

- [ ] Terrain rendering (heightmap or procedural)
- [ ] UI rendering (in-game HUD, menus)
- [ ] Text rendering (glyph atlas, monospace font)

### V1.0 — Production

- [ ] API stabilization + documentation pass
- [ ] Performance profiling + GPU timing queries
- [ ] Multi-window support
- [ ] WebGPU target validation

## Stats

- **Source:** 5,437 lines across 14 modules + 5 WGSL shaders
- **Tests:** 159 (147 unit + 12 integration), 22 benchmarks
- **Features:** `optics` (prakash), `ranga` (pixel buffers), `physics-debug` (impetus wireframes)

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
