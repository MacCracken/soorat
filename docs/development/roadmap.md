# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

All planned pre-1.0 features have been implemented. Version is 0.30.0.

## Goonj Integration (acoustic visualization) — DONE

- [x] **Ray path rendering**: `ray_paths_to_lines()` — acoustic ray paths as colored line segments with energy-based color fading
- [x] **Pressure map heatmap**: `pressure_map_slice()` — XZ-plane slices of 3D pressure grids with heat-map or signed-pressure coloring
- [x] **Room mode patterns**: `mode_pattern_to_mesh()` — height-field mesh from standing wave patterns with computed normals
- [x] **Portal visualization**: `portal_to_lines()` — wireframe rectangle with normal arrow for portal openings
- [x] **Directivity balloons**: `directivity_balloon_to_mesh()` — deformed sphere mesh with gain-based radius and heat-map coloring
- [x] **Coupled room decay**: `coupled_decay_to_lines()` — double-slope energy decay curves with early/late color blending

## Bijli Integration (electromagnetism visualization) — DONE

- [x] **FDTD field heatmap**: `field_slice_2d_to_mesh()` / `field_slice_3d_to_mesh()` — 2D/3D FDTD field slices as colored quad heatmaps with magnitude or signed coloring
- [x] **Field line rendering**: `field_lines_to_lines()` — electric/magnetic field line traces as magnitude-colored polylines
- [x] **Point charge visualization**: `charges_to_lines()` — wireframe spheres sized by charge magnitude (red=positive, blue=negative)
- [x] **Radiation pattern**: `radiation_pattern_to_mesh()` — far-field patterns as 3D polar balloon with gain-based radius and heat-map coloring
- [x] **Vector field arrows**: `vector_field_to_arrows()` — sampled vector fields as arrow glyphs with 4-line arrowheads

## Ushma Integration (thermodynamics visualization) — DONE

- [x] **Thermal grid heatmap**: `thermal_grid_to_mesh()` — `ThermalGridVisualization` as colored quad heatmap with temperature or alpha-blend coloring
- [x] **Temperature profile**: `temperature_profile_to_lines()` — 1D temperature profile as heat-mapped line strip along a 3D direction
- [x] **Cycle diagrams**: `cycle_diagram_to_lines()` — T-s and P-v cycle diagrams as normalized colored line plots
- [x] **Thermal network graph**: `thermal_network_to_lines()` — circular-layout node-link diagram with temperature-colored nodes and conductance-weighted edges
- [x] **Heat flux arrows**: `heat_flux_to_arrows()` — 2D heat flux vectors as arrow glyphs on the XZ plane

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
  ├── bijli        — electromagnetism (fields, FDTD, charges)  [optional: em]
  └── ushma        — thermodynamics (thermal grids, cycles)    [optional: thermo]
```

## Context for Agent

Soorat is consumed by:
- **kiran** (`src/gpu.rs`) — `SooratRenderer` implements kiran's `Renderer` trait
- **salai** (`src/viewport.rs`) — uses `draw_into_pass()` APIs + egui_bridge for 3D viewport

Current: 374 tests, 29 benchmarks, 42 modules, 9 WGSL shaders.
