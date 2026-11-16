# Benchmarks: Rust 1.2.0 vs Cyrius

> **Generated** by `python3 -I -B scripts/bench_record.py .`; `--check` gates it (port plan §4 step 7, §7).
> Speed is measured, not gated (port plan §0.1, §11): a slow row is a finding to file with its numbers.

**Rust column:** `rust-old/bench-history.csv`, run `2026-11-15T21:19:13Z` at `7f3213a`: the 1.2.0 release run (plan §4 step 8), criterion medians.

**Cyrius column:** `bench-history.csv` at the repository root (`timestamp,commit,branch,benchmark,estimate_ns`, as the Rust history), its latest run: `2026-11-16T01:25:25Z` at `fe59578+wip`, `cyrius bench` means. `scripts/bench_record.py . --append <cyrius bench log>` adds a run.

**35 Rust benches:** 15 twinned, 0 not applicable, 20 pending, 0 missing. Milestones done (plan §14): M0, M1.

| Rust bench | Milestone | Rust 1.2.0 | Cyrius twin | Cyrius | Cyrius / Rust |
|---|---|---:|---|---:|---:|
| `color/from_hex` | M1 | 1.36 ns | `color/from_hex` | 18.92 ns | 13.9× |
| `color/from_rgba8` | M1 | 3.30 ns | `color/from_rgba8` | 14.40 ns | 4.37× |
| `color/lerp` | M1 | 1.05 ns | `color/lerp` | 36.89 ns | 35.0× |
| `color/luminance` | M1 | 0.76 ns | `color/luminance` | 10.41 ns | 13.7× |
| `color/to_array` | M1 | 0.52 ns | `color/to_array` | 4.62 ns | 8.91× |
| `sprite/create` | M2 | 2.39 ns | `sprite/create` | 36.28 ns | 15.2× |
| `sprite/create_full_builder` | M2 | 3.54 ns | `sprite/create_full_builder` | 42.81 ns | 12.1× |
| `sprite/center_bounds` | M2 | 0.25 ns | `sprite/center_bounds` | 29.75 ns | 117× |
| `sprite/batch_push_100` | M2 | 836.95 ns | `sprite/batch_push_100` | 4.74 µs | 5.66× |
| `sprite/batch_push_100_prealloc` | M2 | 841.34 ns | `sprite/batch_push_100_prealloc` | 4.66 µs | 5.54× |
| `sprite/batch_sort_100` | M2 | 73.57 ns | `sprite/batch_sort_100` | 300.00 ns | 4.08× |
| `sprite/batch_push_1000` | M2 | 8.31 µs | `sprite/batch_push_1000` | 47.80 µs | 5.75× |
| `sprite/batch_sort_1000` | M2 | 565.87 ns | `sprite/batch_sort_1000` | 3.28 µs | 5.80× |
| `vertex/bytemuck_cast_100` | M1 | 0.77 ns | `vertex/byte_view_100` | 1.21 ns | 1.57× |
| `vertex/quad_generation_100` | M2 | 633.72 ns | `vertex/quad_generation_100` | 5.55 µs | 8.76× |
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
| `batch/to_vertices_100` | M2: sr_batch_to_vertices_into on 100 unrotated sprites; no Rust bench (cf. vertex/quad_generation_100) | 8.95 µs |
| `batch/to_vertices_100_rotated` | M2: the same with each sprite rotated (sin and cos per sprite); no Rust bench | 15.49 µs |
| `color_optics/from_prakash` | M1 (`optics`): a prakash Rgb clamped and narrowed to a Color; no Rust bench | 20.07 ns |
| `color_optics/from_temperature` | M1 (`optics`): a blackbody colour through prakash's fit; no Rust bench | 434.00 ns |
| `color_optics/from_wavelength` | M1 (`optics`): a spectral colour through prakash; no Rust bench | 57.89 ns |
| `json/color_from_json` | M1: a serde_json Color document, read (bayan's value tree); no Rust bench | 2.12 µs |
| `json/color_to_json` | M1: Color's serde document, written (bayan's round-trip digits); no Rust bench | 3.92 µs |
| `math/compose_trs` | M1: `compose_trs` in f64, rounded once; no Rust bench (animation's joint chains call it, M3) | 47.34 ns |
| `math/look_at` | M1: `look_at` in f64, rounded once; no Rust bench (`shadow/directional_matrix` calls it, M4) | 77.13 ns |
| `math/mul_mat4` | M1: `mul_mat4` in f64, rounded once (D4 rule b); no Rust bench (`shadow/*` call it, M4) | 117.00 ns |
| `math/normal_to_basis` | M1: `normal_to_basis` in f64, rounded once; no Rust bench (acoustic and em call it, M5) | 61.15 ns |
| `math/normalize3` | M1: `normalize3` in f64, rounded once; no Rust bench | 17.91 ns |
| `math/perspective_90` | M1: `perspective_90` in f64, rounded once; no Rust bench (`shadow/point_shadow_6_faces`, M4) | 30.99 ns |
| `noop` | the scaffold's harness check: an empty call, i.e. loop and call overhead (tests/soorat.bcyr) | 2.33 ns |
| `tables/f16_to_f32` | M1: binary16 → binary32 (bit manipulation); no Rust bench (environment's HDR faces, M4) | 6.58 ns |
| `tables/f32_to_f16` | M1: binary32 → binary16, ties to even; no Rust bench (environment's HDR faces, M4) | 12.34 ns |
| `tables/linear_to_srgb` | M1: sRGB encode, f32 with a correctly rounded pow; no Rust bench (environment, M4) | 204.00 ns |
| `tables/srgb8_decode` | M1: a decode-table lookup (the generated table); no Rust bench (environment, M4) | 3.55 ns |
| `tables/srgb_to_linear` | M1: sRGB decode, f32 with a correctly rounded pow; no Rust bench (environment, M4) | 200.00 ns |
