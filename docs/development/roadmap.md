# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

## V0.24 — Multi-Light PBR

- [ ] Wire `LightArrayUniforms` into PBR shader (loop over up to 8 lights)
- [ ] Point light attenuation (range-based falloff)
- [ ] Spot light cone (inner/outer angle falloff)
- [ ] Inverse-transpose model matrix for non-uniform scale normals

## V0.25 — Vertex Skinning + Normal Mapping

- [ ] Skinned vertex format (joint indices + weights in Vertex3D)
- [ ] Joint matrix palette in vertex shader (sample JointUniforms)
- [ ] Normal mapping (tangent-space normal maps)
- [ ] Tangent vector generation in glTF loader + Vertex3D

## V0.26 — Post-Processing Suite

- [ ] Bloom pass (brightness threshold → Gaussian blur → composite)
- [ ] SSAO pass (screen-space ambient occlusion)
- [ ] Separate HDR render target for scene → post-process chain

## V0.27 — Advanced Shadows

- [ ] Cascaded shadow maps (2-4 cascades for large scenes)
- [ ] Shadow map atlas (multiple lights in one texture)
- [ ] Point light shadow maps (cube map or dual-paraboloid)

## V0.28 — Fluid Rendering (pravash integration)

- [ ] SPH particle rendering (point sprites or screen-space fluid)
- [ ] Shallow water surface mesh from pravash height field
- [ ] Particle color from velocity/density (pravash FluidParticle fields)
- [ ] Feature flag: `fluids` (dep: pravash)

## V0.29 — GPU Profiling + WebGPU

- [ ] GPU timestamp queries for per-pass timing
- [ ] FrameStats extended with gpu_time_ms per pass
- [ ] WebGPU browser target validation
- [ ] wgpu feature/limit capability reporting

## V1.0 — Release

- [ ] API freeze — no breaking changes after this
- [ ] Full documentation pass (all pub items documented)
- [ ] Migration guide from 0.23.3 → 1.0
- [ ] Performance regression test suite

## Future (post-V1.0)

- [ ] IBL ambient lighting (environment maps + BRDF LUT sampling)
- [ ] Electromagnetism field visualization (science crate TBD)
- [ ] Thermodynamics heat mapping (science crate TBD)
- [ ] LOD system (level of detail for terrain + meshes)
- [ ] Instanced rendering (draw thousands of same mesh)
- [ ] Compute shader pipeline (general-purpose GPU compute)

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
  ├── impetus      — physics (collider debug wireframes)        [optional]
  └── pravash      — fluid dynamics (particle rendering)        [optional, planned]
```

## Context for Agent

Soorat is consumed by:
- **kiran** (`src/gpu.rs`) — `SooratRenderer` implements kiran's `Renderer` trait
- **salai** (`src/viewport.rs`) — needs soorat 3D viewport for the editor

Current: 188 tests, 21 benchmarks, 25 modules.
