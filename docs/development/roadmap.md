# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

All planned pre-1.0 features have been implemented. Version is 0.29.3.

## Goonj Integration (acoustic visualization) — DONE

- [x] **Ray path rendering**: `ray_paths_to_lines()` — acoustic ray paths as colored line segments with energy-based color fading
- [x] **Pressure map heatmap**: `pressure_map_slice()` — XZ-plane slices of 3D pressure grids with heat-map or signed-pressure coloring
- [x] **Room mode patterns**: `mode_pattern_to_mesh()` — height-field mesh from standing wave patterns with computed normals
- [x] **Portal visualization**: `portal_to_lines()` — wireframe rectangle with normal arrow for portal openings
- [x] **Directivity balloons**: `directivity_balloon_to_mesh()` — deformed sphere mesh with gain-based radius and heat-map coloring
- [x] **Coupled room decay**: `coupled_decay_to_lines()` — double-slope energy decay curves with early/late color blending

## Bijli Integration (electromagnetism visualization)

> Blocked on: bijli `integration/soorat.rs` module (see bijli roadmap)

- [ ] **FDTD field heatmap**: Render `Fdtd2d`/`Fdtd3d` field slices as 2D/3D heatmaps
- [ ] **Field line rendering**: Draw electric/magnetic field line traces as colored polylines
- [ ] **Point charge visualization**: Render charge positions with field halos (size/color by magnitude)
- [ ] **Radiation pattern**: Render far-field patterns as 3D polar balloon or 2D polar plot
- [ ] **Vector field arrows**: Render sampled vector fields as arrow glyphs or streamlines

## Ushma Integration (thermodynamics visualization)

> Blocked on: ushma `integration/soorat.rs` module (see ushma roadmap)

- [ ] **Thermal grid heatmap**: Render `ThermalGrid2D` temperature distributions as colored heatmaps
- [ ] **Temperature profile**: Render `ThermalGrid1D` as a line or ribbon in 3D space
- [ ] **Cycle diagrams**: Render T-s and P-v cycle diagrams as colored line plots
- [ ] **Thermal network graph**: Render node-link diagrams with temperature-colored nodes
- [ ] **Heat flux arrows**: Render thermal gradient vector fields as arrow glyphs

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
  ├── pravash      — fluid dynamics (particle/surface render)  [optional: fluids]
  ├── goonj        — acoustics (ray paths, pressure, modes)    [optional: acoustics]
  ├── bijli        — electromagnetism (fields, FDTD, charges)  [optional: em] (planned)
  └── ushma        — thermodynamics (thermal grids, cycles)    [optional: thermo] (planned)
```

## Context for Agent

Soorat is consumed by:
- **kiran** (`src/gpu.rs`) — `SooratRenderer` implements kiran's `Renderer` trait
- **salai** (`src/viewport.rs`) — uses `draw_into_pass()` APIs + egui_bridge for 3D viewport

Current: 278 tests, 29 benchmarks, 40 modules, 9 WGSL shaders.
