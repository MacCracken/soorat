# render_ref: the class-G render-reference corpus

This is M0 step 6 of the Cyrius port (`docs/development/soorat-port-plan.md` §4 step 6, §2.2 and §5, class **G**).
It renders a fixed set of scenes offscreen with the **frozen Rust soorat 1.2.0 oracle** (`rust-old/`), using only
soorat's public API, and records each RGBA8 readback in `tests/golden/render/`. The Cyrius port renders the same
scenes through mabda and compares pixels on the same machine.

## Commands

```sh
cd scripts/render_ref
cargo build --release --locked

./target/release/render_ref           # render twice, prove the runs byte-identical, (re)write the corpus
./target/release/render_ref --check   # re-render and compare with the stored corpus
./target/release/render_ref --corpus DIR [--check]   # another corpus directory (it must hold scenes.json)
```

Exit codes:

| Code | Meaning |
|---|---|
| 0 | The corpus was written, or `--check` found every scene identical |
| 1 | A difference, a run that wasn't deterministic (nothing is written then), or a stale corpus (`scenes.json` changed since recording) |
| 2 | Refused: the adapter, driver or stack differs from the recorded header, there is no adapter or no corpus, the arguments are bad, or `scenes.json` breaks a rule |

`--check` prints, for each scene that differs, the count of differing pixels, the maximum channel delta and the first
differing pixel. It also names any upload whose bytes changed.

## Rules

- **Never edit `rust-old/`.** It is the oracle. This crate depends on it by path, and its `Cargo.lock` was seeded from
  1.2.0's own lock: every shared dependency is the oracle's exact version (wgpu 29.0.4, mabda 1.0.0, image 0.25.10,
  …). The lock dropped the packages this crate doesn't use and added only `sha2` 0.10.9 and its dependencies. Always
  build with `--locked`.
- **Regenerate only from the oracle**, only with `render_ref` and only on the recorded machine. Never edit the PNGs,
  `manifest.tsv` or `uploads.tsv` by hand.
- **`tests/golden/render/scenes.json` is the definition.** Change scenes there, never in code.
  - Every number must be exactly representable as an f32, written as its exact decimal. `render_ref` refuses the file
    otherwise.
  - Camera matrices come from `derive_camera.py` (Python 3 standard library only).
  - After any change, rerun `render_ref`. `--check` reports a changed `scenes.json` as a stale corpus.
- **Class G belongs to one machine.**
  - The manifest header records the adapter, its vendor and device ids, the device type, the backend, the driver and
    driver info (`wgpu::AdapterInfo`), the Mesa version (parsed from the driver info), glibc, and the wgpu and soorat
    versions.
  - glibc is recorded because the CPU-side sin/cos/tan in the uploads go through it.
  - `--check` refuses (exit 2) when any of these differs.
  - CI does not run class G (§5).

## Scenes

The scenes are 128×128, `Rgba8UnormSrgb` unless noted. `tests/golden/render/scenes.md` gives the exact call
sequence for each scene kind.

| Scene | Status | Content |
|---|---|---|
| `sprite_grid` | recorded | 6×6 tinted, translucent sprites with atlas UVs, plus three z-sorted overlays (one rotated); 4×4 sRGB atlas |
| `debug_lines` | recorded | grid, circle, box, wire sphere and axes (`LinePipeline`) over a depth-cleared target |
| `pbr_lit` | recorded | checker-textured ellipsoid (rotation + non-uniform scale → normal matrix); directional + point + spot lights |
| `shadow_mesh` | recorded | plane + cube + sphere; 512² directional shadow map; PCF |
| `ibl_sphere` | recorded | metallic sphere lit by IBL only: flat irradiance cube, 16² gradient prefiltered cube (5 mips), prakash BRDF LUT |
| `post_bloom_tonemap` | recorded | glossy HDR sphere (`Rgba16Float`) → bloom threshold/blur → `set_bloom` composite → ACES |
| `post_ssao` | recorded (`Rgba8Unorm`) | SSAO, 16 samples, from the depth of the plane/cube/sphere scene; constant +Z normal (1.2.0 has no normal pass) |
| `gpu_particles` | recorded | 64 xorshift32-seeded particles; 32 compute steps of 1/64 s; drawn as sprites |
| `science_acoustic` | recorded | goonj room-mode pattern → `mode_pattern_to_mesh` (heat-mapped height field) |
| `science_thermo` | recorded | ushma thermal grid → `thermal_grid_to_mesh` |
| `science_fluid` | recorded | pravash shallow water → `shallow_water_to_mesh` |
| `science_em` | recorded | bijli Ez slice → `field_slice_2d_to_mesh` (signed) |
| `skinned_mesh` | **skipped** | 1.2.0 has no public skinned pipeline (`pbr_skinned.wgsl` is only compiled by a smoke test; nothing binds `JointUniforms`) |

## Output (`tests/golden/render/`)

| File | Content |
|---|---|
| `scenes.json` | the scene definitions, read by the renderer |
| `scenes.md` | the meaning of every field and the call sequence for each scene |
| `<scene>.png` | lossless RGBA8 readback |
| `manifest.tsv` | the platform header (`# key<TAB>value`), then `scene width height format sha256 description`, where sha256 is over the raw RGBA bytes |
| `uploads.tsv` | every CPU-side buffer each scene uploads (vertices, indices, uniforms, texels, particle state), with its size, sha256 and hex (when ≤ 256 B), so the Cyrius side can check its uploads before its pixels |

The whole corpus is about 130 KB.

## Determinism

The write mode renders all scenes twice, each run on its own `GpuContext`. It writes only when both runs are
byte-identical, pixels and uploads alike. `--check` adds a third run in a separate process.

On the dev box (AMD Radeon Graphics, RADV RENOIR, Mesa 26.2.6), every scene was identical across all runs, and two
separate writes produced byte-identical files.
