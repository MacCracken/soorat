# Soorat Roadmap

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for the Kiran game engine and AGNOS ecosystem.

Version 1.1.0 — all planned features implemented. Patch sprints 1-3 complete.
1.1.0 (2026-11-15): language/dependency update (Rust 1.100, MSRV 1.100) + P(-1) audit and hardening.

**Cyrius port — proposed (2026-11-15):** see [soorat-port-plan.md](soorat-port-plan.md). 1.1.0 would be the final Rust release and the oracle, and the Cyrius port would ship as 2.0.0. Sprints 4–8 below were planned for Rust; under the port plan they move after the Cyrius 2.0.0 (the version numbers in their headings would change).

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

## Audit & Hardening — 2026-11 (P(-1)) — DONE

- [x] Rust 1.100 / MSRV 1.100 (edition 2024 is still the latest); all dependencies to latest compatible (wgpu stays 29.x — pinned by mabda 1.0, the final Rust mabda)
- [x] `hisab` made optional (only `acoustics` needs it)
- [x] Normal matrix uploaded transposed — rotated meshes were lit as if rotated the opposite way
- [x] Environment-map mip chains were allocated but never written (black specular IBL on rough surfaces) — CPU box-filter mip generation
- [x] BRDF LUT uploaded as sRGB — now `Rgba8Unorm` + bilinear sampler (`Texture::from_rgba_linear`)
- [x] Back-facing geometry: `primitives::sphere`, cylinder caps, all viz heatmaps, particle quads, directivity/radiation balloons
- [x] Animation: quaternion slerp, STEP/CUBICSPLINE samplers, unsorted/cyclic joint hierarchies, non-joint channels no longer hijack joint 0, O(log n) keyframe lookup
- [x] Shadows: non-mirrored light basis, centred frustum (`directional_light_matrix_at`), perspective-correct lookups, `textureSampleCompareLevel` (WebGPU uniformity), atlas division by zero, NaN-safe splits
- [x] SSAO view-position Y flip; skinned shader weight renormalization + tangent transform
- [x] `draw_batched` index-buffer overrun above 16,383 sprites; `WindowConfig::fullscreen` ignored
- [x] Untrusted-input hardening: glTF (index ranges, buffer-view ranges, non-triangle primitives), image size vs device limits, viz grids backed by data, NaN-safe color maps, segment caps
- [x] Zero-size GPU resource guards (depth/HDR/shadow/atlas/particles), particle dispatch limit
- [x] Shared viz helpers (`viz.rs`): one heatmap + one height-field implementation (was 4 + 3 copies)
- [x] naga validation of all WGSL in CI (`tests/shaders.rs`) + GPU smoke tests (`tests/gpu_smoke.rs`)
- [x] `deny.toml` checks all features; CI runs all features + MSRV job

## Sprint 4 — Performance Foundations (v1.1.0)

> Priority: low-effort, high-impact performance wins. Unlocks scene scaling for kiran.

- [ ] **Depth pre-pass** — reuse shadow depth shader with main camera; `DepthPrePass` variant in `PassType` (effort: S, impact: halves overdraw)
- [ ] **Draw call sorting** — front-to-back opaque, back-to-front transparent, by material within groups (effort: S)
- [ ] **Bind group caching** — `BindGroupCache` in mabda keyed by layout+content hash (effort: S)
- [ ] **Ring buffers for uniforms** — double/triple-buffered uniform writes via mabda `FrameRingBuffer` (effort: M)
- [ ] **Debug draw buffer reuse** — `LinePipeline` creates GPU buffers per-frame; reuse with grow-on-demand (effort: S)
- [ ] **Pipeline warm-up API** — `PipelineCache::warm_up()` pre-populates to avoid first-frame hitches (effort: S)
- [ ] **Render graph auto-profiling** — wire `RenderGraph` execution to insert GPU timestamp queries per pass (effort: S)
- [ ] **Per-draw model uniforms** — `MeshPipeline`/`ShadowPipeline` hold one model matrix, so every mesh in a pass shares the last-written transform; use dynamic offsets or a per-draw buffer (effort: M, found in 2026-11 audit)
- [ ] **Depth load/clear control** — `MeshPipeline::draw` always clears depth, so successive `draw` calls ignore each other's depth (needs an API addition; `MeshDrawParams` is not `#[non_exhaustive]`) (effort: S)

## Sprint 5 — Visual Quality (v1.2.0)

> Priority: anti-aliasing and lighting quality. PBR looks noisy without AA.

