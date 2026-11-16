# soorat Rust → Cyrius port plan

> **Decided 2026-11-15: the maintainer took every recommendation in §0.2 (D1–D9).**
> **M0 steps 0 and 1 are done (2026-11-15).** 1.2.0 is tagged and released (`6625dc2`). The probes are recorded in [port-parity.md](port-parity.md) §1. They confirmed the language and the math (P1–P3, P7 and P8). They also found three things the plan didn't have:
> - **dependency name clashes** that stop the full bundle compiling, and in one case give a silently wrong sRGB result (P11, ledger L18–L20);
> - a **render-loop leak** in mabda (P8, L17);
> - mabda's **missing compute** (P6, MB4).
>
> **M0 step 2 is done (2026-11-15).** `cyrius port` moved the 1.2.0 crate to `rust-old/`, and the scaffold defects are repaired (`state.md`).
> - The oracle passes all 501 tests there.
> - The Cyrius scaffold builds, tests and passes `cyrius audit`.
> - CI checks both trees.
> - ADRs 0001 and 0002 are written.
>
> **M0 is complete (2026-11-15).** Steps 3–8 built the manifest, the golden generator and corpus (`scripts/oracle/`, 1,473 rows), the parity harness, the class-G render corpus (`scripts/render_ref/`, 12 scenes), the five tooling scripts in CI, and the baselines.
>
> **M1 parts a–c are done (2026-11-15)**: error, log, math, mathx, layout (generated), colour's core, lights and lod. 6,697 golden rows all pass. The 1,149 class-E rows match the f64-once values bit for bit, and on the 28 spot-cone rows where glibc's `cosf` misrounds, the port gives the correctly rounded value (§14; port-parity §2). `material` moved to M6: it is a GPU object.
>
> **M1 part d is done (2026-11-15)**: f16 ⇄ f32 and the sRGB transfer functions, with the decode table generated from the oracle. All 5,215 `tables` rows pass; on the 3 sRGB rows where glibc's `powf` misrounds, the port gives the correctly rounded value. A correctly rounded `pow` costs about 210–280 ns: a performance item for M4's face encoding (port-parity §2.2).
>
> **M1 is complete (2026-11-15).** Part b2 added colour's `optics` half and serde, both opt-in:
> - **optics:** prakash is an optional dependency behind an `optics` feature, bundled as `dist/soorat-optics.cyr` (ADR 0002 #18). All 1,704 rows agree with Rust, so Cyrius prakash 2.15.2 and Rust prakash 1.2.0 agree once narrowed to f32.
> - **serde:** JSON is the `[lib.serde]` profile on bayan (ADR 0002 #19). It is opt-in because including bayan costs a binary ~372 KB; three bayan-policy rows stay pending.
>
> **M2 is complete (2026-11-15).** Sprites, batches, text, UI and debug geometry pass all 1,445 new golden rows bit for bit (1,328 in four new modules, 117 in `json`; 2 through correctly rounded `sinf`/`cosf` rows). The impetus collider is a third optional profile (`physics-debug`). The 2D loops run 4–11× behind Rust; the `sin`/`cos` cache and the inlined line writes are in port-parity §3.2.
>
> Next: **M3 (geometry and animation)**: primitives, terrain, viz, animation (CUBICSPLINE per ADR 0002 #13), and the GLB loader (D6). The `ranga` feature cannot be combined with `acoustics` or `thermo` until L18 lands.
>
> **Step 0 detail (2026-11-15, committed as `7f3213a`, tagged as 1.2.0).** 1.1.0 was tagged and released on 2026-11-15 (`0a94507`, GitHub release 14:26 UTC) before the oracle work started. The hooks and D2's fixes therefore ship together as **1.2.0**, the final Rust release and the oracle. It is a minor release, not 1.2.0, because it adds public API (`set_bloom`, `clear_bloom`, `with_bloom_intensity` and the `oracle` feature). The maintainer tags 1.2.0.
> - Part 1, the hooks: done, every gate green.
> - Part 2, the fixes: particle damping and the bloom composite are done, each with a GPU regression test. The bathymetry item was not a defect (§6.3). CUBICSPLINE moved to the Cyrius port (ADR 0002 #13, decided 2026-11-15), because the fix needs tangent storage in public types.

> **Plan of record** for porting soorat to Cyrius.
>
> - **Versions.** Rust **1.2.0** is the final Rust release and the oracle. It is 1.1.0 (the 2026-11 audit, released 2026-11-15) plus D1's hidden `oracle` feature and D2's fixes. The Cyrius port ships as **2.0.0**.
> - **Method.** tanmatra's ([ADR 0006](../../../tanmatra/docs/adr/0006-cyrius-port-with-rust-oracle.md)), as adapted by impetus, kimiya, pravash, bijli and selah.
> - **Models.**
>   - **Process: pravash** ([its plan](../../../pravash/docs/development/pravash-port-plan.md)). It is the most recent completed port and close in size: 6.9k code lines against soorat's 7.6k.
>   - **Domain: ranga** ([port mechanics](../../../ranga/docs/development/port-mechanics.md)). It does f32 pixel math and is a GPU consumer of mabda.
>   - **Images: selah**. It defines the ground-truth image classes.
>   - "As pravash", "as ranga" and so on mean: follow that document.
>
> ▶ **The critical path is mabda, not soorat.**
> - mabda 4.2.0 has no general render pipeline: no vertex or index buffers, depth, blending, uniforms in a render pass, `Rgba16Float`, or window surface (§2.1).
> - soorat reaches wgpu **only through mabda** (the maintainer, 2026-11-15), so those gaps can't be bridged with a local stand-in.
> - The plan therefore has three tracks:
>   - **CPU track (M1–M5):** about 60 % of the code. It needs nothing new from mabda.
>   - **mabda render tier (MB1–MB5):** done in mabda's repository (§8).
>   - **GPU track (M6–M9):** each milestone starts when its MB milestone ships.

| Thing | Value | Source |
|---|---|---|
| Rust source | **soorat 1.2.0**, the oracle (`rust-old/`, tag `1.2.0` at `6625dc2`). The code-line counts are 1.1.0's; 1.2.0 has **280 public `pub fn`s** (plus 4 in `egui_bridge`) and 912 public items in all (`scripts/surface_parity.py`). 7,649 code lines in `src/` (no tests, comments or blank lines) across 44 modules, 267 `pub fn`s and 15 `#[repr(C)]` GPU-layout types, plus 9 WGSL shaders (1,009 lines) | `VERSION`; count of `src/` |
| Tests (1.2.0) | **502**: 460 unit, 12 integration, 9 GPU smoke (they skip without an adapter), 18 oracle-hook, 2 shader-validation (naga), 1 ignored doc test. The ledger (`docs/development/rust-test-ledger.md`, `scripts/rust_test_ledger.py`): **471 to port**, 28 n/a (the oracle hooks, `egui_bridge`, naga, error text, `Send + Sync`, `GpuContext`), 3 deferred (drm, L10) | `rust-old/src/`, `rust-old/tests/` |
| Benchmarks | 35 criterion benches. History in `bench-history.csv`: a baseline and two post-audit runs (2026-11-15) | `benches/benchmarks.rs` |
| Features | `optics` (prakash), `ranga`, `physics-debug` (impetus), `fluids` (pravash), `acoustics` (goonj + hisab), `em` (bijli), `thermo` (ushma), `screenshot` (selah, uuid, chrono), `drm` (agnosys, git), `full` | `Cargo.toml` |
| Rust health, 2026-11-15 | The P(-1) audit closed with every gate clean: fmt; clippy on all features and on each feature alone; rustdoc; deny; audit; vet. 479 tests pass with all features, and the GPU smoke tests ran on the local AMD GPU | `CHANGELOG.md` 1.1.0 |
| Rust GPU and IO stack | wgpu 29.0.4 directly, plus mabda 1.0.0 (the final Rust mabda); winit 0.30.13; image 0.25.10 (PNG, JPEG, BMP); gltf 1.4.1 | `Cargo.toml` |
| Rust math | **f32 throughout**, because everything is GPU-facing. std f32 methods call host glibc: `sin` 19 calls, `cos` 16, `sqrt` 15, `round` 8 (all u8 quantization), `sin_cos` 4, `powf` 3, `exp` 2, `acos` 2, `ln` 1, `log10` 1. No glam. hisab is used only for goonj's `Vec3` | grep of `src/` |
| Cyrius toolchain | **6.7.18** | `~/.cyrius/versions` |
| mabda | **4.2.0** (Cyrius), with three backends: wgpu-native, native AMD (GFX9) and native NVIDIA (SM75). It pins **cyrius 6.7.5** | [mabda](../../../mabda/README.md) |
| Science dependencies, already Cyrius | prakash 2.15.2, ranga 2.4.0, impetus 2.0.0, pravash 2.0.1, goonj 2.3.2, ushma 2.0.0, selah 1.0.1 and agnodrm 1.8.1, all pinning cyrius 6.7.18. hisab: **3.4.1** in prakash, ushma, pravash and impetus, but **3.5.0** in goonj | each `cyrius.cyml` |
| bijli | Rust 1.2.0. The Cyrius 2.0.0 port is **in progress**: M0–M3 are done and M4 (RF) part 1 is done (2026-11-15); M5–M8 are open. The visualization types soorat consumes are M7 | [bijli state](../../../bijli/docs/development/state.md) |
| Asset libraries, Cyrius | chitra 1.4.1 (decodes PNG, JPEG, BMP and GIF to RGBA8); bayan 1.11.1 (JSON tree); sankoch (deflate, zlib, crc32; stdlib); kashi 1.3.0 (PSF/BDF/PCF bitmap fonts). **No glTF loader and no windowing library** | each repository |
| Dev hardware | PCI `0x1638`: Cezanne, which RADV reports as RENOIR (gfx90c). This is mabda's HW-verified native-AMD chip | `/sys/class/drm/card1/device/device`; [mabda benchmarks](../../../mabda/docs/benchmarks-rust-v-cyrius.md) |
| Consumers | kiran (its `rendering` feature), salai and joshua. All Rust; none is checked out locally | [agnosticos kiran.md](../../../agnosticos/docs/applications/libs/kiran.md) |
| Waiting on this port | tanmatra and kimiya defer their soorat integration modules "until soorat ports". jantu (Rust) produces soorat-shaped data. mabda's **v5.1 wgpu-retirement gate** requires soorat to run native | [tanmatra plan](../../../tanmatra/docs/development/tanmatra-port-plan.md); [kimiya rust-parity](../../../kimiya/docs/development/rust-parity.md); [mabda roadmap](../../../mabda/docs/development/roadmap.md) §v5.1 |
| Target | **soorat 2.0.0**, Cyrius, carrying everything 1.2.0 does except the not-carried list (§3.2) | this document |

---

## 0. Decisions

### 0.1 Carried from the sibling ports

- **No deferral without a language check.** A Rust feature leaves the 2.0 scope only for a written reason against §1.1: a capability Cyrius lacks, a measured cost, or nothing to test against.
- **The Rust crate is frozen.**
  - 1.2.0 moves to `rust-old/` and is never edited.
  - A generator crate in `scripts/golden/` drives it by path.
  - If a value 1.2.0 records is wrong, it is fixed in Rust as 1.2.x and the corpus is regenerated (as tanmatra).
- **No Rust 2.0.** API-shape fixes land in the Cyrius API only, through ADR 0002 ([kimiya's rule](../../../kimiya/handoff/memory/api-fixes-in-cyrius-port-only.md)).
- **Upstream requests never block CPU work** (as pravash §0.1).
  - Every gap in a Cyrius library or the stdlib is filed with soorat's numbers.
  - soorat ships a local stand-in, marked `UPSTREAM(Ln)` and tracked in the ledger (§10).
  - **Exception: the GPU tier.** soorat reaches wgpu only through mabda, so a stand-in for a missing mabda feature would be a wgpu FFI call inside soorat, which is ruled out.
  - The mabda work is therefore **scheduled in this plan** as milestones MB1–MB5, done in mabda's repository (§8). It is not filed and waited on.
- **Performance is measured, not gated.** Every Rust benchmark gets a Cyrius twin, and both columns are recorded in `docs/benchmarks-rust-v-cyrius.md`.
- **Accuracy beats glibc, within D4's scope.** Correctly rounded primitives are used wherever soorat computes in f64. Quantized outputs keep Rust's f32 sequence (D4).
- **Tables are generated, never retyped.** tanmatra's audit found 17 of 32 hand-transcribed AME rows wrong. The sRGB table, f16 edge rows and BRDF LUT rows come from the oracle.

### 0.2 Decided 2026-11-15 (the maintainer took every recommendation)

| # | Question | Decision | Not chosen |
|---|---|---|---|
| ✅ D1 | **Freezing the oracle.** At decision time 1.1.0 was believed uncommitted; it had already been tagged and released, so the hooks ship in 1.2.0 with D2's fixes. The golden generator needs private helpers: `viz`, `math_util`, animation's `slerp` and `bracket`, environment's f16/sRGB/downsample, and shadow's split and basis code | Add a hidden `oracle` feature that re-exports those helpers read-only, plus a test proving they are read-only (as pravash D1). They ship in 1.2.0, the oracle, which the maintainer tags | — |
| ✅ D2 | **Known value defects in 1.1.0's backlog.** CUBICSPLINE keeps values but drops tangents; particle damping is per step, not scaled by dt; shallow water ignores bathymetry; the bloom composite is unreachable, so bloom output is never used | **Fix all four in Rust 1.2.0 before the freeze** (each is effort S). Otherwise the corpus and the reference renders freeze wrong values that the port must reproduce, then fix again. The API-shaped items (per-draw model uniforms, depth load/clear) become Cyrius-only ADR 0002 rows. The feature-shaped items (GGX-prefiltered mips, cascade selection, node-centred heatmaps) wait until after 2.0 | Port 1.1.0 as it is and fix in 2.x |
| ✅ D3 | **GPU route for 2.0** | **WGSL on mabda's wgpu-native backend.** The nine shaders carry over verbatim. Native AMD and NVIDIA follow once mabda has a render-capable shader path (§13). Two consequences: wgpu-on-AMD is deprecated in mabda 4.0.1 (a one-shot warning; `-D MABDA_AMD_WGPU_STRICT` must stay off), and mabda's v5.1 retirement gate waits on soorat going native | Hand-encoded GFX9 and SM75 shaders for 2.0: 9 × 2 programs, no compiler, and every GPU milestone blocked. SPIR-V on native AMD: the lowerer parses only `GLCompute` and caps a shader at 256 ids |
| ✅ D4 | **Precision policy.** soorat computes in f32 because the GPU does; the Cyrius science crates now hand over f64 | **Three rules.** (a) Quantized or bit-defined outputs (u8 colours, sRGB tables, f16 packing, indices, counts, glyph lookups, UV rects) follow **Rust's f32 sequence exactly**: class A. (b) Geometry and matrices bound for the GPU are **computed in f64 and rounded once to f32** at packing: an ADR 0002 divergence, checked against Rust and against an f64 reference (§5). (c) Science inputs stay f64 until vertex packing | Bit-faithful f32 everywhere ("compute in Rust's precision, not more", ranga): the cheapest parity, but it gives up §6's gains. f64 everywhere: it flips quantized outputs (ranga: f32 `0x3f010101` gave 129 in Rust, 128 in f64) |
| ✅ D5 | **Who owns the window.** mabda's wgpu path already makes the application own the entry point: a C launcher calls `mabda_main(fn_table, preinit)` | **soorat 2.0 owns no window or event loop.** The application passes a mabda `GpuContext` and a surface. A shared Wayland window library is its own project: the `win_*` client exists in four copies, in puka, cyrius-doom, jalwa and thoth | A Wayland client inside soorat, which would be the fifth copy |
| ✅ D6 | **glTF loading** | A soorat-owned `src/gltf.cyr`, built on bayan's JSON tree, `load8/16/32` and chitra for embedded images. It carries exactly 1.1.0's subset and validation: GLB plus embedded buffers, triangles only, index range checks, checked buffer views, skins, LINEAR/STEP/CUBICSPLINE. P9 (port-parity §1.6) widened it: 1.2.0 also loads **JSON-text `.gltf`**, and the `gltf` crate reads **sparse accessors**; bayan carries everything soorat reads (42 of 51 fixtures identical, and the rest are ADR 0002 #14) | A separate glTF package first. Only soorat needs one today; wait for the third instance |
| ✅ D7 | **em_render and bijli** | Port `em_render` in M5 **if bijli 2.0.0 (its M7) has shipped**. Otherwise 2.0.0 ships without the `em` feature and `em_render` follows in 2.1.0. The EM axis-convention fix (roadmap backlog) goes in with it as an ADR 0002 row | Hold 2.0.0 for bijli, or port against bijli's pre-M7 API |
| ✅ D8 | **Reshaped modules** | `egui_bridge`: **not carried** (Cyrius has no egui; dhancha is retained-mode). `window`: reduced per D5. `drm`: reduced to what agnodrm 1.8.1 offers (device list, driver version, capabilities) until agnodrm adds connector, mode and render-node queries (L10). `screenshot` encoding: a selah request for a public raw-RGBA PNG/JPEG/BMP encoder (L9), with selah's private encoders vendored as the stand-in | Writing new encoders in soorat |
| ✅ D9 | **Shared dependency pins** | M0 probe P4 builds the full bundle on **one hisab** and on cyrius 6.7.18. Alignment requests are filed: the hisab 3.4.1 consumers move to 3.5.x (L11), and mabda's pin moves to 6.7.18 (L12). **soorat itself declares no hisab dependency**, because goonj's `soorat_*` accessors return plain values. *P4 note:* with no root pin, the hisab version follows `[deps]` order, so §4 step 3 must declare goonj before ushma, which resolves 3.5.0. hisab 3.4.1 lacks `hisab_alloc_scope`, which goonj needs. If order-dependence proves fragile, the alternative is pinning 3.5.0 at the root, an amendment to this row | soorat pinning its own hisab |

---

## 1. Porting models, references and the language

**Models.** Copy these and adapt them:
- **pravash 2.0.1** is the template:
  - `scripts/golden/` (the generator crate, glibc header and `validate.py`);
  - `tests/parity_core.cyr` and `tests/ulp.cyr`;
  - `scripts/surface_parity.py`, `rust_test_ledger.py`, `bench_record.py`, `doc_examples.py` and `upstream_ledger.py`;
  - `src/mathx.cyr` (Rust-semantics helpers) and its stable merge sort (pravash ledger L9);
  - the CLAUDE.md translation of the Rust principles.
- **ranga 2.4.0** covers f32 practice:
  - get f32 literal bits from rustc, because `f32_from(decimal)` rounds twice;
  - ADR 002, f32 transcendentals at ≤ 2 ulp: 1,419 and 1,830 of 140,608 values differ;
  - mutation testing per module;
  - identity assertions (white → L = 1), which caught bugs that round trips missed.
- **selah 1.0.1** covers image parity: [ADR 0003 "accuracy is the bar"](../../../selah/docs/adr/0003-accuracy-is-the-bar.md), with its B, R and W classes and its known-upstream-defect list.
- **kimiya** covers correctly rounded rows (`*.cr.tsv` from MPFR) and ADR 0002's "noted rows mapped" accounting.
- **impetus 2.0.0** covers correctly rounded `src/crmath.cyr` (ADR 0011).
- **mabda 4.2.0:**
  - [native-migration.md](../../../mabda/docs/guides/native-migration.md);
  - the `programs/*_e2e.cyr` patterns, `wgpu_cube_sample_e2e.cyr` in particular;
  - ADR 004, the C launcher.
- **goonj 2.3.2 and ushma 2.0.0** already ship `src/soorat.cyr`, the producer side of soorat's acoustic and thermal inputs.

**Language and toolchain** (`/home/macro/Repos/cyrius`): `docs/guides/cyrius-guide.md`, `docs/stdlib-reference.md`, `docs/ffi/fncall-abi.md` and `docs/ffi/struct-packing.md`. The f32/u32 field sizes in `struct-packing.md` are stale; the guide's 6.7.10 rule (4 bytes) holds.

**Open upstream issues this port inherits:**
- `cyrius port` clobbers project docs and omits `bench` (`cyrius/docs/development/issues/2026-11-08-…`);
- there is no scoped allocator (S1);
- `memcpy` is a byte loop;
- ganita's correctly rounded tier is still unshipped.

### 1.1 Language review: Cyrius 6.7.18 against soorat's Rust surface

| Rust in soorat | Cyrius 6.7.18 | Where | Verdict |
|---|---|---|---|
| f32 arithmetic and std f32 methods | Native `f32` (`addss` etc.). Operators are **typed by the left operand**: an f32 left operand reads the right operand's low 32 bits, so `x * 2.0` is 0. `fn f(): f32` is refused, so a function returns the bits in an i64. A `const` can't be f32, so use `var`. There are no exponent literals. ganita's f32 tier widens sqrt and the transcendentals to f64, then narrows. There is **no f32 asin/acos** | guide *Types*; `lib/ganita.cyr` 4618–5060 | port. Every mixed expression goes through `f32_from`. `acos` goes through f64 (D4) |
| f64 (D4's rule b) | Builtins within 1 ulp. x87 `exp`/`ln`/`atan` on x86 differ across targets, so use `_f64_*_polyfill` for identical bits. ganita has pow, tan, acos, atan2, hypot and cbrt | guide *Math builtins*; stdlib-reference 1108–1160 | port, with correctly rounded stand-ins (impetus crmath) on paths parity checks |
| `.round()` (8 sites, all `* 255.0` → u8) | `f64_round` rounds **ties to even**; Rust rounds half away from zero | probe | a Rust-semantics `sr_round` in `mathx` (as impetus) |
| `#[repr(C)]` + bytemuck `Pod` (15 types, plus mabda's vertex types) | Packed structs with no padding; f32/u32 fields are 4 bytes since 6.7.10. **No array fields** and no vector-type fields. Structs over 16 bytes pass through a hidden pointer | guide 1676–1736, 1944; a probe built a 96-byte nested struct | nested `SrV4`/`SrM4` structs with explicit `_pad` fields. Every layout is asserted against Rust's `size_of`/`offset_of` rows (class A, probe P1) |
| `bytemuck::cast_slice(&[Vertex])` | raw `alloc` buffers written with `store32` | — | vertex streams as byte buffers with a stride |
| `Result<T, RenderError>`, thiserror | Value-form `Result` (a tag and payload register pair) with `?`. Pitfalls: an 8-byte struct payload read back is treated as an address and segfaults (filed); 16- and 32-byte structs can't ride a `Result`; a `Result` through `callptr` loses its payload | guide *`?` Propagation*; pravash §1.1 | `SR_ERR_*` codes. Matrices (64 B) and colours (16 B) return through out-pointers |
| enums with data | `enum X: stack` takes one field per variant; others box onto the heap | guide *Sum types* | a kind plus a payload struct, as impetus's `ImColliderShape` |
| closures (`xz_heatmap_mesh(grid, \|i\| color)`, `heightfield_mesh`) | Capture by value; at most 5 parameters (3 on Windows); untyped parameters; a **one-word result** | guide *Closures*; fncall-abi *Results* | a function pointer plus a context; the colour is written through an out-pointer |
| generics, traits | monomorphized generics; static traits; `dyn` | ADR 007 | port (soorat uses few) |
| `Vec<T>`, `Vec<f32>` | `vec` holds 8-byte words, never frees an outgrown buffer, and aborts on a bad index. f32 array elements take 4 bytes | stdlib *vec* | caller-sized buffers and a frame arena (§9.4) |
| `HashMap` (`TextureCache` u64 keys; `RenderGraph` name → index) | `hashmap.cyr` with string and u64 keys; the seed is per process, so **iteration order varies** | stdlib *hashmap* | port. A test proves no output depends on iteration order |
| `sort_by_key` (stable: `SpriteBatch::sort_by_z`, test `batch_sort_stability`); `partition_point` | `vec_sort_by` is an unstable introsort. No stable sort and no binary search in `lib/` | `lib/vec.cyr` | pravash's stable merge sort (L9) and a local `sr_partition_point` |
| checked and saturating integer arithmetic | `+?` **aborts** with exit 57, which is ruled out in a library. `+\|` saturates only at i64 | `lib/overflow.cyr` | explicit compare-then-add helpers in `mathx` |
| serde on soorat-owned config types | `#derive(Serialize/Deserialize)` through bayan | guide *Derive* | port the derives where 1.1.0 has them. Science inputs now arrive as Cyrius structs |
| `tracing` | sakshi | `lib/sakshi.cyr` | port every span and warning |
| `image` decode | chitra 1.4.1 (PNG, JPEG, BMP, GIF) | chitra public surface | port. JPEG is not bit-identical to `image` (§6) |
| `image` encode | selah 1.0.1 has no public raw-RGBA encoder | `selah/src/png_enc.cyr` is private | D8, L9 |
| `gltf` | none | — | D6 |
| `winit` | none | — | D5 |
| wgpu 29 + mabda 1.0 | mabda 4.2.0's `gpu_*` tier | §2.1 | MB1–MB5 |
| `pollster` | — (mabda calls are synchronous) | — | drop |
| criterion | `lib/bench.cyr`, `.bcyr` | — | port all 35 |
| naga validation (`tests/shaders.rs`) | no offline WGSL validator; wgpu-native validates at pipeline creation | mabda proposal `v3-wgsl-frontend-choice.md` | keep naga in the oracle crate while `rust-old/` exists; a GPU smoke test creates every pipeline |
| GPU tests that skip without an adapter | the test runner has no skip status | `cbt/` | exit 0 with a printed notice (as Rust does) |
| f16 conversion (`environment`) | no f16 helpers | — | port the bit operations (class A over every edge) |
| Cargo features | `[features]` switches optional dependencies; `#ifdef` takes at most 16 names; dead-code elimination | guide *Manifest*, *Preprocessor* | one bundle; each science crate an optional dependency |
| `#[inline]`, `#[must_use]`, `#[non_exhaustive]` | `#inline` refuses more than 2 parameters, more than 32 tokens or a pointer parameter. `#must_use` exists. Enums are append-only by convention | guide *Attributes* | translate. Hand-inline hot paths only where a bench shows it |

---

## 2. What is different from the sibling ports

### 2.1 The GPU half has an upstream gate

No Cyrius program draws real geometry through mabda today. puka's `gpu.cyr` draws mabda's hand-encoded solid triangle through `@internal` helpers, and `programs/render_e2e.cyr` uses raw wgpu FFI. What soorat needs against what mabda 4.2.0's public API offers:

| soorat needs | mabda 4.2.0 | Status | MB |
|---|---|---|---|
| A render pipeline with vertex layouts (2D, 3D, skinned, line), depth state with bias, culling, blending, TriangleList and LineList, explicit bind-group layouts | `gpu_render_pipeline_create(ctx, vs, fs, fmt)` is fixed: `vs_main`/`fs_main`, no depth, no blending, no vertex buffers, automatic layout (`src/backend_wgpu.cyr` 896–924). `rpb_depth`, `rpb_cull_mode` and `rpb_vertex_layout` exist, but **`rpb_build` ignores them** (`src/render_pipeline.cyr`: "only the minimal-path fields take effect") | missing | MB1 |
| Draw-time binding: vertex and index buffers (u16 and u32), indexed draws, uniform/storage bind groups, dynamic offsets, viewport and scissor | Buffers can be created but not bound. `draw_cmd_draw_indexed` only records a command. The C launcher's function table has no `SetVertexBuffer`, `SetIndexBuffer`, `DrawIndexed`, `SetViewport` or `SetScissorRect` | missing | MB1 |
| Depth32Float targets, depth-texture binding, a **comparison sampler** (LessEqual) | `depth_texture_new` and `bglb_comparison_sampler` are wgpu-tier descriptors only. `gpu_sampler_create` offers point/bilinear × clamp/wrap. `bglb_texture_2d` hard-codes a float sample type | effectively missing | MB2 |
| `Rgba16Float` HDR targets; MSAA; several colour targets | Absent from `MabdaTexFmt` (`src/texture_format.cyr`); `gpu_*` render targets are RGBA8 only | missing | MB3 |
| Cube textures with a mip chain; writing each mip; **rendering into a cube face or mip level** | `gpu_texture_create_cube`, `gpu_texture_write_layer_level` and `gpu_render_pass_bind_texture_layer` work on wgpu. `gpu_texture_generate_mipmaps` returns NOT_IMPLEMENTED on wgpu. Rendering into a face or level is missing | partial | MB3 |
| Texture-to-texture copy (3 sites); render-target readback (screenshots, tests) | `gpu_texture_read` works for RGBA8 textures. No copy, no render-target readback | partial | MB3 |
| Compute on the `gpu_*` tier (`gpu_particles`) | On wgpu, `_backend_wgpu_compute_dispatch` **returns `GPU_ERR_OTHER` unconditionally**. The older `compute_pipeline_new` works but needs the `@internal` FFI for its bind group | partial | MB4 |
| A window surface: a `WGPUSurface` from a Wayland handle, BGRA format, present modes | `gpu_surface_configure_wgpu` needs a `WGPUSurface`, but nothing can create one, and the format is forced to RGBA8_UNORM. The native path is full-screen KMS only | missing | MB5 |
| Already there and reusable | `rg_*` render graph, `profiler_*`, `gpu_timestamps_*`, the shader, pipeline and bind-group caches, `gpu_caps_*`, the `rtb_*` render-target builder | ✅ | — |

The reusable row means soorat's `render_graph`, `profiler`, `capabilities`, `compute`, `render_target` and part of `texture` (about 1,050 lines in Rust, tests included) become thin layers over mabda.

Adding function-table slots changes `deps/wgpu_main.c`, and **every mabda consumer must rebuild** (mabda roadmap backlog, *Encoder debug-group FFI*). MB1–MB5 should land as **one launcher change** where possible, so consumers rebuild once.

### 2.2 Image parity needs a GPU

pravash's corpus is CPU arithmetic. Half of soorat's output is pixels. Class **G** (§5) compares a Cyrius readback against a Rust 1.2.0 readback of the same scene on the **same adapter and driver**:
- the dev box's Cezanne, RADV;
- the same wgpu-native 29 underneath both: Rust's wgpu 29 and mabda's wgpu-native 29.

The corpus header records the adapter, the driver and the Mesa version, and `--check` refuses a corpus recorded on other hardware, as kimiya does for glibc.

### 2.3 f32 is the domain precision

The sibling science crates are f64. soorat's work ends in f32 vertex and uniform data. D4 sets which values keep Rust's f32 bits and which gain accuracy.

### 2.4 The application owns the entry point and the window

D5. soorat's public API takes a mabda context. This is the biggest consumer-facing change, so the migration guide (M10) leads with it.

### 2.5 The inputs changed shape upstream

The six science crates ported first, each with its own ADR 0002. soorat 2.0 binds the Cyrius shapes, not the Rust ones:

| Feature | soorat module | Cyrius producer | Shape change since Rust |
|---|---|---|---|
| `acoustics` | `acoustic_render` | goonj 2.3.2 `src/soorat.cyr` (`soorat_*` accessors; `ray_path_*`, `directivity_balloon_*`, `CoupledDecay_*`) | f32 → **f64**; `gains[]` → `gain_band(b, band)` |
| `thermo` | `thermo_render` | ushma 2.0.0 `src/soorat.cyr` (`UsGridVisualization`, `UsHeatFluxField`, `UsCycleDiagramData`, …) | renamed; arrays flattened; still f64 |
| `fluids` | `fluid_render` | pravash 2.0.1 `PvFluidParticle`, `PvShallowWater` | Clone became an explicit deep copy |
| `physics-debug` | `debug_draw` | impetus 2.0.0 `ImColliderShape`, a flat tagged struct | the enum became kind + fields |
| `optics` | `color`, `pbr_material` | prakash 2.15.2 `Rgb` and `pbr_integrate_brdf_lut(…, out)` | prefixes added; out-pointers |
| `em` | `em_render` | bijli 2.0 **M7, not yet written** | D7 |

**Naming.** goonj already defines `soorat_*` functions, so soorat **must not use `soorat_` as its own prefix**. Use `Sr` for types, `sr_` for functions and `SR_ERR_*` for error codes. `sr_` also appears in szal and `SR_ERROR` in nous; neither is linked with soorat, and probe P11 checks a full bundle.

### 2.6 Shaders carry over verbatim

The nine WGSL files are data. They move to `shaders/` and are embedded (`[embed]`). Only the uniform-layout structs that feed them are ported, and probe P1 proves those byte for byte.

---

## 3. Scope

### 3.1 In scope for 2.0.0

Code lines exclude tests, comments and blank lines. "Track" is C (CPU, needs nothing new from mabda) or G (GPU, gated by an MB milestone).

| M | Rust module | Code | `pub fn` | Tests | Cyrius module(s) | Uses | Track |
|---|---|---|---|---|---|---|---|
| M1 | `error.rs` | 40 | — | 9 | `src/error.cyr` (`SR_ERR_*`) | — | C |
| M1 | *(Rust semantics, f16, sRGB, ulp)* | — | — | — | `src/mathx.cyr`, `src/tables.cyr` with `src/tables_data.cyr` (generated). `src/crmath.cyr` (the `log10` stand-in, L1) arrives with its only caller, `acoustic_render` (M5) | ganita | C |
| M1 | `math_util.rs` | 153 | 8 | 13 | `src/math.cyr` (Mat4, look_at, perspective, inverse-transpose, basis) | — | C |
| M1 | `color.rs` | 207 | 14 | 23 | `src/color.cyr`; `src/color_optics.cyr` (the `optics` profile); `src/json.cyr` (the `serde` profile) | prakash (`optics`), bayan (`serde`) | C |
| M1 | `vertex.rs`, `instancing.rs`, the 15 `#[repr(C)]` types | 2 + layouts | — | 14 | `src/layout.cyr` | mabda vertex types | C |
| M1 | `lights.rs`, `lod.rs` | ≈ 180 | 11 | 15 | `src/lights.cyr`, `src/lod.cyr` | — | C |
| M2 | `sprite.rs`, `batch.rs` | 285 | 21 | 15 | `src/sprite.cyr`, `src/batch.cyr` | — | C |
| M2 | `text.rs`, `ui.rs` | 245 | 21 | 16 | `src/text.cyr`, `src/ui.cyr` | kashi (optional, font import) | C |
| M2 | `debug_draw.rs` (geometry half) | ≈ 200 | 12 | 15 | `src/debug_geom.cyr` | impetus (`physics-debug`) | C |
| M3 | `primitives.rs`, `terrain.rs`, `viz.rs` | 549 | 7 | 37 | `src/primitives.cyr`, `src/terrain.cyr`, `src/viz.cyr` | — | C |
| M3 | `animation.rs` | 398 | 4 | 21 | `src/animation.cyr` | — | C |
| M3 | `gltf_loader.rs` (+ animation loading) | 133 | 2 | 5 | `src/gltf.cyr` (D6) | bayan, chitra | C |
| M4 | `shadow.rs` (matrices, splits, atlas; every shadow test is CPU-only) | ≈ 250 | 6 | 23 | `src/shadow_math.cyr` | — | C |
| M4 | `environment.rs` (CPU: f16, sRGB, downsample, mips; crate-private, reached through the oracle hooks) | ≈ 200 | 0 | 6 | `src/env_cpu.cyr` | — | C |
| M4 | `pbr_material.rs` (BRDF LUT) | 93 | 5 | 8 | `src/pbr_material.cyr` | prakash | C |
| M4 | `render_graph.rs`, `profiler.rs`, `capabilities.rs` | 223 | 11 | 22 | `src/render_graph.cyr` and the others over mabda `rg_*`, `profiler_*`, `gpu_caps_*` | mabda | C |
| M5 | `acoustic_render.rs` | 414 | 7 | 35 | `src/acoustic_render.cyr` | goonj | C |
| M5 | `thermo_render.rs` | 343 | 6 | 24 | `src/thermo_render.cyr` | ushma | C |
| M5 | `fluid_render.rs` | 216 | 4 | 15 | `src/fluid_render.cyr` | pravash | C |
| M5 | `em_render.rs` | 339 | 7 | 37 | `src/em_render.cyr` (D7) | bijli 2.0 | C |
| M6 | `texture.rs`, `render_target.rs`, `compute.rs` | 301 | 19 | 18 | `src/texture.cyr`, … | mabda MB1, MB3 | G |
| M6 | `material.rs` (a GPU texture and bind group; moved from M1 at M1 part c) | ≈ 30 | 2 | 1 | `src/material.cyr` | mabda MB1 | G |
| M6 | `pipeline.rs` (sprites) | 395 | 10 | 18 | `src/sprite_pipeline.cyr` | MB1 | G |
| M6 | `debug_draw.rs` (line pipeline) | ≈ 150 | 5 | 1 | `src/line_pipeline.cyr` | MB1, MB2 | G |
| M6 | `mesh_pipeline.rs` | 398 | 15 | 15 | `src/mesh_pipeline.cyr` | MB1, MB2 | G |
| M6 | `gpu.rs` (re-exports mabda's `GpuContext`) | 1 | — | 1 | — (ADR 0002 #2, #3) | mabda | G |
| M7 | `shadow.rs` (passes, maps, atlas textures) | ≈ 190 | 12 | 0 | `src/shadow.cyr` | MB2 | G |
| M7 | `environment.rs` (GPU), PBR and skinned paths | ≈ 205 | 5 | 2 | `src/environment.cyr` | MB3 | G |
| M8 | `hdr.rs`, `postprocess.rs`, `ssao.rs` (1.2.0 added `set_bloom`, `clear_bloom`, `with_bloom_intensity`) | 493 | 20 | 16 | `src/hdr.cyr`, `src/postprocess.cyr`, `src/ssao.cyr` | MB3 | G |
| M8 | `gpu_particles.rs` | 255 | 7 | 5 | `src/gpu_particles.cyr` | MB4 | G |
| M8 | `screenshot.rs` | 210 | 10 | 10 | `src/screenshot.cyr` | MB3, selah (D8) | G |
| M9 | `window.rs` (surface glue only, D5; 3 fns not carried) | ≈ 120 | 8 | 8 | `src/surface.cyr` | MB5 | G |
| M9 | `drm.rs` | 141 | 5 | 9 | `src/drm.cyr` (D8) | agnodrm | C |

The `shadow`, `environment`, `debug_draw` and `window` splits were measured at M0 step 7 by `scripts/surface_parity.py` and `scripts/rust_test_ledger.py`, which fix them item by item (their `SPLITS` and `TEST_MILESTONE` tables).

**Track C:** about 4,640 code lines (61 %). **Track G:** about 2,720. **Not carried:** about 150 (`egui_bridge` and the winit half of `window`). The rest is `lib.rs` and `test_support.rs`.

### 3.2 Not carried

| Rust surface | Why |
|---|---|
| `egui_bridge.rs` (31 lines, 3 tests) | Cyrius has no egui (D8). The 3 tests are n/a in the ledger |
| winit window creation, the event loop, `WindowConfig.fullscreen` | D5. The application owns the window |
| wgpu types in public signatures (`wgpu::Device`, `TextureFormat`, …) | replaced by mabda handles and constants (ADR 0002) |
| `pollster`, `bytemuck`, `thiserror` and `Send + Sync` mechanics | no Cyrius counterpart is needed |
| thiserror message text | errors become `SR_ERR_*` codes, append-only |
| naga validation in tests | stays in the oracle crate until `rust-old/` leaves; then it is replaced by the pipeline-creation smoke test |

### 3.3 API changes in the Cyrius version (seed of ADR 0002)

Finalized at M0 step 2. Each row says how the harness still compares the function, so nothing leaves parity entirely.

| # | Rust 1.2.0 | Cyrius 2.0.0 | Comparison |
|---|---|---|---|
| 1 | `Result<T, RenderError>` with detail strings | value-form `Result` and an `SR_ERR_*` code | class A on the variant |
| 2 | soorat creates the device and window (`Window::new`) | the application passes a mabda `GpuContext` and a surface (D5) | — |
| 3 | wgpu types in the API | mabda handles | — |
| 4 | Matrices and geometry in f32 arithmetic | f64 arithmetic, rounded once to f32 (D4 rule b) | class E against Rust; ≤ 0.5 ulp against the f64 reference |
| 5 | Science inputs as Rust types (f32 in goonj) | the Cyrius producers' f64 types (§2.5) | the oracle feeds f32-representable inputs to both sides |
| 6 | `Vec` returns | caller buffers or a frame arena, plus explicit `_drop` | same contents (class A on counts) |
| 7 | One model matrix per `MeshPipeline` and `ShadowPipeline` (backlog: per-draw uniforms) | per-draw model uniforms through dynamic offsets (D2: an API fix in Cyrius only) | class G on multi-mesh scenes, against Rust renders drawn one mesh per pass |
| 8 | `MeshPipeline::draw` always clears depth | a load/clear choice on the draw parameters (D2) | class G |
| 9 | Cargo features | one bundle with dead-code elimination; each science crate an optional dependency | — |
| 10 | `egui_bridge` | removed (D8) | n/a |
| 11 | `drm` | agnodrm's subset (D8) | class A on what remains |
| 12 | `em_render` axis convention (if ported in 2.0, D7) | one convention for 2D data | rows mapped by name |
| 13 | CUBICSPLINE samplers keep each key's value and drop its tangents, so playback is linear (slerp for rotations) between keys | channels carry their interpolation mode and the in/out tangents, and CUBICSPLINE plays back as the glTF cubic Hermite spline (rotations normalized) | **class R** against the glTF spec formula, evaluated independently in f64. The 1.2.0 value-only rows stay for LINEAR and STEP, and for CUBICSPLINE at key times, where both agree |
| 14 | glTF verdicts on malformed input: 4 inputs that 1.2.0 accepts and bayan refuses (duplicate keys and an integer above i64::MAX in `extras`, both open bayan issues; nesting deeper than 128); 4 inputs that **panic** Rust 1.2.0 (`byteStride: 4` on POSITION, an out-of-range POSITION accessor, a float index type, an unknown target path); and document refusals D6 didn't list (POSITION without min/max, non-object `extras`, an unknown semantic, `extensionsRequired`, an invalid interpolation, mode > 6) | each is a defined `SR_ERR_*`; joint TRS numbers parse correctly rounded (bayan: 4,000 of 4,000; serde_json: 3,959) | class A on every other fixture; these rows are named in the harness (P9)
| 15 | `image` decodes up to its 512 MiB limit | chitra caps an image at 16,777,216 pixels, and the decode goes through a freeing allocator (`chitra_image_decode_budget_via`) | larger images are refused with `SR_ERR_*`; PNG/BMP class A, JPEG B-img (P10) |
| 16 | f32 `sin`, `cos`, `powf` and `expf` come from host glibc, which misrounds 0.73 % of soorat's inputs, each by 1 ulp (probe P3, port-parity §1.3) | correctly rounded: ganita's f32 tier (widen, compute, narrow) gave the reference value on all 226,050 of soorat's inputs; `acos` through `ganita_f64_acos`; `log10` through kimiya's correctly rounded `crlog` (stand-in, L1) | class C, K = 0, against `<module>.cr.tsv` where glibc misrounds; a pass is *accepted (cr)* |
| 17 | `tracing` events under `soorat::<module>`, with fields (`near=1.0 far=1.0`), for the application's subscriber | sakshi events at the same level, `soorat::<module>: <message>` (src/log.cyr); the field values are left out until soorat formats f32 as Rust's shortest round trip does, which the first module that needs it brings | not compared: log text is not API |
| 18 | `optics` Cargo feature: `Color::from_prakash(Rgb, alpha)`, `to_prakash() -> Rgb`, `from_temperature`, `from_wavelength -> Option` | an optional prakash 2.15.2 (`[features] optics`), its code in `src/color_optics.cyr`, outside `[lib]` and bundled by `[lib.optics]` as `dist/soorat-optics.cyr` (ranga's `spectral` precedent). `to_prakash` writes into an `Rgb` the caller owns; `from_wavelength` returns a value-form `Option` (Some(out) / None). `from_temperature` and `from_wavelength` leave prakash's 24-byte `Rgb` on the bump heap per call (ledger L21) | class A on all 1,704 rows of `tests/golden/color_optics.tsv`, replayed by its own harness (`tests/parity_features.tcyr`) so the core replay compiles without prakash |
| 19 | `#[derive(Serialize, Deserialize)]` on `Color`, read and written by serde_json (the tests' `to_string` / `from_str`), with `serde_json::Error` | `SrColor_to_json(c, sb)` and `SrColor_from_json(buf, len, out)` (`src/json.cyr`, on bayan; pravash's ADR 0002 #6 and #10 are the precedent). **Opt-in**, unlike Rust's unconditional derive: the `[lib.serde]` profile (`dist/soorat-serde.cyr`), because including bayan costs a binary ~372 KB of unicode tables (soorat's smoke program: 49 KB → 421 KB; bayan's open issue `2026-10-31-bundle-carries-unicode-tables-into-every-binary.md`). the Err the Cyrius-only `SR_ERR_DESERIALIZE` (appended as 12). The writer prints an f32 as bayan's round-trip digits of its exact f64 (serde_json prints its shortest), so the text differs and every document still reads back to the same bits in both. Three documents serde_json reads, bayan's tree can't, by its open issues: the integer `-0` (read as +0), an integer above `i64::MAX` and a duplicated unknown field (both refused) | class A on the decoded values of `tests/golden/json.tsv` (177 rows); the three bayan cases are rows of their own (`Color::from_json_bayan_*`), pending until bayan changes; every decode round-trips through the writer (`tests/color.tcyr`) |
| 20 | `SpriteBatch { sprites: Vec<Sprite> }`, grown by `Vec` and sorted by `sort_by_key` (stable, adaptive); `batch_to_vertices` returning `(Vec<Vertex2D>, Vec<u16>)`, or refilling the caller's `Vec`s (`_into`) | `SrSpriteBatch`: a freelist-backed array (doubling from 4; `clear` keeps the capacity) freed by `SrSpriteBatch_drop`. `sort_by_z` sorts the keys `(z << 32) \| index`, distinct and so stable by construction, returns after one pass on sorted input, and reuses its scratch arrays. `sr_batch_to_vertices(_u32)(b, a, out)` allocate from the caller's allocator (a frame arena) into an `SrQuadData`; the `_into` forms write caller buffers sized by `sr_batch_sprite_count`. `SrSprite`'s fields are reordered for alignment (68 bytes; no GPU layout) | class A on the sort permutations and the quads (`tests/golden/sprite.tsv`; the limit batches as FNV-1a digests); class C at 0 ulp for rotated quads against `sprite.cr.tsv` |
| 21 | `TextBatch::draw_text(&str, …)` walking `chars()`; `UiLabel { text: String, … }` | text is a pointer and a byte length, decoded as UTF-8 (`_sr_utf8_next`): a byte that begins no well-formed sequence (overlong, surrogate, past U+10FFFF, truncated) counts as one character U+FFFD, which a Rust `&str` can't hold. A `SrUiLabel` **borrows** its text (pointer and length), which must outlive the label's use; Rust's owns a `String` | class A on the glyph rectangles and the sprites of every text and UI batch (`tests/golden/text.tsv`, all valid UTF-8); the ill-formed cases are Cyrius-only tests |
| 22 | `physics-debug` Cargo feature: `LineBatch::collider(&impetus::collider::ColliderShape, …)` | an optional impetus 2.0.0 (3D bundle, `[features] physics-debug`), the code in `src/debug_collider.cyr`, bundled by `[lib.physics-debug]` as `dist/soorat-physics-debug.cyr`; the shape is impetus's `ImColliderShape` | class C at 0 ulp on all 60 rows of `tests/golden/debug_collider.tsv` (Rust impetus 1.4.0 against Cyrius impetus 2.0.0), replayed by `tests/parity_features.tcyr` with `color_optics` |

---

## 4. M0: prepare, then initialize with `cyrius port`

0. **Freeze a green Rust oracle (before the move).**
   1. ✅ **The oracle hooks** (done 2026-11-15, for 1.2.0): add the hidden `oracle` feature and its read-only test (D1), then re-run every gate.
      - `src/oracle.rs` re-exports `math_util` and `viz`, which are crate-private modules. It wraps the crate-private helpers of animation (bracket, interpolation, slerp, sampler pairing, local and world joint transforms), environment (sRGB and f16 conversions, the sRGB decode table, face encode/decode, downsample, mip chain), the shadow atlas tile math and the mesh normal matrix.
      - The library itself calls the same functions. `compute_joint_matrices` goes through `world_transforms`, and the cubemap upload goes through `face_mip_chain`. Outputs are unchanged, bit for bit, and a same-machine A/B run shows no speed change (CHANGELOG 1.2.0).
      - `tests/oracle.rs` has 18 tests. Each hook agrees with the public path it backs, and calling every hook leaves every public output bit-identical.
      - The 15 GPU-layout types and mabda's vertex types have all-public fields, so the generator measures layouts itself with `offset_of!` and no layout hook is needed.
   2. **D2's fixes, then the 1.2.0 release:** fix D2's value defects, each with a regression test that fails before the fix.
      - ✅ dt-scaled particle damping. The shader applies `damping^(dt·60)`, unchanged at 60 Hz. The GPU test gives x = 0.01667 on the old shader against the expected 0.00833.
      - ✅ The bloom composite: `PostProcessPipeline::set_bloom`/`clear_bloom` and `PostProcessUniforms::with_bloom_intensity`, with a GPU test (0 → 0.5 → 0).
      - ✖ Shallow-water bathymetry: withdrawn, not a defect (§6.3). A test now locks in the correct behaviour.
      - → CUBICSPLINE Hermite: moved to the Cyrius port (ADR 0002 #13; the maintainer, 2026-11-15). `Keyframe` and `AnimationChannel` have no place for tangents without breaking struct literals, and the plan sends API-shaped fixes to Cyrius only. The 1.x docs state the limitation.

      Then re-run every gate and the benchmarks, then add the CHANGELOG entries. ✅ Done, with the release benchmarks recorded in `bench-history.csv`. The maintainer tags **1.2.0**, the final Rust release and the oracle.
1. **Probes.** Each writes a dated result into `docs/development/port-parity.md`.
   - **P1 Layout.** For each of the 15 `#[repr(C)]` types and mabda's `Vertex2D`/`Vertex3D`, compare Cyrius `sizeof` and field offsets with Rust's `size_of`/`offset_of` and with the WGSL struct layout (vec3 alignment). This proves nested-struct uniforms, since array fields are unavailable.
   - **P2 f32 traps.** Port `math_util` and compile it under `cyrius test --bounds`. Zero warnings is the gate.
   - **P3 f32 transcendentals.** Compare ganita's f32 tier with Rust's glibc f32 on soorat's actual inputs: tessellation angles, slerp, perspective's `tan`, sRGB `powf`, `log10`/`ln`/`exp` in the viz modules. Record the ulp histogram and the glibc version.
   - **P4 One hisab.** Build goonj, prakash, ushma, pravash and impetus bundles together on hisab 3.5.0 (D9).
   - **P5 mabda on 6.7.18.** The mabda bundle must build with zero warnings under `test --bounds` on cyrius 6.7.18.
   - **P6 WGSL through mabda.** Create all nine shader modules through `gpu_shader_module_create` on the Cezanne via wgpu-native. Record the deprecation warning, and confirm strict mode is off.
   - **P7 Upload cost.** Time 1,000 sprites per frame through the `memcpy` byte loop (ranga measured about 30 ms per native GPU call against 2.4–3 ms on wgpu).
   - **P8 Frame arena.** Run the sprite and debug-line batches for 10⁵ frames with RSS flat.
   - **P9 GLB.** The `test_support` fixtures parse through bayan.
   - **P10 Decoders.** chitra against `image` on PNG, JPEG and BMP fixtures: PNG and BMP must be bit-identical; measure JPEG's error against the source pixels (selah's B class).
   - **P11 Names.** A full `full`-feature bundle has no duplicate-symbol warnings.
2. **Run `cyrius port`, then repair what it clobbers** (as tanmatra §2 step 2):
   - it overwrites CLAUDE.md, the roadmap, `state.md` and CI;
   - it writes `version = "0.1.0"`;
   - it leaves `bench` out of `[test] stdlib`;
   - it writes undocumented stubs, so `cyrius audit` fails.
3. **`cyrius.cyml`:**
   - `[deps]`: mabda, chitra, bayan, prakash, ranga, impetus, pravash, **goonj before ushma** (P4: that order resolves hisab 3.5.0), ushma and selah, with bijli once D7 allows. **agnodrm only behind a `drm` feature**: Rust's `full` excludes `drm`, and agnodrm clashes with sigil when selah is present (P11, L19). The science crates are optional per feature;
   - `[embed]`: the nine shaders;
   - `[lib]`: bundle order;
   - `[build] dce = true`;
   - `[test]` with `bench`.
4. ✅ **The golden generator** (done 2026-11-15): `scripts/oracle/`, driving `rust-old/` by path, its lock seeded from 1.2.0's. See `scripts/oracle/README.md`.
   - TSV rows as tanmatra's, with a new **f32 token** `g:<8 hex>` beside `f:<16 hex>`.
   - A header recording the platform (glibc, CPU FMA). `--check` refuses a corpus from another platform. The adapter and Mesa are recorded by the class-G corpus (step 6).
   - Deterministic: two runs are byte-identical.
   - M0 modules:
     - `constants`, 18 rows;
     - `layout`, 19 rows (P1);
     - `math`, 1,436 rows, `math_util` on P2's grid, class E and A.
   - `validate.py` re-derives every `math` row bit for bit without Rust, by emulating f32 in Python f64. It writes `math.e.tsv`: 1,149 rows where Rust's formula in f64, rounded once, differs from Rust's f32 outputs. It also checks `constants` against `rust-old/src`.
5. ✅ **The parity harness** (done 2026-11-15): `tests/parity_core.cyr` and `tests/ulp.cyr` from pravash, with:
   - the `g:` token, and `ulp32_*` on the 32-bit ordered line;
   - class E, through `<module>.e.tsv` (pravash's `.d3.tsv` mechanism);
   - the correctly rounded `.cr.tsv` rows (ADR 0002 #16).
   
   `tests/parity.tcyr` replays all 1,473 rows. At M0 every row is pending and the run exits 0. It runs in CI through `cyrius test`, and `validate.py` runs in CI too.
6. ✅ **The render-reference harness (class G)** (done 2026-11-15): `scripts/render_ref/` renders the reference scenes with Rust 1.2.0's public API on the Cezanne and writes `tests/golden/render/`, 180 KB. See `scripts/render_ref/README.md`.
   - **The files:**
     - the scene definitions, `scenes.json`, which the renderer itself reads; every number is an exact f32;
     - the call sequences and fixed state, `scenes.md`;
     - 12 PNG readbacks (128²);
     - `manifest.tsv`, which records the adapter, driver, Mesa, glibc, wgpu and soorat versions;
     - `uploads.tsv`, with the size and sha256 of every CPU buffer each scene uploads, so the Cyrius side can check its uploads before comparing pixels.
   - **Determinism:** two renders on separate devices are byte-identical. `--check` exits 1 on a difference and 2 on another adapter or stack.
   - **Scenes:** `sprite_grid`, `debug_lines`, `pbr_lit`, `shadow_mesh`, `ibl_sphere`, `post_bloom_tonemap` (with `set_bloom`), `post_ssao`, `gpu_particles` (simulated, then drawn as sprites), and `science_acoustic`, `science_thermo`, `science_fluid` and `science_em`. **The skinned mesh was dropped:** 1.2.0 has no public pipeline that builds `pbr_skinned.wgsl` or binds `JointUniforms`.
   - **What 1.2.0's public API forced, which M6–M8 must decide on** (each becomes an ADR 0002 fix or a class-G-matched behaviour):
     1. no public skinned pipeline;
     2. no particle render pipeline, since nothing draws `InstanceData`;
     3. no SSAO normals pass, so a constant +Z normal texture is used;
     4. camera math (`look_at`, `perspective`) is crate-private, so the scenes store the matrices, derived once by `derive_camera.py`;
     5. `MeshPipeline::draw` always clears depth and has one model uniform (ADR 0002 #7, #8), so multi-object scenes are one concatenated mesh;
     6. `LinePipeline` loads depth while a new `DepthBuffer` is zero-filled, so lines fail the depth test unless a draw clears depth first;
     7. a zero-filled shadow map shadows light 0 everywhere until a shadow pass clears it to 1.0.
   - The Cyrius side renders the same `scenes.json` once mabda's render tier lands (M6).
7. ✅ **Tooling** (done 2026-11-15): `scripts/surface_parity.py`, `rust_test_ledger.py`, `bench_record.py`, `doc_examples.py` and `upstream_ledger.py`, adapted from pravash and impetus. They run in CI as the `port-tooling` job.
   - A milestone counts as done when its §14 row has ✅. Every checker takes `--done Mn` to preview a gate.
   - Ported tests are `test_group("<Rust test name>")` (`tests/README.md`).
   - The Cyrius bench history is `bench-history.csv` at the repository root.
   - Today: 912 public items (827 pending, 77 n/a, 8 deferred); 502 Rust tests (471 pending, 28 n/a, 3 deferred); 35 benches pending; 16 ledger rows with 14 filed paths, all present.
8. ✅ **Baselines** (done 2026-11-15): the Rust bench column of `docs/benchmarks-rust-v-cyrius.md` is the 1.2.0 release run (`rust-old/bench-history.csv`, 2026-11-15T21:19Z), written by `bench_record.py`. The M0 corpus has no class-C rows, so there are no C bounds to set yet; each milestone sets its functions' bounds at its first replay. The §10 requests are filed.

---

## 5. Parity policy

tanmatra's classes, with selah's image classes and two additions for soorat.

| Class | Tolerance | soorat examples |
|---|---|---|
| **A — exact** | identical bits or integers | u8 colours (`to_rgba8`, `from_hex`); sRGB tables; f16 packing at every edge (subnormal, overflow, NaN, ties); index buffers, winding, counts and clamps (`MAX_SEGMENTS`, `MAX_QUADS`, `MAX_PARTICLES`, `MAX_SPRITES_PER_BATCH`); glyph lookups; `UvRect` (computed in f64 in 1.1.0); keyframe brackets; glTF accept/reject verdicts; struct layouts (P1); the u8 BRDF LUT |
| **B — arithmetic** | ≤ 2 ulp, expected 0 | f32 paths kept bit-faithful under D4(a): batch quad generation, sprite transforms, terrain normals (sqrt only), box-filter mips, colour lerp and luminance |
| **C — transcendental** | ≤ K ulp by tanmatra's rule (K_F = 2·S·c + 2); 0 against the correctly rounded rows | f32 sin/cos tessellation where D4(a) keeps f32; `log10`/`ln`/`exp` in the viz modules; sRGB `powf` (the table itself is class A) |
| **E — improved** *(new)* | the port evaluates Rust's formula in f64, with the f32 inputs and constants widened exactly, and rounds once. Where that differs from Rust's f32 outputs, the expectation is the row in `<module>.e.tsv` (`scripts/oracle/validate.py`), compared **bit for bit**. Elsewhere it equals Rust's, bit for bit. Each row's distance from Rust's f32 output is reported as the gain. *(Made exact at M0 step 4: the earlier draft said ≤ 0.5 ulp against an f64 reference.)* | D4(b) outputs: `look_at`, `perspective`, `ortho`, `directional_light_matrix_at`, `inverse_transpose_3x3`, joint matrices, slerp, cascade splits, sphere/cylinder/balloon vertices, viz normalization. The harness prints both distances, so each row shows the gain |
| **G — image** *(new)* | on the same adapter and driver: **exact** where the CPU uploads are bit-identical. Where a class-E upload differs, per channel ≤ 1 LSB on ≤ 0.1 % of pixels (a provisional bound, set at M6 from measurement) | every reference scene in §4 step 6 |
| **B-img** (selah) | against ground-truth pixels: mean ≤ Rust's + 0.05, max ≤ Rust's + 2 | JPEG-decoded textures (P10) |
| **R — reference** | the tolerance stated in the test | winding against normals; unit normals; mip averages; white → white; slerp endpoints; joint chains against an independent f64 model; the 12 integration tests |

Rules:
- **Never widen a class to make a row pass.** A failure is a port defect, an upstream defect (filed), or a 1.2.0 defect (fixed as 1.2.x and regenerated). The exception is an ADR-accepted divergence, which the harness still names row by row.
- **Branch points, tested on both sides:**
  - slerp's `cos > 0.9995` nlerp switch;
  - the shadow guards `extent ≤ 0`/NaN and `depth_span < 1e-10`;
  - the split sanitizing (near, far, λ, count);
  - grid backing (`product(dims) ≤ len`, checked);
  - the sRGB thresholds 0.04045 and 0.0031308;
  - f16's subnormal and overflow edges;
  - the 4..=1024 segment clamp;
  - `partition_point`'s ends;
  - non-finite inputs to every colour map.
- **Class G belongs to one machine.** The render corpus is recorded on the dev box and replayed there. CI runs classes A–E and R, and the GPU smoke tests skip without an adapter. Class G is gated per milestone locally, as mabda's HW gates are.
- **D4(a) wins ties.** If a value is both bound for the GPU and quantized (for example a u8 vertex colour), it is quantized from Rust's f32 sequence.

---

## 6. Expected accuracy: Rust 1.2.0 → Cyrius 2.0.0

The expectations below are **predictions with a mechanism**. Each is measured at its milestone and recorded in `port-parity.md`, so the claim becomes a number. The sibling measurements that back the method come first.

### 6.1 What the sibling ports measured

| Port | Measured gain over Rust | Mechanism |
|---|---|---|
| kimiya | glibc misrounds **0.1–5 %** of transcendental calls (log10: 4.6 % of the golden arguments). The port is correctly rounded on ≥ 10⁶ arguments per primitive against MPFR. CFSE −2.2e-16 → **exactly 0**. Rydberg λ was off by up to 9.0·10⁷ ulp in Rust | correctly rounded primitives; rationals rounded once |
| impetus | glibc misrounds 0.105 % of `exp`, 0.14 % of `sin`, 0.15–0.17 % of `cos` | correctly rounded `crmath` (ADR 0011) |
| pravash | 43 math rows corrected; f32 neighbour keys → f64; **bit-identical across x86_64 ELF, PE and aarch64**, which Rust does not promise | correctly rounded math; precision fixes as ADR rows |
| ranga | blur bias mean −1.00 → **0.00** (worst −1.94 → 0.50); bilateral −0.47 → 0.00; JSON floats 4,000/4,000 decoded correctly against serde_json's 2,782/4,000 | algorithm fixes; correctly rounded parse |
| ranga (warning) | 11 colour functions computed in f64 were "more accurate and therefore wrong": f32 `0x3f010101` gave **129 in Rust, 128 in the port** | the reason for D4(a) |
| selah | three Rust defects fixed, among them `image`'s JPEG FDCT rounding with 2¹⁰ where 2¹⁴ is right | ground-truth image classes |

### 6.2 soorat, area by area

| Area | Rust 1.2.0's error source | Cyrius 2.0.0 approach | Expected | Class |
|---|---|---|---|---|
| u8 colours, sRGB tables, f16, BRDF LUT bytes, UV rects, glyphs | f32 sequence; `round` half away from zero | Rust's sequence exactly; `sr_round`; tables generated from the oracle | **identical** (no gain, by design) | A |
| Matrix builders (`look_at`, `perspective`, `ortho`, the directional light matrix) | each element is several roundings deep in f32 (dot products, `tan`); typically 0–2 ulp, unbounded relative error under cancellation (for example the translation column for an off-origin center) | f64 arithmetic, one rounding at packing | **≤ 0.5 ulp per element** against the f64 reference (except listed double-rounding rows); against Rust, the gain shows on rows with cancellation | E |
| Normal matrix (`inverse_transpose_3x3`, cofactors / det) | relative error ≈ κ·u_f32 (u = 2⁻²⁴), growing with non-uniform scale | f64 cofactors and det | ≤ 0.5 ulp for κ ≲ 10⁶ (the f64 work keeps about 2²⁹ more headroom); identical where κ is small | E |
| Joint matrices (`compute_joint_matrices`, chains up to 64) | f32 error grows with chain depth (≈ depth × a few ulp at the leaf) | accumulate world transforms in f64, round once per joint | leaf joints **≤ 0.5 ulp** regardless of depth; the gain grows with depth (the 64-joint bench chain is the headline row) | E, R |
| Quaternion slerp | f32 `acos` and `sin(θ)` in the denominator; relative error ∝ u/θ near the 0.9995 switch (θ ≈ 0.032 rad, so tens of ulp) | f64 `acos`/`sin` (ganita has no f32 `acos`), one rounding | **≤ 0.5 ulp** across θ; the largest gain sits just above the nlerp switch | E |
| Cascade splits (practical split, `powf`) | f32 `powf` plus the blend | f64 `pow` (ganita, ≤ 1 ulp; correctly rounded stand-in where parity needs it) | ≤ 0.5 ulp | E |
| Tessellation (sphere, cylinder, balloon, charges) | the angle `i as f32 * step` is rounded before `sin`/`cos` | f64 angle and f64 `sin`/`cos` | vertices ≤ 0.5 ulp; seam vertices meet to within 1 ulp | E |
| Terrain and height-field normals | sqrt-only f32 arithmetic, ≤ about 2 ulp | kept f32 (class B) **or** f64 under D4(b). Decided at M3 from P3's cost | 0 against Rust (B), or ≤ 0.5 ulp against f64 (E) | B/E |
| Science visualization (normalization, colour maps, heights) | inputs arrived as f32 from Rust goonj (rounded at the source, about 6e-8 relative); normalization in f32 | f64 inputs from the Cyrius producers; normalize in f64; narrow at packing | ≤ 0.5 ulp per vertex against f64; the **input rounding step disappears** for acoustics | E |
| f32 transcendentals kept in f32 | glibc f32 (version in the corpus header) | ganita's f32 tier: widen, compute within 1 ulp, narrow | about 1 % of values differ from Rust, all ≤ 2 ulp (ranga ADR 002: 1,419 and 1,830 of 140,608); ≥ 99 % correctly rounded | C |
| JPEG-decoded textures | `image` 0.25's decoder | chitra 1.4.1 | not bit-identical; held to selah's B bar (mean ≤ Rust + 0.05, max ≤ Rust + 2) | B-img |
| PNG and BMP textures | lossless | chitra | identical | A |
| Rendered pixels | the GPU, in f32 | same WGSL, same driver | **identical** where uploads are identical; ≤ 1 LSB on ≤ 0.1 % of pixels where class-E uploads move by ≤ 1 ulp (edge coverage and .5 rounding) | G |
| Cross-target determinism | Rust promises none (glibc, FMA contraction) | `_f64_*_polyfill` forms and correctly rounded stand-ins | bit-identical CPU outputs on x86_64 ELF, PE and aarch64 (as pravash) | A–E |

### 6.3 Gains that are not arithmetic

These belong to D2: fixed in Rust 1.2.0 before the freeze, so both columns carry them. D2 also listed shallow-water bathymetry, but that was not a defect: pravash stores `height` as the surface elevation η (depth is `height − ground`), so soorat already renders the free surface.

| Defect | Change |
|---|---|
| dt-scaled particle damping | frame-rate independent |
| bloom composite | bloom output is actually used |

These are fixed in Cyrius only, through ADR 0002:
- per-draw model uniforms (multi-mesh scenes become correct);
- depth load/clear;
- **CUBICSPLINE Hermite interpolation** (#13): animations follow the authored curve instead of a linear path between keys. Rust has no row for it, so it is checked against the glTF formula (class R).

---

## 7. Milestones

Every milestone ends with pravash's gates:
- `cyrius build`;
- `cyrius test` and `test --bounds` with **zero warnings** (the f32 and f64 type diagnostics appear only there);
- `cyrius bench`, recorded against Rust;
- `vet`, `lint`, `fmt --check`;
- `distlib --all`, `distlib --check`, `deps --locked`;
- surface parity; the test ledger; `upstream_ledger.py --check`;
- on x86_64 ELF, x86_64 PE and aarch64 for the CPU classes;
- class G on the dev box for GPU milestones;
- the CHANGELOG and `state.md`.

Large milestones go in parts, each part gated.

| Milestone | Modules | Key work | Golden grid (minimum) | Gate |
|---|---|---|---|---|
| **M1 Foundation** (parts a–d) | a: error, log, math. b: mathx, layout, color (b2: its `optics` half and serde). c: lights, lod. d: tables (f16, sRGB; crmath moved to M5, its only caller's milestone). `material` moved to M6 (a GPU object) | `RenderError` → `SR_ERR_*`. Rust-semantics helpers (`sr_round`, checked ops, `sr_partition_point`, stable sort). f16 and sRGB with generated tables. Mat4 and vector ops with D4(b) packing. All 15 layouts proved (P1) | every f16 edge; the full sRGB domain; colour hex parsing including malformed input; matrices at degenerate inputs (zero up, zero extent, NaN); every layout row | C |
| **M2 2D** (a–b) | sprite, batch, text, ui, debug_geom | sprite batching with the u16 index limit, the stable z-sort, UV rects; bitmap-font layout; UI quads; wire shapes and impetus collider shapes | batches at 0, 1, the limit and the limit + 1; sort ties; every glyph; each `ImColliderShape` kind | C |
| **M3 Geometry and animation** (a–c) | primitives, terrain, viz, animation, gltf | outward winding everywhere; height fields with non-finite heights; keyframe sampling (LINEAR, STEP, and CUBICSPLINE with Hermite tangents per ADR 0002 #13); slerp; unsorted and cyclic joint chains; the GLB loader with every 1.1.0 rejection | tessellation counts 3…1024; slerp across θ including the switch; chains of 1–64 joints, sorted and unsorted; the `test_support` GLB fixtures plus truncated and malicious ones | C |
| **M4 Lighting math and infrastructure** | shadow_math, env_cpu, pbr_material, render_graph, profiler, capabilities | directional light matrices (including the centred variant); practical splits; the atlas; mip chains and box filters; the BRDF LUT bytes; the render graph over mabda `rg_*` | splits over (near, far, λ, count) including bad input; atlas tiling at 1…64 lights; mips for sizes 1…2048; the LUT at every resolution 1.1.0 allows | C |
| **M5 Science visualization** (a–d) | acoustic, thermo, fluid, em (D7) | bind the Cyrius producers (§2.5); grid backing checks; NaN-safe colour maps; segment and quad caps | each renderer at empty, minimal, backed, unbacked, NaN and huge headers; f32-representable inputs fed to both sides | C |
| **MB1–MB5** (mabda) | see §8 | the mabda render tier | mabda's own gates, plus soorat's P6 | mabda |
| **M6 GPU core** (a–c) | texture, render_target, compute, sprite_pipeline, line_pipeline, mesh_pipeline | the first pixels: sprites, lines, a lit mesh, depth, per-draw uniforms (ADR 0002 #7) | class G: sprite grid, lines, single and multi-mesh scenes | G (MB1, MB2) |
| **M7 Shadows, IBL, PBR** (a–c) | shadow, environment, the PBR and skinned paths | the shadow map pass and comparison sampling; cube maps with a CPU mip chain; PBR + IBL; 4-bone skinning | class G: shadowed mesh, IBL sphere, skinned mesh at fixed times | G (MB2, MB3) |
| **M8 Post, particles, screenshots** (a–c) | hdr, postprocess, ssao, gpu_particles, screenshot | the HDR target, bloom (composited after D2), tonemap, SSAO; the particle compute; readback and encode (D8) | class G: post chain, particles after N steps; screenshot bytes class A for PNG and BMP | G (MB3, MB4) |
| **M9 Integration** | surface, drm, examples | the application-owned window path (D5) on a Wayland test client; the agnodrm subset; the 1.1.0 example ported | an end-to-end frame presented and read back | G (MB5) |
| **M10 Release 2.0.0** | all | Rust-test ledger (all 502: 471 to port, 28 n/a, 3 deferred; `scripts/rust_test_ledger.py`); the full surface diff; `docs/benchmarks-rust-v-cyrius.md` (35 rows); the README; `docs/guides/migration-2.0.md`, led by D5; the CHANGELOG's Breaking section; doc examples compiled and run; VERSION 2.0.0 | full corpus green on 3 targets; class G green on the dev box | — |

**Order and parallelism.**
- M0 → M1 → M2 → M3 → M4 → M5 is the CPU track. It runs while mabda does MB1–MB5.
- M6 waits for MB1 + MB2, M7 for MB3, M8 for MB4, M9 for MB5.
- M5's `em` part follows D7 whenever bijli 2.0 ships.
- If kiran needs pixels early, M6 can start on MB1 + MB2 alone, ahead of M4–M5.

---

## 8. The mabda render tier (MB1–MB5)

This is done in mabda's repository, by mabda's process: its own gates, its own CHANGELOG, and HW verification on the dev box. soorat is its first full consumer, and soorat's M0 probe P6 is mabda's acceptance check. **All of it is wgpu-tier first.** Native-backend parity is mabda's choice per item, and `@public` API stays identical across backends where mabda can promise it.

| MB | Adds to the `gpu_*` public tier | Launcher slots (`deps/wgpu_main.c`) | Unblocks |
|---|---|---|---|
| **MB1** | A pipeline descriptor: vertex layouts (2D, 3D, skinned, line), topology (TriangleList, LineList), culling, front face, blend state, explicit bind-group layouts (uniform, storage, texture, sampler), several bind groups. Draw-time calls: set vertex and index buffer (u16 and u32), `draw_indexed`, bind group with dynamic offsets, viewport, scissor | SetVertexBuffer, SetIndexBuffer, DrawIndexed, SetBindGroup (offsets), SetViewport (**float arguments: a C shim**, per fncall-abi), SetScissorRect, plus device-side layout creation | M6 |
| **MB2** | Depth32Float targets and depth state (compare function, write, bias); depth-texture binding with a depth sample type; **comparison samplers** (LessEqual) on the `gpu_*` tier; render-pass depth load/clear | — (descriptor contents only) | M6, M7 |
| **MB3** | `Rgba16Float` (and `Rgba8UnormSrgb` without collapsing to UNORM) render targets; MSAA plus resolve; several colour targets; rendering into a cube face or mip level; mip generation on wgpu; texture-to-texture copy; render-target readback | CopyTextureToTexture | M7, M8 |
| **MB4** | compute on the `gpu_*` tier for wgpu: today `_backend_wgpu_compute_dispatch` returns `GPU_ERR_OTHER`. Storage plus uniform bind groups | — (existing compute slots) | M8 |
| **MB5** | surface creation from a Wayland `wl_display`/`wl_surface` pair; BGRA swapchain formats; present modes (AutoVsync, AutoNoVsync) | InstanceCreateSurface (a chained `WGPUSurfaceSourceWaylandSurface` descriptor), SurfaceGetCapabilities | M9 |

Land MB1–MB5's launcher slots as **one** `wgpu_main.c` change if the scope is known up front, so every mabda consumer rebuilds once (§2.1). That change should also carry the command-buffer release from L17.

Probe P6 (port-parity §1.5) confirmed the table above by running it, and adds:
- **MB4 is missing, not partial.** `compute_pipeline_new` builds storage-only layouts and cannot make soorat's particle pipeline.
- MB1 needs **explicit entry-point names** (bloom's `fs_threshold`/`fs_blur_h`/`fs_blur_v`, postprocess's `fs_no_bloom`) and an **optional fragment stage** (shadow is depth-only).
- MB1 needs an **error channel**, because a validation error currently aborts the process.
- Public calls should take constants with public names.

P8 found that the render loop leaks (L17).

All of this is filed in mabda: `docs/development/issues/2026-11-15-render-tier-for-geometry-consumers.md` and `2026-11-15-wgpu-render-loop-leaks-command-buffers-and-pass-records.md`.

---

## 9. Rust → Cyrius mapping (soorat specifics)

### 9.1 Math functions

Every std f32 method in `src/` outside tests, as of 1.1.0:

| Rust (calls) | Cyrius | Parity |
|---|---|---|
| `abs` (118), `clamp` (26) | f32 bit operations; ganita's f32 `clamp` propagates NaN like Rust | A |
| `sqrt` (15) | f32 tier (correctly rounded through f64), or f64 under D4(b) | B / E |
| `sin` (19), `cos` (16), `sin_cos` (4) | f64 `f64_sin`/`f64_cos` (identical bits on every target; need `lib/math.cyr`) under D4(b); ganita's f32 tier otherwise | E / C |
| `acos` (2, slerp) | ganita f64 `acos` (no f32 version) | E |
| `powf` (3), `exp` (2), `ln` (1), `log10` (1) | ganita `pow`; `_f64_exp_polyfill`/`_f64_ln_polyfill` for cross-target identity; `log10` is a correctly rounded stand-in (kimiya's `crlog`, L1) | C / E |
| `round` (8) | `sr_round` (half away from zero) | A |
| `powi` (3), `rem_euclid` (2), `ceil` (1) | `mathx` transcriptions of Rust's semantics | A / B |

### 9.2 Types and idioms

- **Vectors and matrices:** `SrV3`/`SrV4` and `SrM4` (column-major, 64 B), always passed and returned by pointer.
- **Colours:** `SrRgba` (16 B), returned through an out-pointer.
- **Vertex streams:** byte buffers with a stride constant per layout. Written with `store32` of f32 bits; never a `vec` of words.
- **Handles:** mabda handles for every GPU object; soorat objects hold them and free them in `_drop`.

### 9.3 Names

- Types use `Sr`, functions `sr_`, error codes `SR_ERR_*`, constants `SR_*`.
- **Never `soorat_`**: goonj owns those names (§2.5).
- Rust method `Foo::bar` becomes `SrFoo_bar` (as pravash's `PvShallowWater_*`).

### 9.4 Memory

The global heap is a bump allocator with no free. `alloc_reset` invalidates everything.
- **Per frame:** sprite and line batches, viz meshes and staging data go in a frame arena reset by the caller once per frame. P8 proves RSS stays flat over 10⁵ frames.
- **Long-lived:** objects (pipelines, textures, meshes) use `freelist` plus explicit `_drop`.
- **Upload staging:** reused across frames, not reallocated.

---

## 10. Upstream requests and the conversion ledger

Each row is filed in the named project with soorat's numbers. Each has a stand-in where one is allowed (§0.1) and an `UPSTREAM(Ln)` note at its use.

| L | Project | Request | Stand-in |
|---|---|---|---|
| L1 | ganita | correctly rounded f64 tier (the open request; soorat joins as a consumer). **soorat's P3 numbers added 2026-11-15** to `ganita/docs/development/issues/2026-11-09-correctly-rounded-exp-sin-cos-hypot.md`, with a request for f32 `acos`/`asin` and a correctly rounded `log10` | vendored `crmath`/`crlog` (impetus, kimiya) |
| L2–L6 | mabda | MB1–MB5 (§8), **filed 2026-11-15** as `mabda/docs/development/issues/2026-11-15-render-tier-for-geometry-consumers.md`. It includes P6's additions: explicit entry points, an optional fragment stage, an error channel, public constant names, and MB4 as missing | **none allowed** (D3, §0.1); the GPU track waits |
| L17 | mabda | the render loop's leaks (P8): command buffers never released, plus 432 B per pass begin and 224 B per textured draw from the bump heap; about 1.6 KB a frame. **Filed 2026-11-15** as `mabda/docs/development/issues/2026-11-15-wgpu-render-loop-leaks-command-buffers-and-pass-records.md` | none (a launcher change). It lands with L2–L6's single launcher revision |
| L7 | mabda | a WGSL front end (or another render-capable shader path) for the native backends | none; after 2.0 (§13) |
| L8 | cyrius | `fn f(): f32` returns; `const` of f32; an f16 helper pair. **Filed 2026-11-15:** `cyrius/docs/development/issues/2026-11-15-f32-returns-consts-and-f16.md` | return the bits as i64; `var` constants; local f16 code |
| L9 | selah | a public raw-RGBA encoder: PNG, JPEG, BMP. **Filed 2026-11-15:** `selah/docs/development/issues/2026-11-15-public-raw-rgba-encoders.md` | selah's private encoders, vendored |
| L10 | agnodrm | connector, mode and render-node queries (mabda's `backend_native_kms.cyr` has the ioctls internally). **Filed 2026-11-15:** `agnodrm/docs/development/issues/2026-11-15-connector-mode-and-render-node-queries.md` | the reduced `drm` subset (D8) |
| L11 | hisab consumers | prakash, ushma, pravash and impetus move to hisab 3.5.x (goonj's pin). *P4: tidy-up only.* 3.5.0 serves all five; prakash, pravash and impetus are test-scoped; ushma calls only `calc_lerp` and `calc_integral_simpson`. **Filed 2026-11-15** (ushma, the only production 3.4.1 pin): `ushma/docs/development/issues/2026-11-15-hisab-pin-3-5.md` | P4's result decides whether this blocks the `full` bundle |
| L12 | mabda | toolchain pin 6.7.5 → 6.7.18 (*P5: tidy-up*, 0 warnings on 6.7.18), plus making `[deps.chitra]` and `[deps.samvada]` optional: as production dependencies they reach every consumer's compile. **Filed 2026-11-15:** `mabda/docs/development/issues/2026-11-15-cyrius-pin-and-production-only-deps.md` | P5's result |
| L18 | **hisab or ranga** | **name clash.** Both define `srgb_to_linear`, `linear_to_srgb`, `premultiply_alpha` and `unpremultiply_alpha`. The last two differ in arity, which is a compile **error**; the first two silently take hisab's, and ranga's `srgba_to_lin_rgba(128).r` becomes `0xa` instead of `0x3e5d0a8b` (P11). One side must prefix or privatize. **Filed 2026-11-15:** primary in hisab, `hisab/docs/development/issues/2026-11-15-color-names-collide-with-ranga.md` (rename hisab's four colour functions; nothing outside hisab calls them). Companion in ranga, `ranga/docs/development/issues/2026-11-15-color-names-collide-with-hisab.md` (correct `spectral.cyr`'s rename-around guidance; add a co-compile test) | **none possible** (a dependency's names): blocks `ranga` together with `acoustics`/`thermo` |
| L19 | agnodrm or sigil | name clash: `result_print_err`, `is_syscall_err`, `wrap_syscall`, plus the agreeing `Errno`/`SysErrorKind` enums (P11). **Filed 2026-11-15** in agnodrm (prefix its helpers; sigil is the stdlib): `agnodrm/docs/development/issues/2026-11-15-syscall-result-helpers-collide-with-sigil.md` | agnodrm behind `drm` only |
| L20 | goonj or pravash | name clash: `logging_init` (P11). **Filed 2026-11-15** in both (each prefixes its own init): `goonj/docs/development/issues/2026-11-15-logging-init-collides-with-pravash.md`, `pravash/docs/development/issues/2026-11-15-logging-init-collides-with-goonj.md` | none; warning only, last definition wins |
| L21 | prakash | allocation-free (out-pointer) forms of `color_temperature_to_rgb` and `wavelength_to_rgb` (and `rgb_clamp`, `rgb_to_u8`): each returns a fresh 24-byte `Rgb` from the bump heap (M1 part b2: `alloc_used` grows 24 B per `from_temperature`). **Filed 2026-11-15:** `prakash/docs/development/issues/2026-11-15-allocation-free-rgb-results.md` | the allocating forms, called by `SrColor_from_temperature` / `from_wavelength` (setup-time use documented) |
| L13 | cyrius | `memcpy`/`memset` word copies (the open byte-loop issue). **soorat's P7 numbers added 2026-11-15** to the open `cyrius/docs/development/issues/2026-11-02-memcpy-memset-byte-loops-dominate-frame-copies.md` | measured by P7; arena reuse |
| L14 | cyrius | `cyrius port` doc clobbering (the open issue). **Reproduced on soorat and repaired, 2026-11-15**; noted on the issue, with the release-workflow consequence | M0 step 2's repairs (`state.md`) |
| L15 | new project | a shared Wayland window and input library, extracted from the four `win_*` copies | none needed by 2.0 (D5) |
| L16 | bijli | M7's visualization surfaces (in progress) | D7 |

---

## 11. Performance expectations

These come from the sibling records. They are not gated; each row is measured.

| Kind of work | Sibling evidence | Expected Cyrius time / Rust time for soorat |
|---|---|---|
| Tiny call-bound functions (`from_hex`, `lerp`, `luminance`, `to_array`) | bijli 13–114×; tanmatra up to 43× | **10–50×** (no general inlining; `#inline` limits) |
| Loops over buffers (batch to vertices, quad generation, wire shapes, terrain, sphere) | pravash median 4.4×; ranga median 7.9× | **4–10×**. D4(b)'s f64 paths add little |
| Joint matrices, shadow matrices | impetus whole steps 2.6–17× | **5–15×** |
| PNG encode (`encode_png_256`) | selah PNG decode 17× (1080p: 47.5 ms against 1.30 ms) | **15–50×** |
| GPU calls (pipeline build, submit) | mabda `render_pipeline_build` 0.64×; GPU-call benches 0.24–1.33× | **≈ 1×** |
| A frame | dominated by GPU time for any real scene | **near parity**. CPU-side batching for 10³ sprites stays well under 1 ms if P7 holds |

Where the gap matters (sprite batching at scale), the remedies are the sibling ones:
- caller buffers instead of per-call allocation (pravash SPH 0.76×);
- SSE2 `asm` kernels (ranga `blend_row` 0.5×);
- `memcpy` word copies (L13).

---

## 12. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| MB1–MB5 are larger than they look (pipeline descriptors, the launcher, two native backends that the `@public` API promises to match) | the GPU track waits | CPU track first; MB scope fixed at M0 with mabda; wgpu tier first, native parity per item |
| wgpu on AMD is deprecated; strict mode or the v5.1 retirement would cut soorat off | soorat cannot render on AMD | D3 keeps strict off; §13 plans the native path; mabda's own gate waits for soorat |
| Result-ABI pitfalls (8-byte payloads, 16- and 32-byte structs, `callptr`) | crashes in otherwise correct code | out-pointers by convention (§9.2); fuzz harnesses on every entry that takes external data |
| Bump allocator: per-frame allocation leaks | unbounded RSS in a render loop | frame arena (§9.4); P8 soak test gated |
| The f32 operator rule (`x * 2.0` reads 0) | silent wrong values | `test --bounds` zero-warning gate; class B/E rows on every f32 function |
| ~~The 24 MB preprocess cap~~ (retired by P11: the full bundle is ≈ 17.6 MB, 70 % of the cap; DCE takes the binary from 9.7 MB to 2.6 MB) | — | — |
| **Dependency name clashes** (P11): the flat namespace makes two dependencies' same-named functions collide, as a compile error when arities differ and as a silently wrong result when they agree | `full` doesn't compile; ranga's sRGB is silently wrong | L18–L20 requests; P11's `names.py` scan rerun at every dependency bump |
| Class G needs the dev box's GPU | GPU parity isn't checked in CI | CPU classes in CI on 3 targets; class G per milestone locally, as mabda's HW gates are |
| bijli M7 slips | no `em` in 2.0 | D7 |
| Consumers (kiran, salai, joshua) aren't local and still Rust | API decisions made blind | migration guide by M10; review D5 with the kiran maintainer before M6 |

---

## 13. After 2.0.0

- **Native backends.** Once mabda has a render-capable shader path (L7), soorat's nine shaders move to it, and soorat leaves the deprecated wgpu-on-AMD route. This satisfies soorat's part of mabda's v5.1 retirement gate.
- **The feature backlog from 1.1.0:**
  - GGX-prefiltered environment mips;
  - cascade selection in `pbr.wgsl`;
  - node-centred heatmaps;
  - an SSAO normals pass.
  These are new behaviour, checked against reference tests only (as pravash D5).
- **Downstream unblocks.** tanmatra's and kimiya's soorat integration modules, and a Cyrius jantu, can bind soorat 2.0.
- **`rust-old/` leaves** after 2.0.0, as pravash did. The corpus and records stay frozen, and `validate.py` recomputes every f64-reference row without Rust. Class G's reference renders stay as files.

---

## 14. Tracking

| Milestone | Status | Golden rows | Rust tests ported | Benches | Notes |
|---|---|---|---|---|---|
| D1–D9 | ✅ decided 2026-11-15 | — | — | — | |
| M0 Prepare and initialize | ✅ done 2026-11-15 | 1,473 (all pending until M1); 12 class-G scenes | 0 of 471 | Rust column recorded | steps 0–8; next M1 |
| M1 Foundation | ✅ done 2026-11-15 (parts a–d, b2) | 13,793 passing: math 1,436 (1,149 accepted E); color 3,397; color_optics 1,704 (own harness, `optics`); json 177; lights 1,066 and tables 5,215 (31 accepted cr); lod 770; layout 19; constants 9 of 18. Pending: 9 constants (their modules) and 3 `json` bayan rows (ADR 0002 #19) | 67 of 471 (M1: all 67) | 6 of 35 twinned (`color/*`, `vertex/bytemuck_cast_100`) + 16 extras (`math/*`, `tables/*`, `json/*`, `color_optics/*`) | a: error, log, math. b: mathx, layout, color. c: lights, lod. d: tables (f16, sRGB). b2: `optics` (prakash, `[lib.optics]`) and serde (bayan, `[lib.serde]`), both opt-in. `material` moved to M6; crmath to M5 |
| M2 2D | ✅ done 2026-11-15 (parts a, b) | `sprite` 638, `text` 479, `debug` 151, `debug_collider` 60 (own harness, `physics-debug`), `json` + 117: all passing (2 accepted cr); 1 bayan `json` row pending | M2: all 69 (sprite.rs 15, pipeline.rs's 16 batch tests, text.rs 10, ui.rs 6, debug_draw.rs 15, integration 7) | all 16 twinned (`sprite/*`, `pipeline/batch_to_vertices_*`, `debug_draw/*`, `vertex/quad_generation_100`): 4–11× Rust | a: sprite, batch (ADR 0002 #20). b: text, ui (#21), debug geometry, the impetus collider as the `physics-debug` profile (#22) |
| M3 Geometry and animation | ☐ | | | | |
| M4 Lighting math and infrastructure | ☐ | | | | |
| M5 Science visualization | ☐ | | | | `em` per D7 |
| MB1–MB5 (mabda) | ☐ | — | — | — | in mabda |
| M6 GPU core | ☐ | | | | |
| M7 Shadows, IBL, PBR | ☐ | | | | |
| M8 Post, particles, screenshots | ☐ | | | | |
| M9 Integration | ☐ | | | | |
| M10 Release 2.0.0 | ☐ | | | | |
