# soorat — Current State

> Refreshed every milestone. CLAUDE.md is preferences, process and procedures (durable); this file is
> **state** (volatile).

## Version

- **Rust 1.2.0** (tag `1.2.0`, `6625dc2`, released 2026-11-15) is the **final Rust release** and the oracle,
  frozen in `rust-old/`. It is never edited.
- **The Cyrius port** targets **2.0.0**. `VERSION` stays `1.2.0` until the 2.0.0 release (M10);
  `cyrius.cyml` reads it through `${file:VERSION}`.

## Port progress

Plan of record: [`soorat-port-plan.md`](soorat-port-plan.md). Measurements: [`port-parity.md`](port-parity.md).
Decisions: [ADR 0001](../adr/0001-cyrius-port-with-rust-oracle.md),
[ADR 0002](../adr/0002-port-api-divergences.md).

| Step | Status |
|---|---|
| D1–D9 | ✅ decided 2026-11-15 |
| M0 step 0: freeze the oracle | ✅ 1.2.0 (the oracle hooks, damping and bloom fixes) |
| M0 step 1: probes P1–P11 | ✅ 2026-11-15, port-parity §1 |
| M0 step 2: `cyrius port` + repairs | ✅ 2026-11-15 (below) |
| M0 steps 3–8: manifest, golden generator, harness, render references, tooling, baselines | ✅ 2026-11-15: **M0 complete** |
| M1 parts a–c: error, log, math, mathx, layout, color (core), lights, lod | ✅ 2026-11-15 (below) |
| M1 part d: f16 and sRGB (`tables`) | ✅ 2026-11-15 (below) |
| M1 part b2: colour `optics` + serde (prakash, bayan) | ✅ 2026-11-15: **M1 complete** |
| M2 part a: sprite, batch | ✅ 2026-11-15 (below) |
| M2 part b: text, ui, debug geometry | ✅ 2026-11-15: **M2 complete** |
| M3 part a: primitives, viz, terrain | ✅ 2026-11-15 (below) |
| M3 part b: animation (no loader), crmath | ✅ 2026-11-15 (below) |
| M3 part c: the glTF loader (`gltf` profile) | ✅ 2026-11-15: **M3 complete** |
| M4 part a: shadow math | ✅ 2026-11-15 (below) |
| M4 part b: environment CPU, PBR material and LUT | ✅ 2026-11-15 (below) |
| M4 part c: the render graph (profiler and capabilities moved to M6) | ✅ 2026-11-15: **M4 complete** |
| M5 part a: acoustic types and line renderers (`acoustics`, goonj) | ✅ 2026-11-15 (below) |
| M5 part b: acoustic meshes (`acoustic_render` complete) | ✅ 2026-11-15 (below) |
| M5 part c: thermal renderers (`thermo`, ushma) | ✅ 2026-11-15 (below) |
| M5 part d: fluid renderers (`fluids`, pravash) | ✅ 2026-11-15 (below) |
| M5 close: the gate on three targets; `em_render` to 2.1.0 (D7) | ✅ 2026-11-15: **M5 complete**, the CPU track done |
| MB1–MB5 (mabda); M6–M10 | ☐ |

**M1.** 13,793 golden rows over nine modules pass:
- 1,149 class-E rows are bit-identical to Rust's formula in f64 rounded once;
- the 31 rows where glibc misrounds give the correctly rounded value: 28 spot-light `cosf` rows and 3 sRGB `powf`
  rows;
- the `optics` rows (1,704, their own harness) show Cyrius prakash 2.15.2 agreeing with Rust prakash 1.2.0 once
  narrowed to f32.