- [ ] **Temporal anti-aliasing (TAA)** — sub-pixel jitter (Halton), motion vectors, history buffer + neighborhood clamp resolve shader (effort: M)
- [ ] **GTAO** — replace current hemisphere-sampling SSAO with ground-truth AO (Jimenez 2016); fewer samples, no blur pass, better quality (effort: M)
- [ ] **Shader permutation management** — consolidate pbr.wgsl + pbr_skinned.wgsl via naga `override` constants or string preprocessing (effort: M)
- [ ] **Skinned mesh IBL** — add IBL bind group to pbr_skinned.wgsl for feature parity with static PBR (effort: S)
- [ ] **SSAO hash quality** — replace sin-based hash with algebraic hash to eliminate banding on mobile GPUs (effort: S)
- [x] **naga shader validation** — every WGSL file is parsed + validated by naga in `tests/shaders.rs` (2026-11)
- [ ] **Bloom composite** — `PostProcessPipeline` only builds `fs_no_bloom`; `fs_main` + bind group 1 are unreachable, so `BloomPipeline` output is never composited (effort: S)
- [ ] **GGX-prefiltered environment mips** — mips are box-filtered now (no longer black); proper split-sum prefiltering would match roughness (effort: M)
- [ ] **SSAO inputs** — no pass writes the view-space normals SSAO expects (effort: M)
- [ ] **Cascade selection in pbr.wgsl** — `CascadeUniforms`, `ShadowAtlas` and `PointShadowMap` have no shader consumer yet (effort: M)

## Sprint 6 — Scientific Visualization (v1.3.0)

> Priority: joshua consumer needs. Shared infrastructure across viz modules.

- [ ] **Axis convention** — EM `vector_field_to_arrows` maps sim y → world Y while field slices map sim y → world Z; pick one convention for 2D data (effort: S)
- [ ] **Heatmap alignment** — samples render as cells starting at the node (half-cell offset vs arrows at nodes); offer node-centred mode (effort: S)
- [ ] **Shallow water bathymetry** — render `ground + height` as the free surface (effort: S)

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
- [ ] **CUBICSPLINE Hermite interpolation** — tangents are currently dropped (values interpolate linearly/slerp) (effort: S)
- [ ] **Multi-skin animation targeting** — channels map onto the first skin only (effort: S)
- [ ] **Frame-rate-independent particle damping** — damping is applied per step, not scaled by `dt` (effort: S)

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

- [x] **mabda v1.0 on crates.io** — soorat depends on `mabda = "1.0"` from crates.io
- [ ] All Sprint 1 fixes and the 2026-11 audit landed and released
- [ ] **wgpu 30** — blocked: mabda 1.0 (final Rust release) requires `wgpu ^29`

## Dependency Map

```
soorat (rendering engine)
  ├── mabda        — GPU foundation (device, buffers, pipelines) — 1.0, final Rust release
  ├── wgpu         — GPU abstraction (Vulkan/Metal/DX12/WebGPU) — 29.x, pinned by mabda
  ├── winit        — window management + event loop
  ├── bytemuck     — vertex type zero-copy casting
  ├── image        — texture loading (png, jpeg)
  ├── gltf         — 3D model loading
  ├── prakash      — optics, PBR math, BRDF LUT              [optional: optics]
  ├── ranga        — image processing (pixel buffers)          [optional: ranga]
  ├── impetus      — physics (collider debug wireframes)       [optional: physics-debug]
  ├── pravash      — fluid dynamics (particle/surface render)  [optional: fluids]
  ├── goonj        — acoustics (ray paths, pressure, modes)    [optional: acoustics]
  ├── hisab        — math (goonj Vec3)                         [optional: acoustics]
  ├── bijli        — electromagnetism (fields, FDTD, charges)  [optional: em]
  ├── ushma        — thermodynamics (thermal grids, cycles)    [optional: thermo]
  ├── selah        — screenshot annotate/redact/clipboard      [optional: screenshot]
  └── agnosys      — DRM device info (git tag, Linux)          [optional: drm]
```

## Context for Agent

Soorat is consumed by:
- **kiran** (`src/gpu.rs`) — `SooratRenderer` implements kiran's `Renderer` trait
- **salai** (`src/viewport.rs`) — uses `draw_into_pass()` APIs + egui_bridge for 3D viewport
- **joshua** — scientific visualization

Current: 479 tests (all features; 391 default), 35 benchmarks, 44 modules, 9 WGSL shaders.
