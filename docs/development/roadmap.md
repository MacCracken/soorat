# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

All planned features have been implemented. Version is 0.23.3.

## Future Considerations

- [ ] IBL ambient lighting (environment maps + BRDF LUT sampling in shader)
- [ ] Electromagnetism field visualization (science crate TBD)
- [ ] Thermodynamics heat mapping (science crate TBD)

## Dependency Map

```
soorat (rendering engine)
  ├── wgpu         — GPU abstraction (Vulkan/Metal/DX12/WebGPU)
  ├── winit        — window management + event loop
  ├── hisab        — math (vectors, matrices, transforms)
  ├── bytemuck     — vertex type zero-copy casting
  ├── image        — texture loading (png, jpeg)
  ├── gltf         — 3D model loading
  ├── prakash      — optics, PBR math, BRDF LUT              [optional: optics]
  ├── ranga        — image processing (pixel buffers)          [optional: ranga]
  ├── impetus      — physics (collider debug wireframes)       [optional: physics-debug]
  └── pravash      — fluid dynamics (particle/surface render)  [optional: fluids]
```

## Context for Agent

Soorat is consumed by:
- **kiran** (`src/gpu.rs`) — `SooratRenderer` implements kiran's `Renderer` trait
- **salai** (`src/viewport.rs`) — needs soorat 3D viewport for the editor

Current: 251 tests, 29 benchmarks, 34 modules, 8 WGSL shaders.
