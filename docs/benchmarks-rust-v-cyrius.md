# Benchmarks: Rust 1.2.0 vs Cyrius

> **Generated** by `python3 -I -B scripts/bench_record.py .`; `--check` gates it (port plan §4 step 7, §7).
> Speed is measured, not gated (port plan §0.1, §11): a slow row is a finding to file with its numbers.

**Rust column:** `rust-old/bench-history.csv`, run `2026-11-15T21:19:13Z` at `7f3213a`: the 1.2.0 release run (plan §4 step 8), criterion medians.

**Cyrius column:** `bench-history.csv` at the repository root (`timestamp,commit,branch,benchmark,estimate_ns`, as the Rust history), its latest run: no run recorded yet, so every Cyrius cell is —. `scripts/bench_record.py . --append <cyrius bench log>` adds a run.

**35 Rust benches:** 0 twinned, 0 not applicable, 35 pending, 0 missing. Milestones done (plan §14): M0.

| Rust bench | Milestone | Rust 1.2.0 | Cyrius twin | Cyrius | Cyrius / Rust |
|---|---|---:|---|---:|---:|
| `color/from_hex` | M1 | 1.36 ns | `color/from_hex` — pending | — | — |
| `color/from_rgba8` | M1 | 3.30 ns | `color/from_rgba8` — pending | — | — |
| `color/lerp` | M1 | 1.05 ns | `color/lerp` — pending | — | — |
| `color/luminance` | M1 | 0.76 ns | `color/luminance` — pending | — | — |
| `color/to_array` | M1 | 0.52 ns | `color/to_array` — pending | — | — |
| `sprite/create` | M2 | 2.39 ns | `sprite/create` — pending | — | — |
| `sprite/create_full_builder` | M2 | 3.54 ns | `sprite/create_full_builder` — pending | — | — |
| `sprite/center_bounds` | M2 | 0.25 ns | `sprite/center_bounds` — pending | — | — |
| `sprite/batch_push_100` | M2 | 836.95 ns | `sprite/batch_push_100` — pending | — | — |
| `sprite/batch_push_100_prealloc` | M2 | 841.34 ns | `sprite/batch_push_100_prealloc` — pending | — | — |
| `sprite/batch_sort_100` | M2 | 73.57 ns | `sprite/batch_sort_100` — pending | — | — |
| `sprite/batch_push_1000` | M2 | 8.31 µs | `sprite/batch_push_1000` — pending | — | — |
| `sprite/batch_sort_1000` | M2 | 565.87 ns | `sprite/batch_sort_1000` — pending | — | — |
| `vertex/bytemuck_cast_100` | M1 | 0.77 ns | `vertex/byte_view_100` — pending | — | — |
| `vertex/quad_generation_100` | M2 | 633.72 ns | `vertex/quad_generation_100` — pending | — | — |
| `pipeline/batch_to_vertices_100` | M2 | 1.19 µs | `pipeline/batch_to_vertices_100` — pending | — | — |
| `pipeline/batch_to_vertices_1000` | M2 | 11.90 µs | `pipeline/batch_to_vertices_1000` — pending | — | — |
| `pipeline/batch_to_vertices_rotated_100` | M2 | 1.75 µs | `pipeline/batch_to_vertices_rotated_100` — pending | — | — |
| `pipeline/batch_to_vertices_into_1000` | M2 | 11.97 µs | `pipeline/batch_to_vertices_into_1000` — pending | — | — |
| `debug_draw/wire_box_100` | M2 | 3.01 µs | `debug_draw/wire_box_100` — pending | — | — |
| `debug_draw/wire_sphere_100` | M2 | 24.16 µs | `debug_draw/wire_sphere_100` — pending | — | — |
| `debug_draw/grid_10x10` | M2 | 83.16 ns | `debug_draw/grid_10x10` — pending | — | — |
| `terrain/generate_32x32` | M3 | 12.51 µs | `terrain/generate_32x32` — pending | — | — |
| `terrain/generate_64x64` | M3 | 48.08 µs | `terrain/generate_64x64` — pending | — | — |
| `terrain/generate_256x256` | M3 | 733.85 µs | `terrain/generate_256x256` — pending | — | — |
| `animation/compute_joints_16` | M3 | 282.74 ns | `animation/compute_joints_16` — pending | — | — |
| `animation/compute_joints_64` | M3 | 1.15 µs | `animation/compute_joints_64` — pending | — | — |
| `animation/compute_joints_64_unsorted` | M3 | 1.55 µs | `animation/compute_joints_64_unsorted` — pending | — | — |
| `animation/sample_rotation_64x32` | M3 | 3.42 µs | `animation/sample_rotation_64x32` — pending | — | — |
| `primitives/sphere_32x16` | M3 | 5.84 µs | `primitives/sphere_32x16` — pending | — | — |
| `screenshot/encode_png_256` | M8 | 461.65 µs | `screenshot/encode_png_256` — pending | — | — |
| `shadow/directional_matrix` | M4 | 20.86 ns | `shadow/directional_matrix` — pending | — | — |
| `shadow/directional_matrix_centered` | M4 | 21.53 ns | `shadow/directional_matrix_centered` — pending | — | — |
| `shadow/cascade_splits_4` | M4 | 40.54 ns | `shadow/cascade_splits_4` — pending | — | — |
| `shadow/point_shadow_6_faces` | M4 | 213.44 ns | `shadow/point_shadow_6_faces` — pending | — | — |

## Cyrius-only benches

| Cyrius bench | What it measures | Cyrius |
|---|---|---:|
| `noop` | the scaffold's harness check: an empty call, i.e. loop and call overhead (tests/soorat.bcyr) | — |
