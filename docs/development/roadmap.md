# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

All planned features through V1.0 have been implemented. This document now serves as a reference for what was built and where future work may go.

## Future Considerations

- [ ] Wire `LightArrayUniforms` into PBR shader (multi-light loop)
- [ ] IBL ambient lighting (environment maps + BRDF LUT sampling)
- [ ] Normal mapping (tangent-space normal maps in PBR shader)
- [ ] Inverse-transpose model matrix for non-uniform scale normals
- [ ] Cascaded shadow maps for large scenes
- [ ] GPU timestamp queries for per-pass profiling
- [ ] Bloom post-processing pass (threshold + Gaussian blur + composite)
- [ ] SSAO post-processing pass
- [ ] Skeletal animation vertex skinning in shader (joint weights in Vertex3D)
- [ ] WebGPU browser target validation

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

## Context for Agent

Soorat is consumed by:
- **kiran** (`src/gpu.rs`) — `SooratRenderer` implements kiran's `Renderer` trait
- **salai** (`src/viewport.rs`) — needs soorat 3D viewport for the editor

Current: 188 tests, 21 benchmarks, 25 modules.
