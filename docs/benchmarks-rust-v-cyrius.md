# Benchmarks: Rust 1.2.0 vs Cyrius

> **Generated** by `python3 -I -B scripts/bench_record.py .`; `--check` gates it (port plan §4 step 7, §7).
> Speed is measured, not gated (port plan §0.1, §11): a slow row is a finding to file with its numbers.

**Rust column:** `rust-old/bench-history.csv`, run `2026-11-15T21:19:13Z` at `7f3213a`: the 1.2.0 release run (plan §4 step 8), criterion medians.

**Cyrius column:** `bench-history.csv` at the repository root (`timestamp,commit,branch,benchmark,estimate_ns`, as the Rust history), its latest run: `2026-11-19T08:58:18Z` at `9b95cb5+wip`, `cyrius bench` means. `scripts/bench_record.py . --append <cyrius bench log>` adds a run.

**35 Rust benches:** 34 twinned, 0 not applicable, 1 pending, 0 missing. Milestones done (plan §14): M0, M1, M2, M3, M4, M5.

| Rust bench | Milestone | Rust 1.2.0 | Cyrius twin | Cyrius | Cyrius / Rust |
|---|---|---:|---|---:|---:|
| `color/from_hex` | M1 | 1.36 ns | `color/from_hex` | 18.84 ns | 13.8× |
| `color/from_rgba8` | M1 | 3.30 ns | `color/from_rgba8` | 14.25 ns | 4.32× |
| `color/lerp` | M1 | 1.05 ns | `color/lerp` | 77.36 ns | 73.4× |
| `color/luminance` | M1 | 0.76 ns | `color/luminance` | 26.97 ns | 35.4× |
| `color/to_array` | M1 | 0.52 ns | `color/to_array` | 4.53 ns | 8.74× |
| `sprite/create` | M2 | 2.39 ns | `sprite/create` | 35.25 ns | 14.8× |
| `sprite/create_full_builder` | M2 | 3.54 ns | `sprite/create_full_builder` | 40.66 ns | 11.5× |
| `sprite/center_bounds` | M2 | 0.25 ns | `sprite/center_bounds` | 28.37 ns | 112× |
| `sprite/batch_push_100` | M2 | 836.95 ns | `sprite/batch_push_100` | 4.51 µs | 5.39× |
| `sprite/batch_push_100_prealloc` | M2 | 841.34 ns | `sprite/batch_push_100_prealloc` | 4.47 µs | 5.32× |
| `sprite/batch_sort_100` | M2 | 73.57 ns | `sprite/batch_sort_100` | 297.00 ns | 4.04× |
| `sprite/batch_push_1000` | M2 | 8.31 µs | `sprite/batch_push_1000` | 45.92 µs | 5.53× |
| `sprite/batch_sort_1000` | M2 | 565.87 ns | `sprite/batch_sort_1000` | 3.20 µs | 5.65× |
| `vertex/bytemuck_cast_100` | M1 | 0.77 ns | `vertex/byte_view_100` | 1.20 ns | 1.56× |
| `vertex/quad_generation_100` | M2 | 633.72 ns | `vertex/quad_generation_100` | 5.45 µs | 8.60× |
| `pipeline/batch_to_vertices_100` | M2 | 1.19 µs | `pipeline/batch_to_vertices_100` | 8.60 µs | 7.23× |
| `pipeline/batch_to_vertices_1000` | M2 | 11.90 µs | `pipeline/batch_to_vertices_1000` | 85.13 µs | 7.15× |
| `pipeline/batch_to_vertices_rotated_100` | M2 | 1.75 µs | `pipeline/batch_to_vertices_rotated_100` | 16.49 µs | 9.42× |
| `pipeline/batch_to_vertices_into_1000` | M2 | 11.97 µs | `pipeline/batch_to_vertices_into_1000` | 85.31 µs | 7.12× |
| `debug_draw/wire_box_100` | M2 | 3.01 µs | `debug_draw/wire_box_100` | 21.89 µs | 7.28× |
| `debug_draw/wire_sphere_100` | M2 | 24.16 µs | `debug_draw/wire_sphere_100` | 140.91 µs | 5.83× |
| `debug_draw/grid_10x10` | M2 | 83.16 ns | `debug_draw/grid_10x10` | 863.00 ns | 10.4× |
| `terrain/generate_32x32` | M3 | 12.51 µs | `terrain/generate_32x32` | 91.83 µs | 7.34× |
| `terrain/generate_64x64` | M3 | 48.08 µs | `terrain/generate_64x64` | 350.34 µs | 7.29× |
| `terrain/generate_256x256` | M3 | 733.85 µs | `terrain/generate_256x256` | 5.48 ms | 7.47× |
| `animation/compute_joints_16` | M3 | 282.74 ns | `animation/compute_joints_16` | 3.99 µs | 14.1× |
| `animation/compute_joints_64` | M3 | 1.15 µs | `animation/compute_joints_64` | 15.95 µs | 13.9× |
| `animation/compute_joints_64_unsorted` | M3 | 1.55 µs | `animation/compute_joints_64_unsorted` | 17.37 µs | 11.2× |
| `animation/sample_rotation_64x32` | M3 | 3.42 µs | `animation/sample_rotation_64x32` | 35.31 µs | 10.3× |
| `primitives/sphere_32x16` | M3 | 5.84 µs | `primitives/sphere_32x16` | 23.30 µs | 3.99× |
| `screenshot/encode_png_256` | M8 | 461.65 µs | `screenshot/encode_png_256` — pending | — | — |
| `shadow/directional_matrix` | M4 | 20.86 ns | `shadow/directional_matrix` | 107.00 ns | 5.13× |
| `shadow/directional_matrix_centered` | M4 | 21.53 ns | `shadow/directional_matrix_centered` | 102.00 ns | 4.74× |
| `shadow/cascade_splits_4` | M4 | 40.54 ns | `shadow/cascade_splits_4` | 644.00 ns | 15.9× |
| `shadow/point_shadow_6_faces` | M4 | 213.44 ns | `shadow/point_shadow_6_faces` | 1.21 µs | 5.68× |