Pending: 9 constants, until their modules arrive; 3 `json` rows bayan's tree can't read yet (ADR 0002 #19).
`material` moved to M6: it owns a GPU texture and bind group. Measurements: `port-parity.md` §2.

**M2 part a.** `src/sprite.cyr` and `src/batch.cyr` (ADR 0002 #20): 638 `sprite` rows and 117 more `json` rows pass,
with 1 accepted correctly rounded batch row and 1 bayan row pending. 38 more Rust tests are ported. 9 bench twins:
the batch loops are 4–9× Rust. Measurements: `port-parity.md` §3.

**M2 part b.** `src/text.cyr`, `src/ui.cyr`, `src/debug_geom.cyr` and the `physics-debug` profile's
`src/debug_collider.cyr` (ADR 0002 #21, #22): 479 `text`, 151 `debug` and 60 `debug_collider` rows pass, 1 through a
correctly rounded row. All 113 M2 items, 69 tests and 16 benches are done.

**M3 part a.** `src/primitives.cyr`, `src/viz.cyr` and `src/terrain.cyr` (ADR 0002 #23): 637 golden rows pass, 36
of them class E (the geometry in f64, rounded once). All 37 Rust tests are ported; the 4 bench twins run 4.3–8.1×
Rust. `create_terrain_mesh` uploads to the GPU, so it moved to M6. Measurements: `port-parity.md` §4.

**M3 part b.** `src/animation.cyr` (ADR 0002 #24): 1,777 golden rows pass, 520 of them class E. Joint chains accumulate
in f64 (the 64-joint leaf: Rust's f32 is 183 ulp off). slerp's `acos` and `sin` are correctly rounded by
`src/crmath.cyr`, vendored from kimiya 2.0.1 by `scripts/vendor_crmath.py` (ledger L1). It costs every binary that
includes soorat 20.2 KB of tables, and 36.6 KB once slerp is reached. CUBICSPLINE plays back as the Hermite spline (#13).
19 of 21 Rust tests are ported (2 need the loader); the 4 twins run 11.3–15.3× Rust. Measurements:
`port-parity.md` §4.4–4.6.

**M3 part c.** `src/gltf.cyr`, the opt-in `gltf` profile on bayan (ADR 0002 #25). It mirrors gltf-json 1.4.1's schema
field by field. 292 golden rows over 146 fixtures:
- 266 identical to Rust 1.2.0;
- 1 correctly rounded (`float_stress`'s decimals);
- 25 named under ADR #14: 11 inputs that panic Rust, and bayan's 6 differences from serde_json, filed as L23.

`validate.py` re-derives every accepted document. All 63 M3 Rust tests and 81 items are ported. Loading `rich.glb` takes
389 µs for both loaders, and the profile adds ~479 KB to a binary. Measurements: `port-parity.md` §4.8.

**M4 part a.** `src/shadow_math.cyr` (ADR 0002 #26): 6,972 golden rows pass, 1,539 of them class E. Rust's f32 matrices
and splits are up to 29 ulp from the f64-once values, and turn infinite or NaN where an intermediate overflows f32.
ganita's `pow` agreed with a correctly rounded one on every split. All 23 `shadow.rs` tests are ported. The twins run
5.1–5.9× Rust; the splits run 16.8×, held back by `pow` at 200 ns. Measurements: `port-parity.md` §5.

**M4 part b.** `src/env_cpu.cyr`, `src/pbr_material.cyr` and the `optics` profile's `src/pbr_optics.cyr` (ADR 0002 #27):
- **Corpus:** 380 golden rows pass.
- **The LUT is prakash 2.15.2's** (adopted after part b): its image-based-lighting remap changes 76 % of the bytes.
  The 49 LUTs that differ from Rust's are class R, against an independent evaluation (`pbr_optics.r.tsv`, read by
  the harness's new `.r.tsv` side table); all match. F0 stays prakash 1.2.0's formula. About 5 % slower.
- **sRGB encoding** uses encode thresholds in place of a `pow` per channel (identical bytes; a 128² mip chain 5.0 →
  1.4 ms).
- **Tests:** all 10 M4 tests of these modules are ported.

Measurements: `port-parity.md` §5.3.

**M4 part c.** `src/render_graph.cyr` (ADR 0002 #28): soorat's own, not over mabda's `rg_*`, which records GPU work
rather than ordering named passes. Rust's search is made iterative, with its scratch in the passes. 435 golden rows
pass, and `validate.py` re-derives each independently. All 10 `render_graph.rs` tests are ported, with four
Cyrius-only ones (a 3,000-pass chain, scratch reuse, copied names, freed memory). Ordering `forward_with_effects`
takes 327 ns. Measurements: `port-parity.md` §5.5.

**M4 close.** The profiler and capabilities moved to M6: Rust soorat re-exports mabda 1.0's types for both, and mabda
becomes a dependency at M6. mabda 4.2.0's profiler lacks pass timings and times an unbegun frame from clock zero,
filed as L24.

**M5 part a.** `src/acoustic.cyr` (in `[lib]`) and the new `acoustics` profile's `src/acoustic_render.cyr` on goonj
2.3.2 (ADR 0002 #29): the acoustic types, heat map, ray paths, portals and coupled decay curves. 637 golden rows pass;
the 556 renderer rows are class E, 410 of them accepted, all bit-identical to the f64 reference. The decay's exp and
log10 are correctly rounded, from kimiya through `src/crmath_ext.cyr` (33 KB, the profile only). Seven byte buffers
reserved 8 × their size (a bare top-level `var g[N]` is N slots); as `u8[N]` they save 38 KB of static data per
binary. 20 of 35 `acoustic_render.rs` tests are ported. Measurements: `port-parity.md` §6.

**M5 part b.** The acoustic meshes: pressure slices, room modes and directivity balloons, through viz's builders.
`acoustic_render` is 752 golden rows, 564 accepted E, all bit-identical. The balloon's gain lookup (an index) follows
Rust's f32 sequence: in f64, f32 π's nonzero sine at the bottom pole made neighbouring pole vertices break an azimuth
tie differently, and the pole failed Rust's own winding test. `src/crmath_ext.cyr` (renamed from part a's
`crmath_exp`) briefly held kimiya's correctly rounded `cos`, which part c moved to `src/crmath.cyr`. All 35
`acoustic_render.rs` tests are ported. Measurements: `port-parity.md` §6.3–6.4.

**M5 part c.** `src/thermo.cyr` (in `[lib]`) and the new `thermo` profile's `src/thermo_render.cyr` on ushma 2.0.0
(ADR 0002 #30): thermal grids, temperature profiles, cycle diagrams, networks and heat-flux arrows. ushma's types are
f64 on both sides, so the oracle feeds any f64. 248 golden rows pass, `thermo_render`'s 236 class E (189 accepted),
with their own replay: ushma brings 60 KB of static data to a test compile. `validate.py` now writes class-E rows as
fresh token lists, since an f32 overflow (a flux of 1e30) can change an output's length. ushma costs a binary
~712 KB. All 24 `thermo_render.rs` tests are ported. Measurements: `port-parity.md` §6.5–6.6.

**M5 part d.** `src/fluid.cyr` (in `[lib]`) and the new `fluids` profile's `src/fluid_render.cyr` on pravash 2.0.1
(ADR 0002 #31): particles as XZ quads or camera-facing billboards, and a shallow-water surface as a height-field
mesh. pravash's types are f64 on both sides. 151 golden rows pass, `fluid_render`'s 143 class E (117 accepted), with
their own replay: pravash brings 81.5 KB of static data, leaving that replay 3.2 KB under the 128 KiB advisory (L25,
filed with pravash). pravash costs a binary ~457 KB. All 15 `fluid_render.rs` tests are ported. Measurements:
`port-parity.md` §6.7–6.8.

**M5's close.** The five parity harnesses and the three renderer test files gave the same outcome for every row on
x86_64 ELF, aarch64 (qemu) and x86_64 PE (wine) (port-parity §6.9). D7 applied: bijli 2.0.0 has not shipped, so
2.0.0 goes without the `em` feature and `em_render` follows in 2.1.0; its 37 tests and 15 items are deferred in the
ledgers. A PE build with `--features thermo` alone re-resolves hisab to ushma's 3.4.1 pin (L11) and rewrites `lib/`,
so the cross-target builds use `--locked`.

**Optional features.**
- `optics` (prakash), `physics-debug` (impetus), `serde` (JSON on bayan) and `gltf` (the glTF loader, on bayan) are
  opt-in profiles: `dist/soorat-optics.cyr`, `dist/soorat-physics-debug.cyr`, `dist/soorat-serde.cyr` and
  `dist/soorat-gltf.cyr`. The core bundle is `dist/soorat.cyr`.
- serde and gltf are opt-in because including bayan costs a binary ~372 KB.
- The repository's tests use them all, so resolve with
  `cyrius deps --locked --features optics,physics-debug,acoustics,thermo,fluids`.

**Step 2's repairs.** `cyrius port` overwrote CLAUDE.md, the roadmap and both workflows, pinned
`version = "0.1.0"`, left `bench` out of `[test] stdlib`, and wrote undocumented stubs, all as cyrius issue
`2026-11-08-cyrius-port-clobbers-project-docs-and-omits-bench.md` says. The repairs:
- CLAUDE.md and the roadmap are restored;
- CI checks both the Cyrius tree and the `rust-old/` oracle;
- `release.yml` is the Cyrius one: it matches only `N.N.N` tags and no longer publishes to crates.io;
- `cyrius.cyml` reads `VERSION` and adds `bench` to `[test] stdlib`;
- the stubs are documented;
- the probes and helper scripts point at `rust-old/`.

## Toolchain

- **Cyrius:** `6.7.18`, the `cyrius.cyml` `[package].cyrius` pin.
- **Rust (the oracle only):** 1.100, MSRV 1.100, in `rust-old/` (`rust-old/rust-toolchain.toml`).

## Source

- **Rust reference:** `rust-old/`. 7,649 code lines in 44 modules, plus 9 WGSL shaders, 15,767 lines in all.
- **Cyrius:** the library is `[lib]` in `cyrius.cyml`, bundled as `dist/soorat.cyr` (`cyrius distlib --all`):
  `src/{error,log,mathx,crmath,tables_data,tables,math,layout,color,lights,lod,sprite,batch,text,ui,debug_geom}.cyr`
  and `src/{viz,primitives,terrain,animation,shadow_math,env_cpu,pbr_material,render_graph,acoustic,thermo,fluid}.cyr`.
  The profiles add `src/color_optics.cyr` and `src/pbr_optics.cyr` (`[lib.optics]`), `src/crmath_ext.cyr` and
  `src/acoustic_render.cyr` (`[lib.acoustics]`), `src/thermo_render.cyr` (`[lib.thermo]`), `src/fluid_render.cyr`
  (`[lib.fluids]`), `src/debug_collider.cyr` (`[lib.physics-debug]`), `src/json.cyr` (`[lib.serde]`), and `src/json.cyr` with `src/gltf.cyr` (`[lib.gltf]`). `src/layout.cyr` and
  `src/tables_data.cyr` are generated (`scripts/oracle/gen_layout.py`, `gen_tables.py`), and `src/crmath.cyr` and
  `src/crmath_ext.cyr` are vendored (`scripts/vendor_crmath.py`). `src/main.cyr` is still the scaffold's smoke program.
- **Port machinery (M0):**
  - `scripts/oracle/`: the golden generator, `validate.py`, `gen_layout.py` and `gen_tables.py`. `tests/golden/` holds
    29 modules, 26,975 rows, and 4,197 side-table rows: 4,114 class E (`*.e.tsv`), 34 correctly rounded (`*.cr.tsv`)
    and 49 adopted (`pbr_optics.r.tsv`), with the glTF fixtures in `tests/golden/gltf/`;
  - `tests/parity.tcyr` with `parity_core.cyr`, `ulp.cyr` and the generated `layout_probe.cyr`: the replay;
  - `scripts/render_ref/`: the class-G corpus (`tests/golden/render/`, 12 scenes, adapter-pinned);
  - `scripts/{surface_parity,rust_test_ledger,bench_record,doc_examples,upstream_ledger}.py`, in CI.

## Tests

- **Rust oracle:** 501 tests pass with all features (460 unit, 12 integration, 9 GPU smoke, 18 oracle-hook,
  2 shader-validation). CI runs them under `rust-old/`.
- **Cyrius:**
  - `cyrius test` and `cyrius test --bounds` pass with zero warnings: 32 test files, among them 26 of ported tests
    (316 Rust tests), the smoke test, and the five parity replays (core, features, acoustics, thermo, fluids). Of the
    26,975 rows, 26,968 pass (4,222 of them accepted: E 4,114, cr 34, R 49, ADR #14 25); 7 are pending (3 constants of
    later milestones, 4 bayan rows);
  - `cyrius audit` exits 0; `cyrius distlib --check` is current; `cyrius vet` is clean;
  - at M5's close, all five parity harnesses and the M5 renderer tests gave the same outcome for every row on x86_64
    ELF, aarch64 (qemu) and x86_64 PE (wine) (port-parity §6.9; M4's close, §5.7).
- **Ledgers:**
  - surface: 912 public items, 499 ported (every M1–M5 item), 25 deferred, `em_render`'s among them (D7)
    (`scripts/surface_parity.py`);
  - Rust tests: 502, 316 ported (every M1–M5 test), 40 deferred, `em_render`'s 37 among them (D7)
    (`docs/development/rust-test-ledger.md`);
  - benches: 34 of 35 Rust benches twinned, plus 36 extras (`docs/benchmarks-rust-v-cyrius.md`).

## Dependencies

Declared in `cyrius.cyml` today: the stdlib (`string`, `fmt`, `alloc`, `vec`, `str`, `syscalls`, `io`, `args`,
`math`, `ganita`, `tagged`, `result`, `freelist`, `sakshi`, `fnptr`; test: `assert`, `bench`, `overflow`, `bayan`),
prakash 2.15.2 (optional, `optics`), impetus 2.0.0 (optional, `physics-debug`), goonj 2.3.2 (optional, `acoustics`)
and ushma 2.0.0 (optional, `thermo`), both with hisab 3.5.0 (goonj declared first, D9), and pravash 2.0.1
(optional, `fluids`). The `serde` and `gltf` bundles need
bayan, which their consumers add to their own stdlib leaves. mabda arrives at M6. Each bundle's sidecar (`dist/*.deps`) names the stdlib leaves it needs. Each milestone adds its own; the full list and its order
are in the plan's §4 step 3.

**Open upstream requests** (ledger, plan §10):
- mabda: the render tier and the render-loop leaks (L2–L6, L17), the profiler's gaps (L24), and tidy-up (L12);
- name clashes: hisab/ranga (L18), agnodrm/sigil (L19), goonj/pravash (L20);
- cyrius: f32 types (L8);
- selah: raw-RGBA encoders (L9);
- agnodrm: display queries (L10);
- ushma: the hisab pin (L11);
- pravash: the static data every including compile carries (L25);
- prakash: allocation-free `Rgb` results (L21);
- ganita: an fmod (L22); bayan: glTF verdicts that differ from serde_json's (L23);
- soorat's numbers added to the open ganita and cyrius issues (L1, L13).

## Consumers

kiran (rendering), salai (editor viewport), joshua (visualization): all still Rust, and they stay on soorat
1.2.0 until 2.0.0.

## Next

**The GPU track.** M6 (the GPU core) needs mabda's MB1 and MB2 (plan §8), done in mabda's repository. mabda is at
4.2.0; the render-tier request (L2–L6) is filed and not yet started. The `ranga` feature cannot be combined with
`acoustics` or `thermo` until L18 lands, and `acoustics` with `fluids` warns on `logging_init` until L20 does.

**Performance items carried forward:** `mul_mat4` at about 120 ns (M4's shadow matrices call it). The correctly
rounded sRGB `pow` (200–285 ns a call) no longer runs per texel: M4 part b encodes faces by threshold search.
port-parity §2.2 has the numbers and the options.
