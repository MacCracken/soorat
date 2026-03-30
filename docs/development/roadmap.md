# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

Version 1.0.0 — all planned features implemented. Patch sprints 1-3 complete.

## Sprint 1 — Correctness & Safety — DONE

- [x] Eliminated all `expect()`/`assert_eq!()` in library code (8 files, 15 sites → Result propagation)
- [x] Guarded division-by-zero: UvRect, BloomUniforms, BitmapFont, cascade splits, terrain
- [x] Checked integer casts: batch.rs u16/u32, mesh_pipeline indices, terrain width+1 saturating_add
- [x] Fixed PBR specular denominator: `max(4.0 * NdotV * NdotL, 0.001)` in both shaders
- [x] Fixed shadow coord z < 0.0 check in both PBR shaders
- [x] Fixed animation parent-joint logic: removed broken `any(|_| false)`, direct parent search
- [x] Fixed `from_ior()`: F0 now stored in `_pad0` for shader access
- [x] Fixed `flatten_mat4` doc: clarified column-major flatten (no transpose needed for glTF)

## Sprint 2 — Code Quality & Observability — DONE

- [x] Extracted shared viz helpers: `visualization_heat_map`, `signed_value_color`, `normal_to_basis`
- [x] `#[must_use]` sweep: ~50 pure functions across 17 files
- [x] `#[inline]` sweep: ~15 hot-path functions
- [x] `#[non_exhaustive]` on `AnimationProperty` enum
- [x] Tracing spans: 22 `tracing::debug!` calls across 12 files
- [x] `gltf_loader.rs` format! → write! for loop label
- [x] `TextureCache` triple lookup → single `entry()` API
- [x] `#[deprecated]` on `Material` re-export
- [x] `error.rs` From impl: `format!("{other:#}")` preserves error chain

## Sprint 3 — Test Hardening — DONE

- [x] Math edge case tests: look_at parallel/zero, flatten_mat4 roundtrip, compose_trs zero scale/rotation, normal_to_basis orthogonality (9 tests)
- [x] Division-by-zero regression tests: UvRect, BitmapFont, BloomUniforms, cascade splits, terrain, TerrainLod (6 tests)
- [x] Error conversion tests: all 5+ `From<mabda::GpuError>` arms (5 tests)
- [x] Render graph cycle test (1 test)
- [x] Animation crash tests: empty keyframes, single joint, property values (3 tests)
- [x] Primitive zero-input tests: sphere(0,0), plane(0), cylinder(0) (3 tests)
- [x] Feature-gated adversarial tests: mismatched dimensions, NaN magnitudes, equal temps, single cells (5 tests)
- [x] NaN guard fix: `vector_field_to_arrows` now skips non-finite magnitudes
- [x] Additional: batch u32 overflow, singular camera matrix (2 tests)

## Sprint 4 — Performance Foundations (v1.1.0)

> Priority: low-effort, high-impact performance wins. Unlocks scene scaling for kiran.

- [ ] **Depth pre-pass** — reuse shadow depth shader with main camera; `DepthPrePass` variant in `PassType` (effort: S, impact: halves overdraw)
- [ ] **Draw call sorting** — front-to-back opaque, back-to-front transparent, by material within groups (effort: S)
- [ ] **Bind group caching** — `BindGroupCache` in mabda keyed by layout+content hash (effort: S)
- [ ] **Ring buffers for uniforms** — double/triple-buffered uniform writes via mabda `FrameRingBuffer` (effort: M)
- [ ] **Debug draw buffer reuse** — `LinePipeline` creates GPU buffers per-frame; reuse with grow-on-demand (effort: S)
- [ ] **Pipeline warm-up API** — `PipelineCache::warm_up()` pre-populates to avoid first-frame hitches (effort: S)
- [ ] **Render graph auto-profiling** — wire `RenderGraph` execution to insert GPU timestamp queries per pass (effort: S)

## Sprint 5 — Visual Quality (v1.2.0)

> Priority: anti-aliasing and lighting quality. PBR looks noisy without AA.

