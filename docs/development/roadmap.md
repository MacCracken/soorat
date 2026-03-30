# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

Version 1.0.0 — all planned features implemented and stable.

## Future Features (demand-gated)

> These features will be added when downstream consumers (kiran, salai, joshua) need them.

- [ ] **Multi-pass render graph scheduling** — automatic dependency resolution and pass reordering
- [ ] **GPU-driven culling** — compute-shader frustum/occlusion culling
- [ ] **Deferred shading** — G-buffer pass for high light counts
- [ ] **Order-independent transparency** — weighted blended OIT or per-pixel linked lists
- [ ] **Global illumination** — screen-space GI or light probes
- [ ] **Volumetric rendering** — fog, clouds, participating media
- [ ] **Particle LOD** — distance-based particle budget and quality scaling

## v1.0 Criteria — MET

- [x] Core 2D/3D rendering stable
- [x] PBR pipeline with shadows, HDR, bloom, SSAO
- [x] Skinned meshes, animation, glTF loading
- [x] All science integrations complete (goonj, bijli, ushma, pravash, prakash)
- [x] Physics debug rendering (impetus)
- [x] Screenshot capture (selah)
- [x] GPU foundation delegated to mabda
- [x] No `unwrap()`/`panic!()` in library code
- [x] 374 tests, 29 benchmarks, clippy/fmt/doc clean

## Dependency Map

```
soorat (rendering engine)
  ├── mabda        — GPU foundation (device, buffers, pipelines)
  ├── wgpu         — GPU abstraction (Vulkan/Metal/DX12/WebGPU)
  ├── winit        — window management + event loop
  ├── hisab        — math (vectors, matrices, transforms)
  ├── bytemuck     — vertex type zero-copy casting
  ├── image        — texture loading (png, jpeg)
  ├── gltf         — 3D model loading
  ├── prakash      — optics, PBR math, BRDF LUT              [optional: optics]
  ├── ranga        — image processing (pixel buffers)          [optional: ranga]
  ├── impetus      — physics (collider debug wireframes)       [optional: physics-debug]
  ├── pravash      — fluid dynamics (particle/surface render)  [optional: fluids]
  ├── goonj        — acoustics (ray paths, pressure, modes)    [optional: acoustics]
  ├── bijli        — electromagnetism (fields, FDTD, charges)  [optional: em]
  └── ushma        — thermodynamics (thermal grids, cycles)    [optional: thermo]
```

## Context for Agent

Soorat is consumed by:
- **kiran** (`src/gpu.rs`) — `SooratRenderer` implements kiran's `Renderer` trait
- **salai** (`src/viewport.rs`) — uses `draw_into_pass()` APIs + egui_bridge for 3D viewport

Current: 374 tests, 29 benchmarks, 42 modules, 9 WGSL shaders.