## Cyrius-only benches

| Cyrius bench | What it measures | Cyrius |
|---|---|---:|
| `acoustic/balloon_32x64` | M5 (`acoustics`): a 32 × 64 directivity balloon (2,145 vertices, correctly rounded trig); no Rust bench | 1.64 ms |
| `acoustic/coupled_decay_100` | M5 (`acoustics`): the default 100-step coupled decay curve (correctly rounded exp and log10); no Rust bench | 28.96 µs |
| `acoustic/mode_pattern_32x32` | M5 (`acoustics`): a 32 × 32 goonj room-mode pattern as a height-field mesh; no Rust bench | 145.75 µs |
| `acoustic/portal` | M5 (`acoustics`): one goonj portal drawn as 7 lines; no Rust bench | 295.00 ns |
| `acoustic/pressure_slice_32x32` | M5 (`acoustics`): a 32 × 32 slice of a goonj pressure map as a heat-map mesh; no Rust bench | 168.26 µs |
| `acoustic/ray_paths_64x8` | M5 (`acoustics`): 64 goonj ray paths of 8 bounces each, drawn as 512 lines; no Rust bench | 75.08 µs |
| `color_optics/from_prakash` | M1 (`optics`): a prakash Rgb clamped and narrowed to a Color; no Rust bench | 17.39 ns |
| `color_optics/from_temperature` | M1 (`optics`): a blackbody colour through prakash's fit; no Rust bench | 415.00 ns |
| `color_optics/from_wavelength` | M1 (`optics`): a spectral colour through prakash; no Rust bench | 55.27 ns |
| `env/face_mip_chain_128_srgb` | M4: a 128 × 128 sRGB face's CPU mip chain (decode, 7 box filters, encodes); no Rust bench | 1.36 ms |
| `fluid/particles_billboards_1000` | M5 (`fluids`): 1,000 pravash particles as camera-facing billboards; no Rust bench | 255.81 µs |
| `fluid/particles_quads_1000` | M5 (`fluids`): 1,000 pravash particles as velocity-coloured XZ quads; no Rust bench | 282.08 µs |
| `fluid/shallow_water_64x64` | M5 (`fluids`): a 64 × 64 pravash shallow-water surface as a height-field mesh; no Rust bench | 438.79 µs |
| `gltf/load_float_stress` | M3: both glTF loaders on float_stress.glb (125 KB of JSON, 400 joints); no Rust bench | 8.73 ms |
| `gltf/load_rich` | M3: both glTF loaders on rich.glb (4.8 KB of JSON, every feature read); no Rust bench | 376.69 µs |
| `json/color_from_json` | M1: a serde_json Color document, read (bayan's value tree); no Rust bench | 1.96 µs |
| `json/color_to_json` | M1: Color's serde document, written (bayan's round-trip digits); no Rust bench | 3.70 µs |
| `math/compose_trs` | M1: `compose_trs` in f64, rounded once; no Rust bench (animation's joint chains call it, M3) | 45.75 ns |
| `math/look_at` | M1: `look_at` in f64, rounded once; no Rust bench (`shadow/directional_matrix` calls it, M4) | 74.41 ns |
| `math/mul_mat4` | M1: `mul_mat4` in f64, rounded once (D4 rule b); no Rust bench (`shadow/*` call it, M4) | 112.00 ns |
| `math/normal_to_basis` | M1: `normal_to_basis` in f64, rounded once; no Rust bench (acoustic and em call it, M5) | 69.96 ns |
| `math/normalize3` | M1: `normalize3` in f64, rounded once; no Rust bench | 17.16 ns |
| `math/perspective_90` | M1: `perspective_90` in f64, rounded once; no Rust bench (`shadow/point_shadow_6_faces`, M4) | 30.23 ns |
| `noop` | the scaffold's harness check: an empty call, i.e. loop and call overhead (tests/soorat.bcyr) | 2.33 ns |
| `pbr/brdf_lut_32x32_256` | M4 (`optics`): a 32 × 32 BRDF LUT at 256 samples a texel (prakash 2.15.2's integration); no Rust bench | 61.90 ms |
| `render_graph/build_forward_with_effects` | M4: `forward_with_effects` built (7 passes, 8 dependencies) and dropped; no Rust bench | 1.89 µs |
| `render_graph/order_forward_with_effects` | M4: `execution_order` of `forward_with_effects` (7 passes); no Rust bench | 340.00 ns |
| `tables/f16_to_f32` | M1: binary16 → binary32 (bit manipulation); no Rust bench (environment's HDR faces, M4) | 6.63 ns |
| `tables/f32_to_f16` | M1: binary32 → binary16, ties to even; no Rust bench (environment's HDR faces, M4) | 12.29 ns |
| `tables/linear_to_srgb` | M1: sRGB encode, f32 with a correctly rounded pow; no Rust bench (environment, M4) | 213.00 ns |
| `tables/srgb8_decode` | M1: a decode-table lookup (the generated table); no Rust bench (environment, M4) | 3.31 ns |
| `tables/srgb_to_linear` | M1: sRGB decode, f32 with a correctly rounded pow; no Rust bench (environment, M4) | 209.00 ns |
| `thermo/flux_32x32` | M5 (`thermo`): a 32 × 32 ushma heat-flux field as arrows (3 lines each); no Rust bench | 189.73 µs |
| `thermo/grid_32x32` | M5 (`thermo`): a 32 × 32 ushma thermal grid as a heat-map mesh; no Rust bench | 156.25 µs |
| `thermo/network_32` | M5 (`thermo`): a 32-node ring network (correctly rounded cos and sin); no Rust bench | 13.98 µs |
| `thermo/profile_100` | M5 (`thermo`): a 100-sample temperature profile as 99 lines; no Rust bench | 8.92 µs |