- [ ] **Temporal anti-aliasing (TAA)** — sub-pixel jitter (Halton), motion vectors, history buffer + neighborhood clamp resolve shader (effort: M)
- [ ] **GTAO** — replace current hemisphere-sampling SSAO with ground-truth AO (Jimenez 2016); fewer samples, no blur pass, better quality (effort: M)
- [ ] **Shader permutation management** — consolidate pbr.wgsl + pbr_skinned.wgsl via naga `override` constants or string preprocessing (effort: M)
- [ ] **Skinned mesh IBL** — add IBL bind group to pbr_skinned.wgsl for feature parity with static PBR (effort: S)
- [ ] **SSAO hash quality** — replace sin-based hash with algebraic hash to eliminate banding on mobile GPUs (effort: S)
- [ ] **naga build-time shader validation** — validate all `include_str!()` WGSL in build.rs (effort: S)

## Sprint 6 — Scientific Visualization (v1.3.0)

> Priority: joshua consumer needs. Shared infrastructure across viz modules.

- [ ] **Perceptually uniform color maps** — `ColorMap` struct with viridis/inferno/magma/plasma/coolwarm presets as 1D LUT; wire into all viz modules (effort: S, critical for scientific accuracy)
- [ ] **Volume rendering** — 3D texture upload, ray-march fragment shader, transfer function mapping (effort: L)
- [ ] **3D picking / selection** — ID render pass (object IDs as colors → readback pixel under cursor) for salai editor and joshua (effort: M)
- [ ] **Isosurface extraction** — GPU marching cubes via compute shader + lookup table storage buffer (effort: M)
- [ ] **Streamlines / pathlines** — generic vector field streamline tracer, tube mesh generation (effort: M)
- [ ] **Annotation overlays** — 3D-anchored text labels via world→screen projection + existing `TextBatch` (effort: S)

## Sprint 7 — GPU-Driven Rendering (v1.4.0)

> Priority: scene scaling for kiran. Thousands of meshes without CPU bottleneck.

- [ ] **GPU-driven frustum culling** — compute shader tests bounding volumes, outputs indirect draw buffer (effort: L)
- [ ] **Hi-Z occlusion pyramid** — generate from depth pre-pass, feed into culling shader (effort: M)
- [ ] **Order-independent transparency** — weighted blended OIT (two render targets + composite) (effort: M)
- [ ] **Device-lost error recovery** — device lost callback, resource recreation registry, graceful restart (effort: M)

## Sprint 8 — Advanced Rendering (v2.0.0)

> Priority: demand-gated. Only when consumers need it.

- [ ] **Screen-space reflections (SSR)** — depth buffer ray march + Hi-Z acceleration + IBL fallback
- [ ] **Volumetric fog** — froxel-based scattering (Hillaire 2016), god rays from directional lights
- [ ] **Deferred shading** — G-buffer pass for high light counts + deferred decals
- [ ] **Bindless / indirect rendering** — texture binding arrays + multi_draw_indirect, collapse N materials to 1 draw call
- [ ] **Full render graph** — resource lifetime tracking, transient texture allocation, automatic barriers, single `graph.execute()`
- [ ] **Shader hot-reload** — file watcher + pipeline invalidation, feature-gated behind `dev-tools`
- [ ] **Render tier auto-selection** — `RenderTier` enum auto-detected from `GpuCapabilities`
- [ ] **RenderDoc integration** — programmatic frame capture via `renderdoc-rs`, feature-gated

## Future (post-2.0 / demand-gated)

- [ ] Meshlets / mesh shaders (waiting on wgpu support)
- [ ] WebGPU / wasm32 target
- [ ] Visual regression testing framework
- [ ] Property-based testing (proptest) for math/color modules
- [ ] Texture streaming / virtual textures

## Publish Blockers

- [ ] **mabda v1.0 on crates.io** — soorat currently uses `path = "../mabda"` dep
- [ ] All Sprint 1 fixes landed and released

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
- **joshua** — scientific visualization

Current: 408 tests, 29 benchmarks, 42 modules, 9 WGSL shaders.
