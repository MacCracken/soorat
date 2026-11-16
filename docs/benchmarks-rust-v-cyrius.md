# Benchmarks: Rust 1.2.0 vs Cyrius

> **Generated** by `python3 -I -B scripts/bench_record.py .`; `--check` gates it (port plan §4 step 7, §7).
> Speed is measured, not gated (port plan §0.1, §11): a slow row is a finding to file with its numbers.

**Rust column:** `rust-old/bench-history.csv`, run `2026-11-15T21:19:13Z` at `7f3213a`: the 1.2.0 release run (plan §4 step 8), criterion medians.

**Cyrius column:** `bench-history.csv` at the repository root (`timestamp,commit,branch,benchmark,estimate_ns`, as the Rust history), its latest run: `2026-11-16T04:10:32Z` at `54175b2+wip`, `cyrius bench` means. `scripts/bench_record.py . --append <cyrius bench log>` adds a run.

**35 Rust benches:** 34 twinned, 0 not applicable, 1 pending, 0 missing. Milestones done (plan §14): M0, M1, M2, M3.

| Rust bench | Milestone | Rust 1.2.0 | Cyrius twin | Cyrius | Cyrius / Rust |
|---|---|---:|---|---:|---:|
| `color/from_hex` | M1 | 1.36 ns | `color/from_hex` | 19.45 ns | 14.3× |
| `color/from_rgba8` | M1 | 3.30 ns | `color/from_rgba8` | 14.71 ns | 4.46× |
| `color/lerp` | M1 | 1.05 ns | `color/lerp` | 37.51 ns | 35.6× |
| `color/luminance` | M1 | 0.76 ns | `color/luminance` | 10.68 ns | 14.0× |
| `color/to_array` | M1 | 0.52 ns | `color/to_array` | 4.76 ns | 9.19× |
| `sprite/create` | M2 | 2.39 ns | `sprite/create` | 36.26 ns | 15.2× |
| `sprite/create_full_builder` | M2 | 3.54 ns | `sprite/create_full_builder` | 42.07 ns | 11.9× |
| `sprite/center_bounds` | M2 | 0.25 ns | `sprite/center_bounds` | 28.83 ns | 113× |
| `sprite/batch_push_100` | M2 | 836.95 ns | `sprite/batch_push_100` | 4.57 µs | 5.46× |
| `sprite/batch_push_100_prealloc` | M2 | 841.34 ns | `sprite/batch_push_100_prealloc` | 4.58 µs | 5.45× |
| `sprite/batch_sort_100` | M2 | 73.57 ns | `sprite/batch_sort_100` | 310.00 ns | 4.21× |
| `sprite/batch_push_1000` | M2 | 8.31 µs | `sprite/batch_push_1000` | 45.96 µs | 5.53× |
| `sprite/batch_sort_1000` | M2 | 565.87 ns | `sprite/batch_sort_1000` | 3.35 µs | 5.91× |
| `vertex/bytemuck_cast_100` | M1 | 0.77 ns | `vertex/byte_view_100` | 1.23 ns | 1.60× |
| `vertex/quad_generation_100` | M2 | 633.72 ns | `vertex/quad_generation_100` | 5.44 µs | 8.59× |
| `pipeline/batch_to_vertices_100` | M2 | 1.19 µs | `pipeline/batch_to_vertices_100` | 8.75 µs | 7.36× |
| `pipeline/batch_to_vertices_1000` | M2 | 11.90 µs | `pipeline/batch_to_vertices_1000` | 87.78 µs | 7.37× |
| `pipeline/batch_to_vertices_rotated_100` | M2 | 1.75 µs | `pipeline/batch_to_vertices_rotated_100` | 17.03 µs | 9.73× |
| `pipeline/batch_to_vertices_into_1000` | M2 | 11.97 µs | `pipeline/batch_to_vertices_into_1000` | 87.41 µs | 7.30× |
| `debug_draw/wire_box_100` | M2 | 3.01 µs | `debug_draw/wire_box_100` | 22.71 µs | 7.55× |
| `debug_draw/wire_sphere_100` | M2 | 24.16 µs | `debug_draw/wire_sphere_100` | 145.67 µs | 6.03× |
| `debug_draw/grid_10x10` | M2 | 83.16 ns | `debug_draw/grid_10x10` | 888.00 ns | 10.7× |
| `terrain/generate_32x32` | M3 | 12.51 µs | `terrain/generate_32x32` | 94.40 µs | 7.55× |
| `terrain/generate_64x64` | M3 | 48.08 µs | `terrain/generate_64x64` | 362.00 µs | 7.53× |
| `terrain/generate_256x256` | M3 | 733.85 µs | `terrain/generate_256x256` | 5.64 ms | 7.68× |
| `animation/compute_joints_16` | M3 | 282.74 ns | `animation/compute_joints_16` | 4.16 µs | 14.7× |
| `animation/compute_joints_64` | M3 | 1.15 µs | `animation/compute_joints_64` | 16.54 µs | 14.4× |
| `animation/compute_joints_64_unsorted` | M3 | 1.55 µs | `animation/compute_joints_64_unsorted` | 17.54 µs | 11.3× |
| `animation/sample_rotation_64x32` | M3 | 3.42 µs | `animation/sample_rotation_64x32` | 36.39 µs | 10.7× |
| `primitives/sphere_32x16` | M3 | 5.84 µs | `primitives/sphere_32x16` | 24.04 µs | 4.12× |
| `screenshot/encode_png_256` | M8 | 461.65 µs | `screenshot/encode_png_256` — pending | — | — |
| `shadow/directional_matrix` | M4 | 20.86 ns | `shadow/directional_matrix` | 111.00 ns | 5.32× |
| `shadow/directional_matrix_centered` | M4 | 21.53 ns | `shadow/directional_matrix_centered` | 105.00 ns | 4.88× |
| `shadow/cascade_splits_4` | M4 | 40.54 ns | `shadow/cascade_splits_4` | 670.00 ns | 16.5× |
| `shadow/point_shadow_6_faces` | M4 | 213.44 ns | `shadow/point_shadow_6_faces` | 1.26 µs | 5.93× |

