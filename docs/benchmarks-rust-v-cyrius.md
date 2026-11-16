# Benchmarks: Rust 1.2.0 vs Cyrius

> **Generated** by `python3 -I -B scripts/bench_record.py .`; `--check` gates it (port plan §4 step 7, §7).
> Speed is measured, not gated (port plan §0.1, §11): a slow row is a finding to file with its numbers.

**Rust column:** `rust-old/bench-history.csv`, run `2026-11-15T21:19:13Z` at `7f3213a`: the 1.2.0 release run (plan §4 step 8), criterion medians.

**Cyrius column:** `bench-history.csv` at the repository root (`timestamp,commit,branch,benchmark,estimate_ns`, as the Rust history), its latest run: `2026-11-16T06:26:29Z` at `eaa84e1+wip`, `cyrius bench` means. `scripts/bench_record.py . --append <cyrius bench log>` adds a run.

**35 Rust benches:** 34 twinned, 0 not applicable, 1 pending, 0 missing. Milestones done (plan §14): M0, M1, M2, M3, M4.

| Rust bench | Milestone | Rust 1.2.0 | Cyrius twin | Cyrius | Cyrius / Rust |
|---|---|---:|---|---:|---:|
| `color/from_hex` | M1 | 1.36 ns | `color/from_hex` | 20.62 ns | 15.1× |
| `color/from_rgba8` | M1 | 3.30 ns | `color/from_rgba8` | 16.54 ns | 5.02× |
| `color/lerp` | M1 | 1.05 ns | `color/lerp` | 40.98 ns | 38.9× |
| `color/luminance` | M1 | 0.76 ns | `color/luminance` | 11.66 ns | 15.3× |
| `color/to_array` | M1 | 0.52 ns | `color/to_array` | 5.16 ns | 9.96× |
| `sprite/create` | M2 | 2.39 ns | `sprite/create` | 39.61 ns | 16.6× |
| `sprite/create_full_builder` | M2 | 3.54 ns | `sprite/create_full_builder` | 47.32 ns | 13.4× |
| `sprite/center_bounds` | M2 | 0.25 ns | `sprite/center_bounds` | 32.15 ns | 127× |
| `sprite/batch_push_100` | M2 | 836.95 ns | `sprite/batch_push_100` | 5.14 µs | 6.14× |
| `sprite/batch_push_100_prealloc` | M2 | 841.34 ns | `sprite/batch_push_100_prealloc` | 5.11 µs | 6.08× |
| `sprite/batch_sort_100` | M2 | 73.57 ns | `sprite/batch_sort_100` | 335.00 ns | 4.55× |
| `sprite/batch_push_1000` | M2 | 8.31 µs | `sprite/batch_push_1000` | 50.98 µs | 6.14× |
| `sprite/batch_sort_1000` | M2 | 565.87 ns | `sprite/batch_sort_1000` | 3.63 µs | 6.42× |
| `vertex/bytemuck_cast_100` | M1 | 0.77 ns | `vertex/byte_view_100` | 1.36 ns | 1.76× |
| `vertex/quad_generation_100` | M2 | 633.72 ns | `vertex/quad_generation_100` | 6.04 µs | 9.54× |
| `pipeline/batch_to_vertices_100` | M2 | 1.19 µs | `pipeline/batch_to_vertices_100` | 9.74 µs | 8.19× |
| `pipeline/batch_to_vertices_1000` | M2 | 11.90 µs | `pipeline/batch_to_vertices_1000` | 96.59 µs | 8.11× |
| `pipeline/batch_to_vertices_rotated_100` | M2 | 1.75 µs | `pipeline/batch_to_vertices_rotated_100` | 18.73 µs | 10.7× |
| `pipeline/batch_to_vertices_into_1000` | M2 | 11.97 µs | `pipeline/batch_to_vertices_into_1000` | 97.69 µs | 8.16× |
| `debug_draw/wire_box_100` | M2 | 3.01 µs | `debug_draw/wire_box_100` | 24.26 µs | 8.07× |
| `debug_draw/wire_sphere_100` | M2 | 24.16 µs | `debug_draw/wire_sphere_100` | 160.93 µs | 6.66× |
| `debug_draw/grid_10x10` | M2 | 83.16 ns | `debug_draw/grid_10x10` | 980.00 ns | 11.8× |
| `terrain/generate_32x32` | M3 | 12.51 µs | `terrain/generate_32x32` | 102.60 µs | 8.20× |
| `terrain/generate_64x64` | M3 | 48.08 µs | `terrain/generate_64x64` | 386.83 µs | 8.05× |
| `terrain/generate_256x256` | M3 | 733.85 µs | `terrain/generate_256x256` | 6.07 ms | 8.27× |
| `animation/compute_joints_16` | M3 | 282.74 ns | `animation/compute_joints_16` | 4.45 µs | 15.7× |
| `animation/compute_joints_64` | M3 | 1.15 µs | `animation/compute_joints_64` | 17.84 µs | 15.5× |
| `animation/compute_joints_64_unsorted` | M3 | 1.55 µs | `animation/compute_joints_64_unsorted` | 19.35 µs | 12.5× |
| `animation/sample_rotation_64x32` | M3 | 3.42 µs | `animation/sample_rotation_64x32` | 40.11 µs | 11.7× |
| `primitives/sphere_32x16` | M3 | 5.84 µs | `primitives/sphere_32x16` | 25.29 µs | 4.33× |
| `screenshot/encode_png_256` | M8 | 461.65 µs | `screenshot/encode_png_256` — pending | — | — |
| `shadow/directional_matrix` | M4 | 20.86 ns | `shadow/directional_matrix` | 118.00 ns | 5.66× |
| `shadow/directional_matrix_centered` | M4 | 21.53 ns | `shadow/directional_matrix_centered` | 113.00 ns | 5.25× |
| `shadow/cascade_splits_4` | M4 | 40.54 ns | `shadow/cascade_splits_4` | 725.00 ns | 17.9× |
| `shadow/point_shadow_6_faces` | M4 | 213.44 ns | `shadow/point_shadow_6_faces` | 1.41 µs | 6.61× |

