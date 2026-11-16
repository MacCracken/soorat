# Benchmarks: Rust 1.2.0 vs Cyrius

> **Generated** by `python3 -I -B scripts/bench_record.py .`; `--check` gates it (port plan §4 step 7, §7).
> Speed is measured, not gated (port plan §0.1, §11): a slow row is a finding to file with its numbers.

**Rust column:** `rust-old/bench-history.csv`, run `2026-11-15T21:19:13Z` at `7f3213a`: the 1.2.0 release run (plan §4 step 8), criterion medians.

**Cyrius column:** `bench-history.csv` at the repository root (`timestamp,commit,branch,benchmark,estimate_ns`, as the Rust history), its latest run: `2026-11-16T04:46:41Z` at `ca25318+wip`, `cyrius bench` means. `scripts/bench_record.py . --append <cyrius bench log>` adds a run.

**35 Rust benches:** 34 twinned, 0 not applicable, 1 pending, 0 missing. Milestones done (plan §14): M0, M1, M2, M3.

| Rust bench | Milestone | Rust 1.2.0 | Cyrius twin | Cyrius | Cyrius / Rust |
|---|---|---:|---|---:|---:|
| `color/from_hex` | M1 | 1.36 ns | `color/from_hex` | 19.82 ns | 14.6× |
| `color/from_rgba8` | M1 | 3.30 ns | `color/from_rgba8` | 15.45 ns | 4.69× |
| `color/lerp` | M1 | 1.05 ns | `color/lerp` | 38.60 ns | 36.6× |
| `color/luminance` | M1 | 0.76 ns | `color/luminance` | 11.04 ns | 14.5× |
| `color/to_array` | M1 | 0.52 ns | `color/to_array` | 4.88 ns | 9.42× |
| `sprite/create` | M2 | 2.39 ns | `sprite/create` | 37.25 ns | 15.6× |
| `sprite/create_full_builder` | M2 | 3.54 ns | `sprite/create_full_builder` | 44.02 ns | 12.4× |
| `sprite/center_bounds` | M2 | 0.25 ns | `sprite/center_bounds` | 29.96 ns | 118× |
| `sprite/batch_push_100` | M2 | 836.95 ns | `sprite/batch_push_100` | 4.80 µs | 5.73× |
| `sprite/batch_push_100_prealloc` | M2 | 841.34 ns | `sprite/batch_push_100_prealloc` | 4.78 µs | 5.68× |
| `sprite/batch_sort_100` | M2 | 73.57 ns | `sprite/batch_sort_100` | 317.00 ns | 4.31× |
| `sprite/batch_push_1000` | M2 | 8.31 µs | `sprite/batch_push_1000` | 48.14 µs | 5.80× |
| `sprite/batch_sort_1000` | M2 | 565.87 ns | `sprite/batch_sort_1000` | 3.42 µs | 6.04× |
| `vertex/bytemuck_cast_100` | M1 | 0.77 ns | `vertex/byte_view_100` | 1.30 ns | 1.69× |
| `vertex/quad_generation_100` | M2 | 633.72 ns | `vertex/quad_generation_100` | 5.79 µs | 9.14× |
| `pipeline/batch_to_vertices_100` | M2 | 1.19 µs | `pipeline/batch_to_vertices_100` | 9.14 µs | 7.69× |
| `pipeline/batch_to_vertices_1000` | M2 | 11.90 µs | `pipeline/batch_to_vertices_1000` | 91.38 µs | 7.68× |
| `pipeline/batch_to_vertices_rotated_100` | M2 | 1.75 µs | `pipeline/batch_to_vertices_rotated_100` | 17.67 µs | 10.1× |
| `pipeline/batch_to_vertices_into_1000` | M2 | 11.97 µs | `pipeline/batch_to_vertices_into_1000` | 91.71 µs | 7.66× |
| `debug_draw/wire_box_100` | M2 | 3.01 µs | `debug_draw/wire_box_100` | 23.48 µs | 7.81× |
| `debug_draw/wire_sphere_100` | M2 | 24.16 µs | `debug_draw/wire_sphere_100` | 150.54 µs | 6.23× |
| `debug_draw/grid_10x10` | M2 | 83.16 ns | `debug_draw/grid_10x10` | 925.00 ns | 11.1× |
| `terrain/generate_32x32` | M3 | 12.51 µs | `terrain/generate_32x32` | 97.92 µs | 7.83× |
| `terrain/generate_64x64` | M3 | 48.08 µs | `terrain/generate_64x64` | 381.10 µs | 7.93× |
| `terrain/generate_256x256` | M3 | 733.85 µs | `terrain/generate_256x256` | 5.87 ms | 7.99× |
| `animation/compute_joints_16` | M3 | 282.74 ns | `animation/compute_joints_16` | 4.38 µs | 15.5× |
| `animation/compute_joints_64` | M3 | 1.15 µs | `animation/compute_joints_64` | 17.68 µs | 15.4× |
| `animation/compute_joints_64_unsorted` | M3 | 1.55 µs | `animation/compute_joints_64_unsorted` | 18.53 µs | 11.9× |
| `animation/sample_rotation_64x32` | M3 | 3.42 µs | `animation/sample_rotation_64x32` | 37.58 µs | 11.0× |
| `primitives/sphere_32x16` | M3 | 5.84 µs | `primitives/sphere_32x16` | 24.68 µs | 4.23× |
| `screenshot/encode_png_256` | M8 | 461.65 µs | `screenshot/encode_png_256` — pending | — | — |
| `shadow/directional_matrix` | M4 | 20.86 ns | `shadow/directional_matrix` | 115.00 ns | 5.51× |
| `shadow/directional_matrix_centered` | M4 | 21.53 ns | `shadow/directional_matrix_centered` | 110.00 ns | 5.11× |
| `shadow/cascade_splits_4` | M4 | 40.54 ns | `shadow/cascade_splits_4` | 700.00 ns | 17.3× |
| `shadow/point_shadow_6_faces` | M4 | 213.44 ns | `shadow/point_shadow_6_faces` | 1.30 µs | 6.09× |

