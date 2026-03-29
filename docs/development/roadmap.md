# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

All planned features have been implemented. Version is 0.23.3.

## Goonj Integration (acoustic visualization)

- [ ] **Ray path rendering**: Consume `goonj::integration::soorat::RayVisualization` to draw acoustic ray paths as line segments in 3D scenes
- [ ] **Pressure map heatmap**: Render `goonj::integration::soorat::PressureMap` as volumetric or slice-based heatmaps showing sound pressure distribution
- [ ] **Room mode patterns**: Visualize `goonj::integration::soorat::ModeVisualization` standing wave patterns as color-mapped surfaces
- [ ] **Portal visualization**: Render `goonj::portal::Portal` openings with energy flow arrows between rooms
- [ ] **Directivity balloons**: Render `goonj::directivity::DirectivityBalloon` as 3D polar patterns
- [ ] **Coupled room decay**: Visualize `goonj::coupled::CoupledDecay` double-slope curves

## Future Considerations

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
- **salai** (`src/viewport.rs`) — uses `draw_into_pass()` APIs + egui_bridge for 3D viewport

Current: 278 tests, 29 benchmarks, 40 modules, 9 WGSL shaders.