## Cyrius-only benches

| Cyrius bench | What it measures | Cyrius |
|---|---|---:|
| `acoustic/balloon_32x64` | M5 (`acoustics`): a 32 × 64 directivity balloon (2,145 vertices, correctly rounded trig); no Rust bench | 1.83 ms |
| `acoustic/coupled_decay_100` | M5 (`acoustics`): the default 100-step coupled decay curve (correctly rounded exp and log10); no Rust bench | 31.62 µs |
| `acoustic/mode_pattern_32x32` | M5 (`acoustics`): a 32 × 32 goonj room-mode pattern as a height-field mesh; no Rust bench | 160.29 µs |
| `acoustic/portal` | M5 (`acoustics`): one goonj portal drawn as 7 lines; no Rust bench | 324.00 ns |
| `acoustic/pressure_slice_32x32` | M5 (`acoustics`): a 32 × 32 slice of a goonj pressure map as a heat-map mesh; no Rust bench | 191.05 µs |
| `acoustic/ray_paths_64x8` | M5 (`acoustics`): 64 goonj ray paths of 8 bounces each, drawn as 512 lines; no Rust bench | 81.00 µs |
| `color_optics/from_prakash` | M1 (`optics`): a prakash Rgb clamped and narrowed to a Color; no Rust bench | 22.47 ns |
| `color_optics/from_temperature` | M1 (`optics`): a blackbody colour through prakash's fit; no Rust bench | 465.00 ns |
| `color_optics/from_wavelength` | M1 (`optics`): a spectral colour through prakash; no Rust bench | 73.78 ns |
| `env/face_mip_chain_128_srgb` | M4: a 128 × 128 sRGB face's CPU mip chain (decode, 7 box filters, encodes); no Rust bench | 1.58 ms |
| `fluid/particles_billboards_1000` | M5 (`fluids`): 1,000 pravash particles as camera-facing billboards; no Rust bench | 277.94 µs |
| `fluid/particles_quads_1000` | M5 (`fluids`): 1,000 pravash particles as velocity-coloured XZ quads; no Rust bench | 303.27 µs |
| `fluid/shallow_water_64x64` | M5 (`fluids`): a 64 × 64 pravash shallow-water surface as a height-field mesh; no Rust bench | 476.43 µs |
| `gltf/load_float_stress` | M3: both glTF loaders on float_stress.glb (125 KB of JSON, 400 joints); no Rust bench | 9.78 ms |
| `gltf/load_rich` | M3: both glTF loaders on rich.glb (4.8 KB of JSON, every feature read); no Rust bench | 419.79 µs |
| `json/color_from_json` | M1: a serde_json Color document, read (bayan's value tree); no Rust bench | 2.26 µs |
| `json/color_to_json` | M1: Color's serde document, written (bayan's round-trip digits); no Rust bench | 4.22 µs |
| `math/compose_trs` | M1: `compose_trs` in f64, rounded once; no Rust bench (animation's joint chains call it, M3) | 49.44 ns |
| `math/look_at` | M1: `look_at` in f64, rounded once; no Rust bench (`shadow/directional_matrix` calls it, M4) | 81.72 ns |
| `math/mul_mat4` | M1: `mul_mat4` in f64, rounded once (D4 rule b); no Rust bench (`shadow/*` call it, M4) | 122.00 ns |
| `math/normal_to_basis` | M1: `normal_to_basis` in f64, rounded once; no Rust bench (acoustic and em call it, M5) | 76.70 ns |
| `math/normalize3` | M1: `normalize3` in f64, rounded once; no Rust bench | 18.79 ns |
| `math/perspective_90` | M1: `perspective_90` in f64, rounded once; no Rust bench (`shadow/point_shadow_6_faces`, M4) | 32.84 ns |
| `noop` | the scaffold's harness check: an empty call, i.e. loop and call overhead (tests/soorat.bcyr) | 2.73 ns |
| `pbr/brdf_lut_32x32_256` | M4 (`optics`): a 32 × 32 BRDF LUT at 256 samples a texel (prakash 2.15.2's integration); no Rust bench | 72.69 ms |
| `render_graph/build_forward_with_effects` | M4: `forward_with_effects` built (7 passes, 8 dependencies) and dropped; no Rust bench | 2.11 µs |
| `render_graph/order_forward_with_effects` | M4: `execution_order` of `forward_with_effects` (7 passes); no Rust bench | 336.00 ns |
| `tables/f16_to_f32` | M1: binary16 → binary32 (bit manipulation); no Rust bench (environment's HDR faces, M4) | 7.54 ns |
| `tables/f32_to_f16` | M1: binary32 → binary16, ties to even; no Rust bench (environment's HDR faces, M4) | 15.87 ns |
| `tables/linear_to_srgb` | M1: sRGB encode, f32 with a correctly rounded pow; no Rust bench (environment, M4) | 225.00 ns |
| `tables/srgb8_decode` | M1: a decode-table lookup (the generated table); no Rust bench (environment, M4) | 3.79 ns |
| `tables/srgb_to_linear` | M1: sRGB decode, f32 with a correctly rounded pow; no Rust bench (environment, M4) | 223.00 ns |
| `thermo/flux_32x32` | M5 (`thermo`): a 32 × 32 ushma heat-flux field as arrows (3 lines each); no Rust bench | 209.57 µs |
| `thermo/grid_32x32` | M5 (`thermo`): a 32 × 32 ushma thermal grid as a heat-map mesh; no Rust bench | 173.38 µs |
| `thermo/network_32` | M5 (`thermo`): a 32-node ring network (correctly rounded cos and sin); no Rust bench | 15.49 µs |
| `thermo/profile_100` | M5 (`thermo`): a 100-sample temperature profile as 99 lines; no Rust bench | 9.77 µs |