## Cyrius-only benches

| Cyrius bench | What it measures | Cyrius |
|---|---|---:|
| `color_optics/from_prakash` | M1 (`optics`): a prakash Rgb clamped and narrowed to a Color; no Rust bench | 20.32 ns |
| `color_optics/from_temperature` | M1 (`optics`): a blackbody colour through prakash's fit; no Rust bench | 453.00 ns |
| `color_optics/from_wavelength` | M1 (`optics`): a spectral colour through prakash; no Rust bench | 59.61 ns |
| `env/face_mip_chain_128_srgb` | M4: a 128 × 128 sRGB face's CPU mip chain (decode, 7 box filters, encodes); no Rust bench | 1.46 ms |
| `gltf/load_float_stress` | M3: both glTF loaders on float_stress.glb (125 KB of JSON, 400 joints); no Rust bench | 9.37 ms |
| `gltf/load_rich` | M3: both glTF loaders on rich.glb (4.8 KB of JSON, every feature read); no Rust bench | 393.34 µs |
| `json/color_from_json` | M1: a serde_json Color document, read (bayan's value tree); no Rust bench | 2.10 µs |
| `json/color_to_json` | M1: Color's serde document, written (bayan's round-trip digits); no Rust bench | 3.97 µs |
| `math/compose_trs` | M1: `compose_trs` in f64, rounded once; no Rust bench (animation's joint chains call it, M3) | 48.79 ns |
| `math/look_at` | M1: `look_at` in f64, rounded once; no Rust bench (`shadow/directional_matrix` calls it, M4) | 80.82 ns |
| `math/mul_mat4` | M1: `mul_mat4` in f64, rounded once (D4 rule b); no Rust bench (`shadow/*` call it, M4) | 121.00 ns |
| `math/normal_to_basis` | M1: `normal_to_basis` in f64, rounded once; no Rust bench (acoustic and em call it, M5) | 61.99 ns |
| `math/normalize3` | M1: `normalize3` in f64, rounded once; no Rust bench | 18.68 ns |
| `math/perspective_90` | M1: `perspective_90` in f64, rounded once; no Rust bench (`shadow/point_shadow_6_faces`, M4) | 32.69 ns |
| `noop` | the scaffold's harness check: an empty call, i.e. loop and call overhead (tests/soorat.bcyr) | 2.42 ns |
| `pbr/brdf_lut_32x32_256` | M4 (`optics`): a 32 × 32 BRDF LUT at 256 samples a texel (prakash 2.15.2's integration); no Rust bench | 66.98 ms |
| `render_graph/build_forward_with_effects` | M4: `forward_with_effects` built (7 passes, 8 dependencies) and dropped; no Rust bench | 1.98 µs |
| `render_graph/order_forward_with_effects` | M4: `execution_order` of `forward_with_effects` (7 passes); no Rust bench | 327.00 ns |
| `tables/f16_to_f32` | M1: binary16 → binary32 (bit manipulation); no Rust bench (environment's HDR faces, M4) | 7.07 ns |
| `tables/f32_to_f16` | M1: binary32 → binary16, ties to even; no Rust bench (environment's HDR faces, M4) | 13.33 ns |
| `tables/linear_to_srgb` | M1: sRGB encode, f32 with a correctly rounded pow; no Rust bench (environment, M4) | 218.00 ns |
| `tables/srgb8_decode` | M1: a decode-table lookup (the generated table); no Rust bench (environment, M4) | 3.63 ns |
| `tables/srgb_to_linear` | M1: sRGB decode, f32 with a correctly rounded pow; no Rust bench (environment, M4) | 215.00 ns |
