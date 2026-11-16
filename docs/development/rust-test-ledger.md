# Rust-test ledger

> **Generated** by `python3 -I -B scripts/rust_test_ledger.py . --write`; `--check` gates it (port plan §4
> step 7, §7). Every test of the frozen Rust oracle (`rust-old/`, soorat 1.2.0) is ported to a Cyrius
> `test_group` of the same name (tests/README.md), recorded here with its reason, or pending until its
> milestone (plan §3.1; done = ✅ in the plan's §14). The golden corpus checks every ported function on its
> grid besides; this ledger is about the oracle's own assertions.

**502 Rust tests** (460 unit, 12 integration, 9 GPU smoke, 18 oracle-hook, 2 shader-validation, 1 doc): 242 ported, 28 not applicable, 3 deferred, 229 pending, 0 unaccounted. Milestones done: M0, M1, M2, M3, M4.

| Rust file | Kind | Milestone | Cyrius tests | Rust tests | Ported | N/A | Deferred | Pending |
|---|---|---|---|---:|---:|---:|---:|---:|
| `src/error.rs` | unit | M1+M6 | `tests/error.tcyr` | 9 | 1 | 3 | 0 | 5 |
| `src/math_util.rs` | unit | M1 | `tests/math.tcyr` | 13 | 13 | 0 | 0 | 0 |
| `src/color.rs` | unit | M1 | `tests/color.tcyr`<br>`tests/color_optics.tcyr` | 23 | 23 | 0 | 0 | 0 |
| `src/vertex.rs` | unit | M1+M6 | `tests/layout.tcyr` | 8 | 6 | 0 | 0 | 2 |
| `src/instancing.rs` | unit | M1+M6 | `tests/layout.tcyr` | 6 | 2 | 0 | 0 | 4 |
| `src/lights.rs` | unit | M1 | `tests/lights.tcyr` | 8 | 8 | 0 | 0 | 0 |
| `src/lod.rs` | unit | M1 | `tests/lod.tcyr` | 7 | 7 | 0 | 0 | 0 |
| `src/material.rs` | unit | M6 | `tests/material.tcyr` | 1 | 0 | 0 | 0 | 1 |
| `src/sprite.rs` | unit | M2 | `tests/sprite.tcyr` | 15 | 15 | 0 | 0 | 0 |
| `src/text.rs` | unit | M2 | `tests/text.tcyr` | 10 | 10 | 0 | 0 | 0 |
| `src/ui.rs` | unit | M2 | `tests/ui.tcyr` | 6 | 6 | 0 | 0 | 0 |
| `src/debug_draw.rs` | unit | M2+M6 | `tests/debug_geom.tcyr`<br>`tests/line_pipeline.tcyr` | 16 | 15 | 0 | 0 | 1 |
| `src/primitives.rs` | unit | M3 | `tests/primitives.tcyr` | 12 | 12 | 0 | 0 | 0 |
| `src/terrain.rs` | unit | M3 | `tests/terrain.tcyr` | 11 | 11 | 0 | 0 | 0 |
| `src/viz.rs` | unit | M3 | `tests/viz.tcyr` | 14 | 14 | 0 | 0 | 0 |
| `src/animation.rs` | unit | M3 | `tests/animation.tcyr` | 21 | 21 | 0 | 0 | 0 |
| `src/gltf_loader.rs` | unit | M3 | `tests/gltf.tcyr` | 5 | 5 | 0 | 0 | 0 |
| `src/shadow.rs` | unit | M4 | `tests/shadow_math.tcyr`<br>`tests/shadow.tcyr` | 23 | 23 | 0 | 0 | 0 |
| `src/environment.rs` | unit | M1+M4+M7 | `tests/tables.tcyr`<br>`tests/env_cpu.tcyr`<br>`tests/environment.tcyr` | 8 | 6 | 0 | 0 | 2 |
| `src/pbr_material.rs` | unit | M4 | `tests/pbr_material.tcyr` | 8 | 8 | 0 | 0 | 0 |
| `src/render_graph.rs` | unit | M4 | `tests/render_graph.tcyr` | 10 | 10 | 0 | 0 | 0 |
| `src/profiler.rs` | unit | M6 | `tests/profiler.tcyr` | 8 | 0 | 0 | 0 | 8 |
| `src/capabilities.rs` | unit | M6 | `tests/capabilities.tcyr` | 4 | 0 | 0 | 0 | 4 |
| `src/acoustic_render.rs` | unit | M5 | `tests/acoustic_render.tcyr` | 35 | 0 | 0 | 0 | 35 |
| `src/thermo_render.rs` | unit | M5 | `tests/thermo_render.tcyr` | 24 | 0 | 0 | 0 | 24 |
| `src/fluid_render.rs` | unit | M5 | `tests/fluid_render.tcyr` | 15 | 0 | 0 | 0 | 15 |
| `src/em_render.rs` | unit | M5 | `tests/em_render.tcyr` | 37 | 0 | 0 | 0 | 37 |
| `src/texture.rs` | unit | M6 | `tests/texture.tcyr` | 5 | 0 | 0 | 0 | 5 |
| `src/render_target.rs` | unit | M6 | `tests/render_target.tcyr` | 2 | 0 | 0 | 0 | 2 |
| `src/compute.rs` | unit | M6 | `tests/compute.tcyr` | 11 | 0 | 0 | 0 | 11 |
| `src/pipeline.rs` | unit | M2+M6 | `tests/batch.tcyr`<br>`tests/sprite_pipeline.tcyr` | 18 | 16 | 0 | 0 | 2 |
| `src/mesh_pipeline.rs` | unit | M6 | `tests/mesh_pipeline.tcyr` | 15 | 0 | 0 | 0 | 15 |
| `src/gpu.rs` | unit | M6 | — | 1 | 0 | 1 | 0 | 0 |
| `src/hdr.rs` | unit | M8 | `tests/hdr.tcyr` | 6 | 0 | 0 | 0 | 6 |
| `src/postprocess.rs` | unit | M8 | `tests/postprocess.tcyr` | 6 | 0 | 0 | 0 | 6 |
| `src/ssao.rs` | unit | M8 | `tests/ssao.tcyr` | 4 | 0 | 0 | 0 | 4 |
| `src/gpu_particles.rs` | unit | M8 | `tests/gpu_particles.tcyr` | 5 | 0 | 0 | 0 | 5 |
| `src/screenshot.rs` | unit | M8 | `tests/screenshot.tcyr` | 10 | 0 | 0 | 0 | 10 |
| `src/window.rs` | unit | M9 | `tests/surface.tcyr` | 8 | 0 | 0 | 0 | 8 |
| `src/drm.rs` | unit | M9 | `tests/drm.tcyr` | 9 | 0 | 0 | 3 | 6 |
| `src/egui_bridge.rs` | doc, unit | — | — | 4 | 0 | 4 | 0 | 0 |
| `tests/integration.rs` | integration | M1+M2+M6+M9 | `tests/integration.tcyr` | 12 | 10 | 0 | 0 | 2 |
| `tests/gpu_smoke.rs` | gpu smoke | M6+M7+M8 | `tests/gpu_smoke.tcyr` | 9 | 0 | 0 | 0 | 9 |
| `tests/oracle.rs` | oracle hooks | — | — | 18 | 0 | 18 | 0 | 0 |
| `tests/shaders.rs` | shader validation | — | — | 2 | 0 | 2 | 0 | 0 |
| **Total** | | | | **502** | **242** | **28** | **3** | **229** |

## Pending by milestone

M5: 111, M6: 64, M7: 4, M8: 35, M9: 15

## Rust's ignored tests

- `src/egui_bridge.rs` doc test at line 7: not ported.

## Tests without a port

| Rust file | Test | Status |
|---|---|---|
| `src/error.rs` | `error_display` | n/a: thiserror's message text is not carried; errors are SR_ERR_* codes (§3.2, ADR 0002 #1) |
| `src/error.rs` | `error_other_variant` | n/a: `Other` boxes a foreign error and shows its message; a 2.0 error is an SR_ERR_* code with no payload (§3.2, ADR 0002 #1) |
| `src/error.rs` | `error_is_send_sync` | n/a: Rust's `Send + Sync` mechanics have no Cyrius counterpart (§3.2) |
| `src/gpu.rs` | `gpu_context_types` | n/a: checks soorat's re-export of mabda's GpuContext; 2.0 takes the application's mabda context and re-exports nothing (ADR 0002 #2, #3) |
| `src/drm.rs` | `list_render_nodes_returns_vec` | deferred: agnodrm 1.8.1 has no render-node or connector query, so 2.0 carries the reduced drm subset (D8, ADR 0002 #11) until L10 lands |
| `src/drm.rs` | `list_connectors_nonexistent_path` | deferred: agnodrm 1.8.1 has no render-node or connector query, so 2.0 carries the reduced drm subset (D8, ADR 0002 #11) until L10 lands |
| `src/drm.rs` | `connector_summary_debug` | deferred: agnodrm 1.8.1 has no render-node or connector query, so 2.0 carries the reduced drm subset (D8, ADR 0002 #11) until L10 lands |
| `src/egui_bridge.rs` | all 4 | n/a: `egui_bridge` is not carried: Cyrius has no egui (D8, §3.2, ADR 0002 #10) |
| `tests/oracle.rs` | all 18 | n/a: the tests of the `oracle` hooks added to the frozen crate for this port (D1, port plan §4 step 0), not of soorat 1.2.0's API; scripts/golden/ drives the hooks, and the golden corpus checks what they give |
| `tests/shaders.rs` | all 2 | n/a: naga's offline WGSL validation. Cyrius has no offline validator; wgpu-native validates each shader at pipeline creation, and the GPU smoke tests create every pipeline (port plan §1.1, §3.2) |