## Cyrius-only benches

| Cyrius bench | What it measures | Cyrius |
|---|---|---:|
| `color_optics/from_prakash` | M1 (`optics`): a prakash Rgb clamped and narrowed to a Color; no Rust bench | 18.44 ns |
| `color_optics/from_temperature` | M1 (`optics`): a blackbody colour through prakash's fit; no Rust bench | 433.00 ns |
| `color_optics/from_wavelength` | M1 (`optics`): a spectral colour through prakash; no Rust bench | 58.55 ns |
| `env/face_mip_chain_128_srgb` | M4: a 128 × 128 sRGB face's CPU mip chain (decode, 7 box filters, encodes); no Rust bench | 1.40 ms |
| `gltf/load_float_stress` | M3: both glTF loaders on float_stress.glb (125 KB of JSON, 400 joints); no Rust bench | 8.98 ms |
| `gltf/load_rich` | M3: both glTF loaders on rich.glb (4.8 KB of JSON, every feature read); no Rust bench | 386.45 µs |
| `json/color_from_json` | M1: a serde_json Color document, read (bayan's value tree); no Rust bench | 2.02 µs |
| `json/color_to_json` | M1: Color's serde document, written (bayan's round-trip digits); no Rust bench | 3.79 µs |
| `math/compose_trs` | M1: `compose_trs` in f64, rounded once; no Rust bench (animation's joint chains call it, M3) | 46.96 ns |
| `math/look_at` | M1: `look_at` in f64, rounded once; no Rust bench (`shadow/directional_matrix` calls it, M4) | 76.32 ns |
| `math/mul_mat4` | M1: `mul_mat4` in f64, rounded once (D4 rule b); no Rust bench (`shadow/*` call it, M4) | 116.00 ns |
| `math/normal_to_basis` | M1: `normal_to_basis` in f64, rounded once; no Rust bench (acoustic and em call it, M5) | 61.31 ns |
| `math/normalize3` | M1: `normalize3` in f64, rounded once; no Rust bench | 17.72 ns |
| `math/perspective_90` | M1: `perspective_90` in f64, rounded once; no Rust bench (`shadow/point_shadow_6_faces`, M4) | 31.20 ns |
| `noop` | the scaffold's harness check: an empty call, i.e. loop and call overhead (tests/soorat.bcyr) | 2.51 ns |
| `pbr/brdf_lut_32x32_256` | M4 (`optics`): a 32 × 32 BRDF LUT at 256 samples a texel (prakash 1.2.0's formula); no Rust bench | 60.78 ms |
| `tables/f16_to_f32` | M1: binary16 → binary32 (bit manipulation); no Rust bench (environment's HDR faces, M4) | 6.95 ns |
| `tables/f32_to_f16` | M1: binary32 → binary16, ties to even; no Rust bench (environment's HDR faces, M4) | 12.65 ns |
| `tables/linear_to_srgb` | M1: sRGB encode, f32 with a correctly rounded pow; no Rust bench (environment, M4) | 207.00 ns |
| `tables/srgb8_decode` | M1: a decode-table lookup (the generated table); no Rust bench (environment, M4) | 3.54 ns |
| `tables/srgb_to_linear` | M1: sRGB decode, f32 with a correctly rounded pow; no Rust bench (environment, M4) | 203.00 ns |
