# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

All planned features have been implemented. Version is 0.23.3.

## Salai Editor Integration (priority)

> Required by salai for 3D viewport rendering inside egui panels.

- [ ] **Render-into-pass API** — allow `MeshPipeline::draw()` to render into an existing `wgpu::RenderPass` rather than creating its own. Needed for egui-wgpu `CallbackTrait::paint()` integration.
- [ ] **Offscreen-to-egui texture bridge** — helper to register a `RenderTarget` texture view with egui's wgpu renderer for display via `ui.image()`.
- [ ] **Debug shape rendering into pass** — allow `DebugDrawPipeline::draw()` to render lines/wireframes into an existing pass (for entity bounding boxes, gizmo overlays).
- [ ] **Simple mesh primitives** — built-in cube, sphere, plane meshes for editor entity visualization without requiring glTF model loading.

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
- **salai** (`src/viewport.rs`) — needs soorat 3D viewport for the editor

Current: 253 tests, 29 benchmarks, 35 modules, 8 WGSL shaders.
