# soorat port parity: measured baselines

Measured numbers behind the decisions in [soorat-port-plan.md](soorat-port-plan.md). Every entry names its
files and how to re-run it. This is the same record pravash, kimiya and tanmatra keep. Sections are added as
the port proceeds: M0 probes first, then one section per milestone.

**Environment (2026-11-15).** All probes ran on the dev box:
- CPU: AMD Ryzen 7 5800H (Cezanne), with FMA and AVX2.
- GPU: Radeon Vega, PCI `0x1638`; RADV reports it as RENOIR.
- Software: Linux x86_64, glibc 2.44, rustc 1.100.0, Cyrius 6.7.18.
- Oracle: soorat **1.2.0**, tag `1.2.0` at `6625dc2`, built with the hidden `oracle` feature.

The box is shared, so timing differences under ±20 % are noise.

## 1. Probes (M0 step 1, 2026-11-15, x86_64 Linux ELF)

Each probe is a small program, answered with numbers before M1 writes code.
- **Cyrius:** one small project per probe group under `scripts/probes/<group>/` (`math`, `bundle`, `assets`,
  `gpu`), each with its own `cyrius.cyml`. Run `cyrius deps`, then `cyrius test --bounds tests/<probe>.tcyr`
  in that directory.
- **Rust:** the `scripts/probes/rust/` crate, on soorat 1.2.0 by path with the `oracle` feature. Its
  `Cargo.lock` is seeded from 1.2.0's lock. Run `cargo run --release --bin <probe>` there.
- Outputs go to `scripts/probes/<group>/out/`, which git ignores.

### 1.1 P1: GPU layouts (`p1_layout.rs` → `math/tests/p1_layout.tcyr`)

**Question.** Can Cyrius reproduce every byte layout soorat uploads to the GPU, given that it has no array
fields and no vector-type fields?

**Method.**
- `p1_layout.rs` prints the size and the `offset_of!` of every field of soorat's 15 `#[repr(C)]` GPU types and
  of mabda's `Vertex2D`, `Vertex3D`, `SkinnedVertex3D` and `InstanceData`.
- It parses the nine shaders with naga 29 and compares each Rust uniform or storage type with the WGSL struct
  it is bound as (size against span, member offsets in order), and each vertex type with its `layout()`.
- It then **generates** `math/tests/p1_layout.tcyr`: every type as a packed Cyrius struct built from nested
  `f32` structs (`SrV2`/`SrV3`/`SrV4`/`SrU4`/`SrM4`), with fixed arrays expanded into indexed fields
  (`joints_0` … `joints_127`).
- The test writes a distinct marker into each field's first scalar of a zeroed struct and finds it in the
  bytes. Cyrius refuses `&s.field`, so offsets are measured this way; `p0_offsets.tcyr` checks the method.

**Result.**

| Check | Outcome |
|---|---|
| Rust vs WGSL, the 12 bound types | identical size/span and member offsets: `GpuParticle` 48 (the storage-buffer element), `SimParams` 16, `PostProcessUniforms` 16, `JointUniforms` 8,208, `BloomUniforms` 32, `GpuLight` 64, `LightArrayUniforms` 544, `MaterialUniforms` 32, `SsaoUniforms` 144, `CameraUniforms` 192, `ShadowPassUniforms` 80 (as `pbr.wgsl::ShadowUniforms`), `ShadowUniforms` 128 (as `shadow.wgsl::ShadowUniforms`) |
| Rust vs `layout()`, 4 vertex types | identical strides and attribute offsets: `Vertex2D` 32, `Vertex3D` 48, `SkinnedVertex3D` 96, `LineVertex` 28 |
| Types with no WGSL consumer in 1.2.0 | `LightUniforms` (112 B, no shader binds it) and `CascadeUniforms` (272 B, waiting on cascade selection). Port them as data |
| Cyrius mirrors, 19 types | **217 / 217** size and offset checks pass under `cyrius test --bounds` with no warnings. This includes `JointUniforms` (8,208 B: 128 indexed `SrM4` fields), `LightArrayUniforms` (8 indexed `SrGpuLight`s) and `CascadeUniforms` (4 indexed matrices) |

**Language notes (rules for M1).**
- `&s.field` is refused ("the address of a field is not taken").
- An uninitialized struct local is allowed only inside a function.
- Every `struct` must come before the first top-level statement.
- Array fields are refused. A fixed array becomes indexed fields, generated, never retyped. Indexing at run
  time goes through a typed pointer: a `*SrM4` steps 64 bytes.

**Decision.** P1 holds. Packed nested `f32` structs reproduce every soorat GPU layout exactly. In M1,
`src/layout.cyr` is generated from the oracle by this probe's generator, and the layout rows are class A.

### 1.2 P2: f32 traps (`p2_math.rs`, `math/tests/p2_math.tcyr`, `math/p2_compare.py`)

**Question.** Does a hand port of soorat's f32 math reproduce Rust's bits under D4 rule a (Rust's f32
sequence), and do the language's f32 traps stay visible?

**Method.**
- `math/tests/p2_math.tcyr` ports all of `math_util` to Cyrius f32: `mul_mat4`, `normalize3`, `cross`,
  `perspective_90`, `look_at`, `compose_trs` and `normal_to_basis`.
- Both sides print each result's f32 bits for the same inputs. The inputs come from one integer xorshift64
  implemented identically on both sides and turned into floats by exact conversions, so they are bit-identical
  without a data file.
- The inputs include the edge cases: the zero vector, 1e-12, 1e30, NaN, `dir ∥ up`, a zero direction, and
  `near == far`.
- Run: `cargo run --release --bin p2_math > ../math/out/p2_rust.txt`, then in `math/`
  `cyrius test --bounds tests/p2_math.tcyr > out/p2_cyrius.txt` and
  `python3 p2_compare.py out/p2_rust.txt out/p2_cyrius.txt`.

**Result.** All **1,416 cases (15,318 f32 values) are bit-identical**, including the 8 NaN outputs and their
payloads. The test compiles under `--bounds` with no warnings. The idioms that make this work are rules for
the real port:

| Trap | What the probe does |
|---|---|
| An operator is typed by its left operand | Every f32 operation goes through a helper with `: f32` parameters (`fa`, `fs`, `fm`, `fd`, `flt`), as ranga's `composite.cyr` does |
| `fn f(): f32` is refused | Helpers return the bits in an i64. Arrays go out through a caller buffer (`store32`) |
| f32 literals | `1e-10f32`, `f32::EPSILON` and `0.99f32` are written as their bits from rustc (`0x2EDBE6FF`, `0x34000000`, `0x3F7D70A4`), because a decimal through `f32_from` rounds twice |
| Negation | `-x` flips the sign bit (`x ^ 0x80000000`); `0 - x` would be an integer subtract |
| `sqrt` | `f32_from(f64_sqrt(f32_to(x)))` is the correctly rounded f32 square root, identical to Rust's |
| `f32::max` | `ganita_f32_max` is IEEE maxNum, as Rust's is |

**D4 rule b, measured.** `p2_math --accuracy` compares, in Rust, each f32 result with the same formula
evaluated in f64 and rounded once to f32. That rounded f64 value is what the Cyrius f64 path would produce.
Each matrix row has 20,000 random cases:

| Output | Values | f32 path = rounded f64 | ≤ 1 ulp | > 16 ulp | Worst |
|---|---|---|---|---|---|
| `mul_mat4`, one product | 320,000 | 51.1 % | 86.7 % | 1.1 % | 33,795 ulp |
| `mul_mat4`, a chain of three (an MVP or a joint chain) | 320,000 | 36.9 % | 75.0 % | 2.1 % | 111,421 ulp |
| `look_at`, rotation part | 260,000 | 67.0 % | 93.7 % | 0.4 % | 674,066 ulp |
| `look_at`, translation column (dot products) | 60,000 | 38.8 % | 78.5 % | 1.6 % | 42,517 ulp |
| `compose_trs` | 320,000 | 77.6 % | 97.4 % | 0.2 % | 8,260 ulp |
| `normalize3` | 60,000 | 68.0 % | 99.8 % | 0 | 2 ulp |

The large worst cases are cancellation: a sum of products of opposite sign, or the cross product of nearly
parallel vectors. f64 carries 29 more bits through them, so the f64 path's error is ≤ 0.5 ulp at every one of
these outputs.

**Decision.**
- D4 rule a is exact: class B rows are expected at 0 ulp, as here.
- D4 rule b's gain is real and large where it matters. Against the f64 reference, the f32 path is correctly
  rounded on only 37–78 % of matrix outputs, with a tail of thousands of ulp.
- §6.2's rows for the matrix builders and joint chains stand, now with numbers.

### 1.3 P3: f32 transcendentals (`p3_trans.rs`, `math/tests/p3_trans.tcyr`, `math/p3_compare.py`)

**Question.** How do ganita's f32 functions compare with Rust's (host glibc) on the arguments soorat actually
passes? (D4, §6.2, class C.)

**Method.**
- `p3_trans.rs` generates **226,050** arguments the way soorat 1.2.0 computes them, in f32:
  - sphere and balloon lattices `(i/n)·π` and `(j/m)·τ`, and fixed-step rings `(τ/n)·i`, for n = 3…64, 96,
    128, 256, 512 and 1,024;
  - 20,000 sprite rotations;
  - slerp's `acos(cos)` up to the 0.9995 switch, then `sin θ` and `sin(tθ)`;
  - sRGB decode `((c + 0.055)/1.055)^2.4`, every byte above the knee plus a dense grid;
  - sRGB encode `l^(1/2.4)`;
  - cascade splits `(far/near)^ratio`;
  - the coupled decay's `exp(−t/τ)`, `log10(e)` and `ln 10`.
- Rust's value is std's f32 function, i.e. glibc 2.44. The reference is the same function in f64, rounded
  once to f32.
- Cyrius uses ganita's f32 tier: `ganita_f32_sin`, `cos`, `pow`, `exp` and `ln`. ganita has **no f32
  `acos`** and the stdlib has **no `log10` at all**, so those two go through f64: `ganita_f64_acos`, and
  `ln(x)/ln 10` in f64, each narrowed once.
- Run: `cargo run --release --bin p3_trans`, then in `math/`
  `cyrius test --bounds tests/p3_trans.tcyr > out/p3_cyrius.txt` and `python3 p3_compare.py`.

**Result.**

| Inputs | fn | n | Rust = ref | Cyrius = ref | Rust = Cyrius | Worst Rust/ref |
|---|---|---|---|---|---|---|
| sphere / balloon lattice | sin, cos | 8,320 each | 98.750 % | **100 %** | 98.750 % | 1 ulp |
| fixed-step rings | sin / cos | 4,160 each | 98.942 / 98.846 % | **100 %** | same | 1 ulp |
| sprite rotations | sin / cos | 20,000 each | 98.640 / 98.720 % | **100 %** | same | 1 ulp |
| slerp | sin | 60,003 | 98.703 % | **100 %** | 98.703 % | 1 ulp |
| slerp | acos | 20,001 | 100 % | 100 % | 100 % | 0 |
| sRGB decode | pow | 20,245 | 99.921 % | **100 %** | 99.921 % | 1 ulp |
| sRGB encode | pow | 20,000 | 99.950 % | **100 %** | 99.950 % | 1 ulp |
| cascade splits | pow | 840 | 99.405 % | **100 %** | 99.405 % | 1 ulp |
| coupled decay | exp | 20,000 | 99.940 % | **100 %** | 99.940 % | 1 ulp |
| coupled decay | log10, ln | 20,001 | 100 % | 100 % | 100 % | 0 |
| **all** | | **226,050** | **99.271 %** | **100 %** | **99.271 %** | **1 ulp** |

- glibc 2.44's f32 `sinf`, `cosf`, `powf` and `expf` misround about **1 in 75** of soorat's `sin`/`cos`
  arguments, always by exactly 1 ulp.
- ganita's f32 tier, which widens to f64 and narrows once, gives the reference value on **every** one of the
  226,050.
- So Cyrius and Rust differ on **0.729 %** of these rows, by 1 ulp, and in each the Cyrius value is the
  reference value.
- The reference comes from glibc's f64 functions; it agrees with Cyrius's independent f64 implementation on
  every row.

**Effect on quantized outputs (D4 rule a).** All 245 sRGB-decode table bytes above the knee get identical
`powf` results, so the table matches either way (it is generated from the oracle regardless). Across the
20,000 encode cases, the 1-ulp `powf` differences flip **no** byte of `round((1.055·p − 0.055)·255)`.

**Further findings.**
- `ganita_f32_round` rounds ties to even, while Rust's `f32::round` rounds half away from zero. soorat's 8
  `* 255.0` quantization sites need `sr_round` in `mathx`, as the plan already says.
- The `log10` stand-in (`ln/ln 10` in f64) matched the reference on all 20,000 decay rows, but it is not
  correctly rounded by construction. kimiya's correctly rounded `crlog` is the stand-in to vendor (ledger L1).

**Decision.**
- Class C holds with K = 0 against the reference rows (`*.cr.tsv`). Against Rust, up to 1 ulp on about 0.7 %
  of rows; the harness lists those rows as accepted, by name (kimiya's practice).
- §6.2's "f32 transcendentals kept in f32" row is now measured: 0.73 % differ, all by 1 ulp, and Cyrius is
  correct on all of them. That is better than the plan's ranga-based estimate (about 1 %, ≤ 2 ulp).

### 1.4 P4, P5 and P11: toolchain and the full bundle (`scripts/probes/bundle/`)

Each probe directory is its own Cyrius project. `bundle/run.sh [p4|p5|p11]` reproduces every number here and
writes its logs to `bundle/out/`; `names.py` is the static cross-check.

#### P4: one hisab

goonj 2.3.2 pins hisab 3.5.0; prakash, ushma, pravash and impetus pin 3.4.1. Three builds put all five
bundles in one compile:

| Root hisab | Outcome |
|---|---|
| 3.5.0 | **Builds, 9 / 9 checks pass** (goonj's BVH included). No hisab diagnostic |
| 3.4.1 | `undefined function 'hisab_alloc_scope'`, which goonj calls in `_nearest_wall_bvh` (`goonj.cyr:5950`, `5954`). Once that path is reachable: `refusing to emit binary with 1 reachable undefined function(s)`. The function arrived in hisab **3.4.2**, so goonj needs ≥ 3.4.2 |
| none, as D9 says | Resolves the same as 3.5.0 (`note: hisab 3.4.1 (wanted by ushma) not used; 3.5.0 (goonj) resolved first`). But **the version follows `[deps]` order**: with ushma declared before goonj, a dry run resolves 3.4.1 |

- **Static scan.** hisab 3.5.0 removed 16 public functions and 13 private helpers. None of ushma, prakash,
  pravash and its `compute_ad.cyr`, or impetus and impetus-3d calls any of them, or the other APIs 3.5.0
  changed.
- **Scope.** prakash, pravash and impetus declare hisab with `scope = "test"`, so their pins never reach
  soorat. ushma is the only production 3.4.1 consumer, and it calls just `calc_lerp` and
  `calc_integral_simpson`.

**Decision.** D9's conclusion holds: hisab 3.5.0 serves all five. L11 drops from a possible blocker to
tidy-up. Because the version otherwise follows declaration order, §4 step 3 declares goonj before ushma, or
soorat pins 3.5.0 itself; see the plan's D9 note.

#### P5: mabda 4.2.0 on cyrius 6.7.18

The stdlib fold and tag 4.2.0's `dist/mabda.cyr` are **the same file**: sha256 `5cd9f695…`, 1,370,812 B,
not the ≈ 1.2 MB the plan and mabda's README say. A minimal consumer (the README's `mabda_main`, reachable)
was built both ways under `cyrius test --bounds`:

| Route | Warnings | Errors | Compile | Binary | Preprocessed |
|---|---|---|---|---|---|
| stdlib fold (quick-start include order) | **0** | 0 | 0.43–0.54 s | 722,704 B | ≈ 1.97 MB |
| `[deps.mabda]` at tag 4.2.0 | **0** | 0 | 1.12–1.32 s | 1,515,280 B | ≈ 3.45 MB |

Preprocessed size is measured by bisection: pad the source until the compile fails with `expanded source
exceeds 24MB`.

As a dependency, mabda also pulls **samvada 1.0.1 and chitra 1.0.3** into every consumer's compile, because
its manifest declares them as production dependencies. The README describes PNG/JPEG and logind as opt-in
`-D` features.

**Decision.** mabda's 6.7.5 pin doesn't matter on 6.7.18 (L12 is tidy-up). File alongside it a request to
make mabda's `[deps.chitra]` and `[deps.samvada]` optional.

#### P11: the full bundle

The bundle is everything soorat 2.0's `full` would pull, as §4 step 3 declares it with no root hisab:
- mabda 4.2.0, chitra 1.4.1, prakash 2.15.2, ranga 2.4.0, impetus 2.0.0, pravash 2.0.1, goonj 2.3.2,
  ushma 2.0.0, selah 1.0.1 and agnodrm 1.8.1;
- resolved alongside: samvada 1.0.1, sankoch 2.8.9 and hisab 3.5.0;
- bayan, sakshi and sandhi from the stdlib.

**As declared, it does not compile: 8 duplicate-function warnings and 2 errors.**

| Clash | Names | Severity |
|---|---|---|
| **hisab × ranga** | `premultiply_alpha`, `unpremultiply_alpha`: **error**, "disagrees about arity: this one takes 4, the one in lib/ranga.cyr takes 1". `srgb_to_linear`, `linear_to_srgb`: warning, last definition wins | **blocks any build with ranga plus goonj or ushma**. hisab 3.4.1 defines the same names, so this doesn't depend on P4 |
| agnodrm × sigil (sigil arrives through sandhi, which selah needs) | `result_print_err`, `is_syscall_err`, `wrap_syscall` | warnings; last definition wins |
| goonj × pravash | `logging_init` | warning; last definition wins |

**A silent wrong answer.** With ranga's two arity clashes renamed in a generated copy (`patched/`, a
measurement aid only), the rest builds and every bundle answers. But hisab's `srgb_to_linear` wins over
ranga's:
- `srgba_to_lin_rgba(128).r` is **`0xa`**, where ranga's own kernel gives `0x3e5d0a8b`;
- the round trip gives 129 instead of 128;
- the ranga-only control gives `0x3e5d0a8b` and 128.

A static scan finds **31 more** names defined twice that the compiler doesn't report because their values
agree:
- agnodrm and sigil both define the `Errno` and `SysErrorKind` enums and the errno members;
- mabda's variable and agnodrm's enum both define `DRM_IOCTL_VERSION`, both `0xC0406400`.

**Size and time** (the patched build):
- preprocessed **≈ 17.62 MB**, 70 % of the 24 MiB cap, leaving ≈ 7.5 MB;
- 9.5–9.8 s per compile;
- binary 9,692,688 B, and **2,582,032 B with `CYRIUS_DCE=1` (−73 %)**;
- `large static data (410280 bytes)` advisory: the whole program's total, no single bundle.

**Names.**
- No top-level `sr_*`, `Sr*` or `SR_*` name exists in any of the 93 vendored files; the only match is a local
  `var sr_tag` in sandhi.
- goonj defines **14 `soorat_*` functions** (`soorat_ray_visualization_*`, `soorat_pressure_map_*`,
  `soorat_mode_visualization_*`); ushma's `soorat.cyr` uses `Us*` names.

**Decision.**
- **P11 fails**, on dependency name clashes the plan didn't foresee. No local stand-in can fix a dependency's
  names, so each clash needs a request in the dependency that should rename:
  - hisab or ranga (L18), which blocks the `ranga` feature together with `acoustics` or `thermo`. **Filed 2026-11-15** in hisab (primary: rename its four colour functions) and ranga (companion);
  - agnodrm or sigil (L19);
  - goonj or pravash (L20).
- §12's 24 MB risk is retired.
- The planned prefixes `Sr`, `sr_` and `SR_` are free.
- agnodrm belongs behind the `drm` feature only (Rust's `full` excludes `drm`), so the agnodrm × sigil clash
  bites only with `drm` and `screenshot` together.

### 1.7 What the probes changed

| Probe | Verdict | Plan change |
|---|---|---|
| P1 layouts | ✅ 217 / 217 | `src/layout.cyr` generated from the oracle |
| P2 f32 traps | ✅ 15,318 values bit-identical | the f32 helper idiom is a rule; D4(b)'s gain measured |
| P3 transcendentals | ✅ Cyrius correctly rounded on all 226,050 | class C: ≤ 1 ulp against Rust on 0.73 %, 0 against the reference |
| P4 one hisab | ✅ 3.5.0 | L11 is tidy-up; D9 note on `[deps]` order |
| P5 mabda on 6.7.18 | ✅ 0 warnings | L12 is tidy-up; mabda's chitra and samvada should be optional |
| P6 WGSL via mabda | ⚠ 9 / 9 modules, 2 / 9 pipelines | MB scope confirmed and extended; MB4 is missing; filed in mabda |
| P7 uploads | ✅ 5.7–8.5× Rust | generate vertices in place; L13 matters only for readback |
| P8 frame arena | ✅ flat; ⚠ mabda's loop leaks 1.6 KB a frame | L17, filed in mabda |
| P9 GLB via bayan | ✅ 42 / 51 identical | D6 widened; ADR 0002 #14 |
| P10 chitra | ✅ PNG/BMP exact, JPEG within B-img | ADR 0002 #15; no chitra accuracy request |
| P11 full bundle | ❌ 2 errors, 8 warnings, a silent wrong sRGB | L18–L20 (name clashes); §12's 24 MB risk retired |

### 1.5 P6, P7 and P8: the GPU path (`scripts/probes/gpu/`)

**Setup.**
- Device: vendor `0x1002`, device `0x1638`, "AMD Radeon Graphics (RADV RENOIR)", Mesa 26.2.6.
- Stack: wgpu-native v29.0.1.1 under mabda 4.2.0, the folded stdlib bundle.
- Programs are built as Cyrius objects and linked against mabda's own launcher, used in place. `build.sh`:
  ```
  gcc -c …/mabda/deps/wgpu_main.c -I…/deps/wgpu-native/include -Wall -Wextra -Werror -o build/wgpu_main.o
  printf 'object;\n' | cat - src/X.cyr | cycc > build/X.o
  objcopy -L memcpy -L memset -L memchr -L strlen -L strchr -L strstr -L memeq -L atoi -L print_num -L println build/X.o
  gcc build/wgpu_main.o build/X.o …/deps/wgpu-native/lib/libwgpu_native.a -lpthread -ldl -lm -o build/X
  ```
- Run: `sh build.sh && sh run_p6.sh`, `cyrius bench tests/p7_upload.bcyr`, `./build/p7_upload`, `sh run_p8.sh`,
  `sh run_p8_gpu.sh`. Logs are in `build/logs/`, which is tracked.
- Neither the Cyrius sources nor the launcher compile produced any warnings.
- mabda printed its deprecation notice once (strict mode off, and the context was still created):
  `mabda: DEPRECATION - AMD-on-wgpu is deprecated (v4.0.1); use BACKEND_KIND_AMD native.`

#### P6: soorat's WGSL through mabda (`src/p6_wgsl.cyr`)

| Check | Outcome |
|---|---|
| `gpu_shader_module_create`, all nine shaders | **9 / 9 created**: sprite 269 µs, line 66, pbr 862, pbr_skinned 874, shadow 56, ssao 300, bloom 273, postprocess 227, gpu_particles 123. A deliberately broken shader is rejected (`Entry point vs_main at Vertex is invalid`), so validation runs |
| Validation errors | **abort the process**: wgpu-native's default uncaptured-error handler panics, exit 134. There is no error callback in the launcher and no error scope in the public tier, so each failing case ran in its own process |
| `gpu_render_pipeline_create(ctx, m, m, fmt)` | builds only for **ssao** and **postprocess**, the two full-screen shaders. sprite, line, pbr, pbr_skinned and shadow fail with `Location[0] … is not provided by the previous stage outputs` (no vertex buffers). bloom fails with `Unable to find entry point 'fs_main'`, and gpu_particles with `Unable to find entry point 'vs_main'` |
| `rpb_new` + `rpb_vertex_layout(vertex2d_attributes())` + `rpb_build` | the same `Location[0]` failure: **the builder ignores the layout**, as §2.1 read from the code |
| Drawing with the postprocess pipeline | with no bind group: `expects a BindGroup to be set at index 0`. With `gpu_render_pass_bind_texture`: `Number of bindings in bind group descriptor (2) does not match … layout (3)`, because the uniform can't be bound |
| `gpu_compute_dispatch` | returns 99 (`GPU_ERR_OTHER`), as §2.1 says |
| `compute_pipeline_new(device, src, "cs_update", 2)` | **fails**: binding 1 is created as storage, the shader declares a uniform. The older compute path builds storage-only layouts, so it can't build soorat's particle pipeline at all |

**Decision.** §2.1's table holds except for compute, and MB grows by three items:
- **MB4 is "missing", not "partial".** `compute_pipeline_new` can't make a storage + uniform layout.
- **Entry-point names in the pipeline descriptor (MB1).** bloom has three fragment entry points and no
  `fs_main`, postprocess uses `fs_no_bloom`, and shadow has no fragment stage.
- **An error channel (MB1).** An error scope or an uncaptured-error callback, so an invalid shader or pipeline
  returns an error instead of aborting the host application.
- Also, public calls take constants defined only in `@internal` files: buffer usage `WGPU_BUFFER_USAGE_*`,
  `gpu_sampler_create`'s `NATIVE_SAMP_*`, and the pipeline format as a raw `WGPUTextureFormat`. The render
  tier should give them public names.

#### P7: upload cost (`tests/p7_upload.bcyr`, `src/p7_upload.cyr`)

**CPU, two runs.** Averages in µs, with the minimum in brackets. The generator was checked against
hand-computed corners, rotated sprites included.

| Row | Cyrius | Rust 1.2.0 | Cyrius / Rust |
|---|---|---|---|
| `memcpy`, 131,072 B (byte loop) | 211–216 (201) | — | — |
| word-copy loop, 131,072 B | 27.7 (26.0) | — | 7.6× faster than `memcpy` |
| `memset`, 131,072 B | 185–193 | — | — |
| `batch_to_vertices_into_1000` (1,000 sprites into a caller buffer) | 67.6–68.0 (65.5) | 11.97 | **5.7×** |
| the arena-allocating form | 67.0–68.0 | 11.90 | 5.7× |
| `rotated_100` (f64 sin/cos) | 14.9–15.0 | 1.75 | **8.5×** |
| generate, then `memcpy` | 275–276 | — | — |

**GPU, 2,000 frames.**
- `gpu_buffer_write` × 2 (140,000 B) costs a median **9.0 µs**. It hands the pointer straight to
  `wgpuQueueWriteBuffer`, so no Cyrius `memcpy` is on the upload path.
- A whole frame takes a median 172.1 µs with the upload and 143.0 µs without it.
- Reading back matches byte for byte, but `gpu_buffer_read(128000)` takes **2.7 ms**, because it ends in the
  byte-loop `memcpy`.

**Decision.**
- L13 (the byte-loop `memcpy`) doesn't matter on the upload path if vertices are generated straight into the
  upload buffer, which becomes the rule. A stray 128 KB copy would cost about 210 µs, three times the
  generation itself.
- L13 does matter for readback (screenshots, tests): 2.7 ms per 128 KB.
- The loop ratios (5.7× and 8.5×) fall inside §11's 4–10× estimate.

#### P8: frame arena (`src/p8_arena.cyr`, `src/p8_gpu_frame.cyr`)

Each frame builds 232,000 B: 1,000 sprites (64,000 B), their `Vertex2D`s (128,000 B), u16 indices
(12,000 B) and 500 debug lines (28,000 B of `LineVertex`).

| Allocator, 100,000 frames | RSS at frames 1 … 100,000 | Mean frame |
|---|---|---|
| fixed arena (`arena_new(262144)`, reset every frame) | **2,289,664 B at every checkpoint**; `alloc_used` stays 262,216 | 88.8 µs |
| growable arena (starts at 64 KiB) | flat at 2,289,664 B; the chunk chain settles at 259,072 B | — |
| freelist | flat at 425,984 B | 206.6 µs (each block over 4 KiB is an mmap/munmap) |
| global `alloc`, no reset (10,000 frames) | 2.29 MB → 25.4 MB → 233 MB → **2,321,735,680 B**: exactly 232,000 B per frame | 96.6 µs |

The roughly 2 MiB jump at frame 1 is one transparent huge page (THP is `always`).

**A real render loop through mabda is not flat.** `p8_gpu_frame` ran 100,000 frames of arena batches, three
buffer writes, a pass, a textured draw and a wait. RSS went from 96.2 MB to **260.3 MB**, about 1,641 B per
frame:
- **656 B per frame from the Cyrius heap.** `gpu_render_pass_begin` bump-allocates 432 B per call and a
  textured `gpu_render_pass_draw` 224 B. Every other per-frame call allocates 0.
- **About 985 B per frame outside it.** mabda never releases a submitted `WGPUCommandBuffer`, and the
  launcher's 67 function-table slots have no `wgpuCommandBufferRelease`.
- A patched copy of the launcher, built only under `build/`, that releases each command buffer after submit
  brings growth down to the heap rate. That confirms the cause.
- Loops that only wait, or do only CPU work, stay flat at 96.1 MB.
- At 60 fps the two leaks come to about 350 MB per hour (an inference from the per-frame rate).

**Decision.**
- §9.4's frame arena holds: flat and the fastest option.
- A mabda **leak fix joins the render tier**:
  - add a `CommandBufferRelease` launcher slot, released after each submit;
  - stop the per-call bump allocations in pass begin and draw.
  
  This goes into the same single launcher change as MB1–MB5 (§8).

**Filed (2026-11-15).** Both are in mabda's tree:
- `docs/development/issues/2026-11-15-wgpu-render-loop-leaks-command-buffers-and-pass-records.md` (ledger L17);
- `docs/development/issues/2026-11-15-render-tier-for-geometry-consumers.md` (L2–L6, with P6's additions).

### 1.6 P9 and P10: GLB and image decoders (`scripts/probes/assets/`)

Run everything with `scripts/probes/assets/run.sh`. It regenerates the fixtures and the dumps, then compares.
- Rust side: `rust/src/bin/p9_glb.rs`, `p10_codecs.rs` and `p10_compare.rs`.
- Cyrius side: `assets/tests/p9_glb.tcyr`, `p9_mem.tcyr` and `p10_decode.tcyr`, on stdlib bayan 1.11.1 and
  chitra at tag 1.4.1.

#### P9: GLB through bayan

**Method.**
- `p9_glb.rs` copies soorat's `test_support` GLB builders verbatim and adds three more inputs:
  - `rich.glb`: every number form, escape and accessor type the loader reads;
  - `float_stress.glb`: 400 joints carrying 4,000 numbers;
  - **47 malformed or edge variants**.
- It dumps what soorat 1.2.0's `load_gltf_meshes` and `load_gltf_animations` make of each fixture (f32s as
  hex).
- `p9_glb.tcyr` is a loader prototype (885 code lines) on bayan's JSON tree and `load8/16/32`, writing the
  same dump format.
- Rust is compared in shipped release semantics, without overflow checks. The probe crate's own build has
  them on, and is reported where it differs.

**Result.**

| Check | Outcome |
|---|---|
| Crashes, 51 fixtures under `--bounds` | **none** |
| Dumps identical to Rust 1.2.0 | **42 / 51** (40 / 51 against the overflow-checks build) |
| What bayan carries (`rich.glb`, byte for byte) | integer and float tags; exponent forms (`1E0`, `2E+2`, `1.5E-3`); `-0.0`; nested arrays and objects; escapes including a surrogate pair; u8/u16/u32 indices; normalized u8/u16 UVs; i8/u8/i16/u16 rotations; interleaved strides; inverse bind matrices; STEP, LINEAR and CUBICSPLINE; skipped `weights` channels |
| Float vs integer | serde refuses `"count": 3.0` and negative counts. `bayan_json_v_int` silently coerces a float, so the loader must check the tag itself. The prototype does, and its verdicts match |
| JSON → f32, 4,000 numbers | bayan + `f32_from` gives the correctly rounded f64, narrowed, on **4,000 / 4,000**; Rust (serde_json) on 3,959. All 41 misses are among the 2,000 adversarial 20-digit decimals near f32 midpoints, none among the 2,000 shortest round-trip strings. Against the exactly rounded f32: Cyrius 3,947, Rust 3,916 |
| Memory | Through the global bump, never freed: 40,440 B per parse of the 4,780-byte rich JSON, 560,968 B for the 125,560-byte stress JSON. Through `arena_allocator_growable` + `reset_via`: **0 B growth over 1,000 parses**; the stress file chains once to 589,824 B, then stays flat |
| Parse time (wall clock, noisy) | about 0.2–0.7 ms for rich, 5–13 ms for stress |

**The 9 that differ:**
- **bayan refuses, Rust accepts (4):**
  - a duplicate key inside `extras`, and a duplicate unknown key (bayan issue 2026-11-09, open);
  - an integer above i64::MAX in `extras` (bayan issue 2026-11-09, open);
  - objects nested 200 deep in `extras`: bayan caps nesting at 128, while serde skips unknown content at any
    depth. This one is not filed.
- **Rust 1.2.0 panics, so there is no verdict to copy (4):**
  - a POSITION view with `byteStride: 4` (an assert in gltf's `from_slice`);
  - POSITION pointing at accessor 99 (the validation hook indexes `accessors[99]`);
  - a float index componentType (`unreachable!`);
  - an unknown animation target path (`validate()` passes it, then `property()` unwraps it).

  The Cyrius prototype returns an error for each.
- **`float_stress`:** the 41 f32 rows above, where Cyrius is the correctly rounded one.

With overflow checks on, two more inputs panic in Rust: a GLB header length below 12, and an accessor count of
0. Release builds give an error and a skipped primitive instead, which the prototype matches.

**Contradictions with the plan.**
- D6 says "GLB plus embedded buffers", but 1.2.0 also accepts **JSON-text `.gltf`**: `plain.gltf` loads with
  0 meshes and 2 empty images.
- The `gltf` crate also reads **sparse accessors** (seen in its source; not exercised by these fixtures).
- §5 makes glTF accept/reject verdicts class A. That needs ADR 0002 rows for the 6 inputs that panic Rust, and
  for document refusals D6 doesn't list:
  - POSITION without min/max;
  - non-object `extras`;
  - an unknown semantic;
  - any `extensionsRequired`;
  - an invalid interpolation;
  - a primitive mode above 6.
- Joint TRS read from JSON is not class A on adversarial decimals, though it is on shortest round-trip
  output, which is what exporters write.

**Estimate.** A full `src/gltf.cyr` is about 1,100–1,400 lines (an inference): the prototype, plus matrix-joint
decomposition, sparse accessors, the material, texture and camera schema checks, and the Result and struct
output. The Rust side is about 400 lines on top of the `gltf` crate.

**Decision.**
- **D6 holds**: bayan carries everything soorat reads.
- **Widen D6** to JSON-text `.gltf` and sparse accessors.
- ADR 0002 gets these glTF rows:
  - the 4 bayan refusals (the two open bayan issues get soorat as a consumer; the nesting cap is new);
  - the 6 inputs where Rust panics or refuses outside D6's list, which become `SR_ERR_*` in Cyrius;
  - the f32 rows, where Cyrius is correctly rounded.
- The loader parses through a reset arena, not the global heap.

#### P10: chitra vs image

**Fixtures (88).**
- Sizes: 61×37 (gradient, edges, noise), 64×48 (gradient) and 1024² (a photo-like image), each with a
  ground-truth `.source.rgba`.
- `image` can't write palette, sub-byte or Adam7 PNGs. Four are built by hand with stored deflate:
  - 8-bit palette with a tRNS chunk covering 200 of 256 entries;
  - 4-bit palette;
  - 1-bit gray;
  - Adam7 RGBA8 with filters cycled.
- `image`'s JPEG encoder writes **baseline 4:4:4** (h = v = 1 in SOF0), although its doc comment says 4:2:2,
  and takes only L8 and Rgb8. The subsampled 4:2:0 and 4:2:2 q90 files, each with a 180°-rotated partner, come
  from the system `cjpeg` (libjpeg-turbo 3.2.1).

**Result.**
- **PNG and BMP: 49 / 49 bit-identical** between chitra and `image`, and both equal the ground truth. This
  covers 16-bit narrowing, a short tRNS, 32-bit V4-alpha BMP, 8-bit gray BMP and Adam7.
- **JPEG 4:4:4** (q90, q75, q50, gray q90; 19 files): selah's B-img bar passes on **19 / 19**. The worst mean
  excess over `image` is +0.0031, and the worst max excess 0. RGB mean / max error against the source:

| File | image | chitra |
|---|---|---|
| grad 61×37, q90 | 0.744 / 4 | 0.744 / 4 |
| edges, q90 | 5.807 / 37 | 5.799 / 37 |
| noise, q90 | 6.301 / 44 | 6.303 / 44 |
| photo 1024², q90 | 1.681 / 10 | 1.681 / 10 |

- Chitra against `image` directly: 0–7.71 % of pixels differ.
- On 4 files the max |Δ| is 3, so selah's conformance bound K_jpeg = 2 would fail them, e.g. edges q90, pixel
  (56, 31), R channel: 215 vs 218, source 230.
- **Subsampled (10 files): 10 / 10 pass** selah's 180° pair rule. The pair-mean excess ranges from −0.071
  (chitra better, gradient 4:2:0) to +0.015, and chitra against `image` reaches max |Δ| 4.

**Speed and memory.** Wall clock, minimum of 21 runs:

| 1024² decode | image | chitra | ratio |
|---|---|---|---|
| PNG RGBA8 (1.24 MB) | 7.7–10.3 ms | 103–108 ms | about 11–13× |
| JPEG q90 4:4:4 | 4.4–5.0 ms | 39.5–40.1 ms | about 8–9× |

- `chitra_image_decode` grew the global bump by **315 MiB over 42 decodes** of 1024², never freed.
- It also caps images at 16,777,216 pixels (read from the source, not run), where `image` allows up to 512 MiB.
- So the port decodes through `chitra_image_decode_budget_via` with a freeing allocator, as selah does
  (inferred from selah's use).

**Decision.**
- **§6.2's rows hold.** PNG and BMP textures are class A.
- JPEG textures are B-img: mean ≤ Rust + 0.05, max ≤ Rust + 2. **K_jpeg = 2 is not a gate**, because chitra
  and `image` differ by up to 3 on sharp edges, symmetrically against the source.
- No chitra accuracy request is needed. Decode speed is already chitra's open issue of 2026-11-14.
- A texture's decode memory goes through a freeing allocator.
- The 16.7 Mpx cap is an ADR 0002 row: soorat 1.2.0 allows larger images up to the device limit.

## 2. M1: foundation (parts a–d and b2, 2026-11-15; M1 complete)

Same dev box and toolchain as §1. Re-run: `cyrius test --bounds tests/parity.tcyr` (the replay),
`python3 -I -B scripts/oracle/validate.py .` (the independent re-derivation), `taskset -c 2 cyrius bench`.

### 2.1 Parity

| Module | Rows | Classes | Result |
|---|---|---|---|
| `math` | 1,436 | E (7 functions), A (`flatten_mat4`) | 287 identical to Rust; **1,149 accepted (E)**, each bit-identical to Rust's formula in f64 rounded once |
| `color` | 3,397 | A, B | all identical to Rust, including every −0 output. Re-run with class B held to exact bits: still all identical |
| `lights` | 1,066 | A; C (`GpuLight::spot`, bound 0) | 1,038 identical; the **28 spot rows where glibc's `cosf` misrounds** (28 of 2,016 cosines, 1.4 %) give the correctly rounded value, never glibc's |
| `lod` | 770 | A | all identical, thresholds ± 1 ulp and NaN included |
| `color_optics` | 1,704 | A | all identical, replayed by tests/parity_features.tcyr (the `optics` feature). Rust calls prakash 1.2.0 (glibc `powf`, `ln`); the port calls prakash 2.15.2: after narrowing to f32 the two agree on all 511 temperatures and 948 wavelengths |
| `json` | 177 (+ 3 pending) | A | every serde_json document of a colour grid, and 25 hand-written ones (field order, array form, extra, missing and duplicated fields, null, strings, signed zeros, exponents, f32 and f64 overflow, trailing text), decode to Rust's bits or are refused as serde_json refuses them. Three are rows of their own, pending: the integer `-0`, an integer above `i64::MAX`, a duplicated unknown field (bayan's open issues; ADR 0002 #19) |
| `tables` | 5,215 | A (f16, the decode table); C (sRGB, bound 0) | f16 ⇄ f32 identical on every subnormal half, every exponent's mantissa edges and every rounding midpoint ± 1 ulp. sRGB: 5,105 identical; the **3 rows where glibc's `powf` misrounds** (0.06 %, each by 1 ulp, as P3 found) give the correctly rounded value. The generated 256-entry decode table equals Rust's, and `validate.py` gets the same 256 with `pow` correctly rounded |
| `layout` | 19 | A | all identical, measured on the compiled structs (`tests/layout_probe.cyr`) |
| `constants` | 9 of 18 | A | identical; the other 9 arrive with their modules |

**Class E, by function** (`validate.py`): f64-once differs from Rust's f32 on `mul_mat4` and `compose_trs` 100 %,
`look_at` 98.5 %, `normal_to_basis` 83.0 %, `cross` 73.5 %, `normalize3` 60.2 %, `perspective_90` 53.2 % of their
rows. The port reproduces the f64-once value on every one, so the corpus pins D4 rule b exactly (§5).

### 2.2 Speed

`taskset -c 2 cyrius bench`, 10⁶ calls per bench, in ns per call. The Rust column is the 1.2.0 release run.

| Bench | Rust | Cyrius | Ratio | Plan §11 |
|---|---|---|---|---|
| `color/from_hex` | 1.36 | 19.98 | 14.7× | 10–50× |
| `color/from_rgba8` | 3.30 | 15.70 | 4.76× | 10–50× |
| `color/lerp` | 1.05 | 38.92 | 36.9× | 10–50× |
| `color/luminance` | 0.76 | 11.07 | 14.5× | 10–50× |
| `color/to_array` | 0.52 | 4.91 | 9.47× | 10–50× |
| `vertex/bytemuck_cast_100` | 0.77 | 1.28 (`vertex/byte_view_100`) | 1.67× | — |

The Rust numbers are sub-nanosecond because criterion's loop inlines each call; a bare Cyrius call costs about 3 ns
(`noop`). Every ratio is within or below §11's band.

**Two wins, measured on the same box in the same session:**
- **Colour arithmetic in one typed helper per formula.** Each ganita `f32` op is a call, so `lerp` made 12. A helper
  with `: f32` parameters does the same native single-precision operations in one call per component: `lerp`
  75 → 37 ns (−51 %), `luminance` 20 → 11 (−45 %), `to_array` 11 → 5 (−55 %), `from_rgba8` 27 → 15 (−44 %),
  `from_hex` 30 → 19 (−37 %). All 3,397 rows stay identical.
- **Straight-line f64 matrices.** The loop form of `mul_mat4` widened each input four times: 294 ns; unrolled, 115
  (−61 %). Locals instead of in-memory f64 triples: `look_at` 116 → 75 (−35 %), `compose_trs` 59 → 46,
  `normalize3` 29 → 17, `normal_to_basis` 66 → 57. Every row stays bit-identical. `mul_mat4` stays at about 120 ns,
  which looks like the compiler's code for 48 f64 locals: a performance item for M4, whose shadow benches call it.

One full run read about 2× slow on everything, `noop` included (3 → 6 ns). Two repeat runs agreed with the earlier
numbers, so it was the machine's state; `bench-history.csv` keeps that run, and the record uses the later one. It
happened again at part d (the files after the first ran 2× slow), and again two full repeat runs agreed with the
per-file numbers. The box changes clock state (`cpu MHz` near 4 GHz when boosting), so **a run is accepted only
when a repeat agrees**, and the history keeps the outliers.

**Part d's extras** (no Rust bench: `environment` keeps these crate-private):

| Bench | Cyrius | What dominates |
|---|---|---|
| `tables/f16_to_f32` | 6.6–13 ns | bit manipulation and a call |
| `tables/f32_to_f16` | 12.5–25 ns | the rounding branch |
| `tables/srgb8_decode` | 3.5–7 ns | a table load |
| `tables/srgb_to_linear`, `linear_to_srgb` | 202–285 ns | **ganita's correctly rounded `pow`: 271 ns of 283**, measured alone |

The ranges are the box's clock states. A plain f64 `exp(y · ln x)` takes 49 ns but is not correctly rounded, and
x87 `exp`/`ln` differ across targets (§1.4). Nothing in M1 calls sRGB in a loop. M4's face encoding does, three
times per texel, which is about 1.2 s for six 512² faces. So M4 decides between an exhaustively checked fast `pow`
on the sRGB domain (about 3·10⁷ f32 inputs to decode, 8·10⁷ to encode) and a quantized encode by threshold search, which gives Rust's
bytes without calling `pow` at all.

**Part b2's extras** (no Rust bench), ns or µs per call:

| Bench | Cyrius |
|---|---|
| `color_optics/from_prakash` | 23 ns |
| `color_optics/from_wavelength` | 61 ns |
| `color_optics/from_temperature` | 458 ns (prakash's `pow` and `ln`) |
| `json/color_from_json` | 2.11 µs (bayan's value tree on a reset arena) |
| `json/color_to_json` | 3.90 µs (four Grisu2 conversions) |

These run at setup, not per frame. The first full run of the three read slow again; the next two agreed.

### 2.3 What M1 taught (rules for the next parts)

- **An `include` of a file that declares an `enum` must come before the first top-level statement.**
  tests/parity.tcyr includes the library first.
- **Assigning an integer to an f32 struct field warns under `--bounds`** ("keeps its integer bits"), so f32 fields
  are written with `store32` at their offsets (`SrColor`, the lights).
- **ganita's `f32_clamp` turns −0 into +0**; Rust's `clamp` keeps −0. `heat_map(−0)` would have lost its sign bit,
  so src/mathx.cyr has Rust's clamp (`_sr_clamp32`), and its round (half away from zero) and `as u8`.
- **Class B can't see a sign-of-zero slip**: −0 and +0 sit at the same point of the ordered line. Rows expected
  at 0 ulp were also run with B held to exact bits.
- **Constants that are arrays** (`IDENTITY_MAT4`, the `Color` constants) are pointers to static arrays filled at
  load (`var SR_COLOR_WHITE = &_sr_color_consts;`). A library's top-level code runs before the program's
  `alloc_init`, so they can't use `alloc`.
- **There is no `offsetof`.** The layouts are measured with P1's marker technique, in generated code
  (`scripts/oracle/gen_layout.py`).
- **An integer literal passed to an `: f32` parameter warns under `--bounds`**, as a field assignment does. Pass the
  bits through a variable (`var f255 = 0x437F0000;`), as the modules' named constants already do.
- **A static table is generated code**: `scripts/oracle/gen_tables.py` writes the decode table as `store32` lines
  run once at load. Cyrius has no array initializer.
- **An optional dependency is a profile, not an `#ifdef`.** Its code is a file outside `[lib]` that includes the
  dependency, a `[lib.<feature>]` profile bundles it, and the tests that need it include it (ranga's `spectral`).
  A replay of its rows gets its own harness (`_par_skip`), so the core replay compiles as a consumer without the
  feature does.
- **Measure what a stdlib leaf costs before making it a production dependency.** bayan's unicode tables survive
  dead-code elimination: adding bayan to `[deps] stdlib` took the smoke binary from 49,072 to 421,264 bytes, though
  nothing called it. So serde is the `[lib.serde]` profile, and bayan a test leaf.
- **`fn f(): Option` returns `Some(v)` or `None()`**; the caller binds both halves (`var t, v = f(…)`), as for a
  Result.

## 3. M2: 2D (parts a and b, 2026-11-15; M2 complete)

### 3.1 Parity

| Module | Rows | Classes | Result |
|---|---|---|---|
| `sprite` | 638 | A, B, C | all identical but one row: the rotated batch where glibc's `sinf`/`cosf` misround gives the correctly rounded corners (`sprite.cr.tsv`). `center`/`bounds` are identical even with class B held to exact bits. Batches of 16,382, 16,383, 16,384 and 16,483 sprites (u16) and 16,384 and 70,000 (u32) match as FNV-1a digests of the vertex and index bytes, so the truncation at 16,383 is Rust's |
| `text` | 479 | A | glyph rectangles for every code point 0–300 and unusual fonts (zero atlas, zero columns, saturating cells), and the sprites of 48 text batches (ASCII, spaces, empty, multi-byte UTF-8, control characters; scales including NaN) and 30 UI panels: all identical |
| `debug` | 151 | A, C | lines, boxes, grids (NaN and −0 edges), circles, spheres and capsules from 0 to 100 segments: all identical but one circle, where glibc's `sinf`/`cosf` misround and the port gives the correctly rounded vertices (`debug.cr.tsv`; `validate.py` explains every difference as glibc 1 ulp off on that angle) |
| `debug_collider` | 60 | C | each impetus shape kind, with f64 dimensions that don't fit f32: all identical (Rust impetus 1.4.0 against Cyrius impetus 2.0.0), replayed by `tests/parity_features.tcyr` |
| `json` | + 117 | A | `UvRect` and `Sprite`: serde_json's documents, and hand-written ones for serde's u64 / i32 rules (negative, float and out-of-range integers), the array forms and nested arrays; one bayan row pending (a `texture_id` above i64::MAX) |

### 3.2 Speed (`taskset -c 2 cyrius bench`; two runs agreed)

| Bench | Rust | Cyrius | Ratio | Plan §11 |
|---|---|---|---|---|
| `sprite/create` | 2.39 ns | 36.3 ns | 15.2× | 10–50× |
| `sprite/create_full_builder` | 3.54 ns | 42.8 ns | 12.1× | 10–50× |
| `sprite/center_bounds` | 0.25 ns | 29.8 ns | 117× | 10–50× |
| `sprite/batch_push_100` | 837 ns | 4.74 µs | 5.66× | 4–10× |
| `sprite/batch_push_100_prealloc` | 841 ns | 4.66 µs | 5.54× | 4–10× |
| `sprite/batch_sort_100` | 73.6 ns | 300 ns | 4.08× | 4–10× |
| `sprite/batch_push_1000` | 8.31 µs | 47.8 µs | 5.75× | 4–10× |
| `sprite/batch_sort_1000` | 566 ns | 3.28 µs | 5.80× | 4–10× |
| `vertex/quad_generation_100` | 634 ns | 5.55 µs | 8.76× | 4–10× |

- `center_bounds` is outside the band. Rust's 0.25 ns for two calls is less than a cycle, so criterion's loop has
  hoisted the work out; the Cyrius figure is two calls of real work.
- Both sort benches re-sort a sorted batch after their first iteration, and Rust's sort is adaptive. The port's
  sorted-input check gives the same behaviour: one pass, no scratch.
- **Part b's twins**, after the work below:

  | Bench | Rust | Cyrius | Ratio | Plan §11 |
  |---|---|---|---|---|
  | `pipeline/batch_to_vertices_100` | 1.19 µs | 9.40 µs | 7.91× | 4–10× |
  | `pipeline/batch_to_vertices_1000` | 11.9 µs | 93.1 µs | 7.82× | 4–10× |
  | `pipeline/batch_to_vertices_rotated_100` | 1.75 µs | 17.9 µs | 10.2× | 4–10× |
  | `pipeline/batch_to_vertices_into_1000` | 12.0 µs | 92.6 µs | 7.73× | 4–10× |
  | `debug_draw/wire_box_100` | 3.01 µs | 23.8 µs | 7.92× | 4–10× |
  | `debug_draw/wire_sphere_100` | 24.2 µs | 153 µs | 6.35× | 4–10× |
  | `debug_draw/grid_10x10` | 83.2 ns | 933 ns | 11.2× | 4–10× |

- **Debug geometry, first cut against final.** Writing a line's two vertices inline (one call per line, not four):
  `wire_box_100` 52.5 → 23.6 µs, `grid_10x10` 2.13 → 0.93 µs. Caching a circle's `sin`/`cos` table for its segment
  count (the values it would compute; module state, as the JSON arena), plus one typed call for a segment's 8 points:
  `wire_sphere_100` 566 → 153 µs. Every row stays identical.
- A rotated sprite costs about 83 ns more than an unrotated one: correctly rounded `sin` and `cos`, about 40 ns each.

### 3.3 What part a taught

- **A pointer stored past a header must be freed at the header.** The sort's keys array is `block + 8` (the first
  word records its capacity), and `SrSpriteBatch_drop` first freed the inner pointer. The unit tests passed (nothing
  allocated after the bad free); the golden replay, which builds and drops hundreds of batches, crashed.
- **A test's byte length is the test's to get right.** `"é A€"` is 7 bytes; given 6, the decoder correctly read the
  truncated `€` as U+FFFD, a fourth glyph.
- **A cache can be a parity-neutral speed-up** when it returns exactly the values it replaces: the circle's trig
  table changed no golden row.

## 4. M3: geometry and animation (parts a–c, 2026-11-15; M3 complete)

### 4.1 Parity

| Module | Rows | Classes | Result |
|---|---|---|---|
| `primitives` | 28 | A, E | the cube identical (A). Spheres from 0 × 0 (clamped to 3 × 2) to 32 × 16, planes, and cylinders up to 100 segments: 23 rows differ from Rust's f32 and match the f64-once value bit for bit (`primitives.e.tsv`) |
| `viz` | 599 | A, B, E | `unit_t`, `signed_t`, `magnitude_t`, `finite_min_max`, the grid checks, the colour maps, `with_alpha` and `lattice_indices`: all identical. Heatmaps and height fields (non-finite heights, zero spacing, single rows): 8 rows class E |
| `terrain` | 10 | A, E | the default and `flat_heightmap` identical; six terrains (0 × 5 to 32 × 32, a short height map, a NaN height): 5 rows class E |

How far Rust's f32 sits from the f64-once value on the 36 class-E rows:

| Function | Values that differ | Largest difference |
|---|---|---|
| `sphere` | 10,954 of 39,996 (27.4 %) | 33 ulp where \|v\| ≥ 1e-6, plus 770 values below 1e-6; 2.7e-7 absolute |
| `cylinder` | 2,186 of 12,096 (18.1 %) | 70 ulp where \|v\| ≥ 1e-6, plus 76 values below 1e-6; 3.7e-7 absolute |
| `plane` | 150 of 2,076 (7.2 %) | 3 ulp |
| `xz_heatmap_mesh` | 498 of 13,536 (3.7 %) | 1 ulp |
| `heightfield_mesh` | 122 of 1,200 (10.2 %) | 2 ulp |
| `generate_terrain` | 1,394 of 15,396 (9.1 %) | 2 ulp |

- The tessellation differences are the ones §7 predicted: Rust rounds the angle `(i / n) · τ` to f32 before `sin`
  and `cos`.
- The height-field normals (§7's open row) are within 2 ulp in f32. They are f64 under D4(b), the same rule as
  every other mesh: one formula, and the terrain twins stay inside §11's band.
- **The f64 `sin` and `cos` agree with glibc's once narrowed.** No geometry row needed a correctly rounded side
  table.

### 4.2 Speed (`taskset -c 10 cyrius bench`; two runs agreed)

| Bench | Rust | Cyrius | Ratio | Plan §11 |
|---|---|---|---|---|
| `terrain/generate_32x32` | 12.5 µs | 98.0 µs | 7.84× | 4–10× |
| `terrain/generate_64x64` | 48.1 µs | 378 µs | 7.86× | 4–10× |
| `terrain/generate_256x256` | 734 µs | 5.90 ms | 8.05× | 4–10× |
| `primitives/sphere_32x16` | 5.84 µs | 24.9 µs | 4.26× | 4–10× |

- **Sphere, first cut against final.** The first cut called `sin` and `cos` for every vertex, but θ repeats on every
  ring. Computing each column's pair once, into scratch from the caller's arena, took `sphere_32x16` from 68.1 to
  24.8 µs (−64 %). The bench compared the vertex bytes with the uncached form's, and no golden row changed.
- A terrain vertex costs about 90 ns. That covers two callbacks through `fncall` (height and colour), two clamped
  slopes, a `sqrt` and four f64 divisions.
- **Core 2 was busy.** A long-running process from another project sat on core 2 during this part. Every bench ran
  2× slow there, the old ones included (`debug_draw/wire_box_100` 45 µs against its recorded 23.8 µs). The recorded
  run is on core 10, with its SMT sibling idle. Every core on this machine (a Ryzen 7 5800H) is the same Zen 3 core.
  There the old benches reproduce their history: `wire_box_100` 23.6 µs, `math/mul_mat4` 121 ns.

### 4.3 What part a taught

- **Two agreeing runs are not proof of a quiet machine.** Four contended runs agreed with each other at 2× slow.
  Check that the core is idle (`/proc/stat`) and that a known bench reproduces its recorded figure.
- **A twin is found by its literal name.** `bench_record.py` reads `bench_new("<name>")`. A helper that took the name
  as a variable hid three twins until the name moved to the call site.
- **A Rust struct's home decides its surface.** `MeshData` is defined in `gltf_loader.rs` with a
  `material_index: Option<usize>`, so it grew that field now (`SR_MATERIAL_NONE`), not at part c. `TerrainData` has
  no such field, so the terrain builds into a local `SrMeshData` and copies the four words it has.
- **Rust's `f32::min`/`max` keep the later operand on a ±0 tie.** `finite_min_max` over `[-0.0, 0.0]` gives
  `(0.0, 0.0)` (golden row `set 3`). The golden rows caught it, and `validate.py` now compares bits.

### 4.4 Part b: animation parity

| Function | Rows | Class | Result |
|---|---|---|---|
| `Joint::default`, `JointUniforms::default`, `set_joints` | 7 | A | identical (`set_joints` with 0, 1, 3, 128 and 129 matrices) |
| `bracket`, `interpolate_keyframes` | 760 | A | identical on 14 tracks, unsorted and NaN-timed ones included: `sr_bracket` runs Rust 1.100's branch-free `partition_point`, so even an unsorted track picks Rust's keys |
| `build_keyframes` | 16 | A | identical for every mode, including truncated CUBICSPLINE triplets and mismatched counts |
| `slerp` | 451 | E | 283 rows differ from Rust and match the f64-once value bit for bit |
| `interpolate_rotation`, `AnimationClip::sample` | 441 | E | 143 rows differ from Rust and match the f64-once value |
| `local_transform`, `world_transforms(_resolved)`, `Skeleton::compute_joint_matrices` | 102 | E | 94 rows differ from Rust and match the f64-once value |

How far Rust's f32 sits from the f64-once value on the 520 class-E rows (values of magnitude ≥ 1e-6):

| Function | Values that differ | Largest difference |
|---|---|---|
| `Skeleton::compute_joint_matrices` | 3,536 of 5,456 (64.8 %) | 6,310 ulp, on a small element (9.8e-4, joint 61 of the 64-joint chain). Relative to each matrix's largest element, Rust's error on that chain grows with depth: 9.2e-8 at the root (about 1.5 f32 ulp), 5.6e-7 at joint 15, 1.3e-6 at the leaf. Largest ulp gap per joint (elements ≥ 1e-6): root 2, joint 15 40, joint 31 31, joint 47 43, leaf 183 |
| `world_transforms` | 3,486 of 5,456 (63.9 %) | 21,976 ulp, on a small element (3.5e-4, joint 33 of the 64-joint chain). The largest absolute gap is 4.4e-3 (85 ulp), on the scale-2 chain with translations near 100 |
| `local_transform` | 231 of 928 (24.9 %) | 44 ulp |
| `interpolate_rotation` | 286 of 492 (58.1 %) | 58 ulp |
| `AnimationClip::sample` | 309 of 1,820 (17.0 %) | 69 ulp |
| `slerp` | 529 of 1,132 (46.7 %) | Rust returns the identity for quaternions whose f32 dot product overflows (1e30 components), while f64 gives the normalized rotation, 0.707 away. Elsewhere tens of ulp |

### 4.5 Correctly rounded `acos` and `sin` (ledger L1)

The first replay passed 1,776 of the 1,777 rows. The one failure was `slerp(identity, cos 0.5 about z, t = 1.5)`: the
w component is −sin(π/6)/sin(π/3) + 0.5·sin(π/2)/sin(π/3), exactly 0. glibc's `acos(0.5)` is the correctly rounded π/3
(`0x3FF0C152382D7366`); ganita's is one ulp low (`…7365`, a 0.52-ulp error). After it, `sin((1 − t)θ)` is
−0.49999999999999994 rather than −0.5, and the w component is 1.1e-16 rather than 0.

Over the 65,537 points k/65536 of [0, 1], against glibc 2.44:

| | Differs from glibc | Largest difference |
|---|---|---|
| `ganita_f64_acos` | 14,296 (21.81 %) | 2 ulp |
| `ganita_f64_sin` (at 2x) | 1,688 (2.58 %) | 1 ulp |

- The final f32 rounding hides these differences almost always. The tessellation rows (part a) never showed them.
  slerp's weights cancel, and then they surface.
- glibc's f64 functions are not always correctly rounded either. Against `validate.py`'s 80-digit decimal reference,
  they differ on about 1 in 3,000 random `acos` arguments and 4 in 3,000 `sin` arguments.
- So the class-E reference is now Rust's formula in f64 **with correctly rounded transcendentals**. `validate.py`
  computes them itself (Taylor series and Newton in `decimal`, then Python's correctly rounded conversion). The port
  calls kimiya 2.0.1's `_km_cr_acos` and `_km_cr_sin`: two-phase, on MPFR tables, from + − × ÷, `f64_sqrt` and
  integer operations only. They are vendored by `scripts/vendor_crmath.py` as `src/crmath.cyr`: 29 functions,
  26 tables, renamed `_sr_km_`. That is L1's planned stand-in, arriving at M3 rather than M5.
- **Cost.**
  - Binary size: a binary that includes soorat carries crmath's tables, +20.2 KB (121,984 against 101,776 bytes for
    a `from_hex` program). One that reaches slerp also carries the code, +36.6 KB in all (154,752 against 118,160).
  - Time: about 68 ns a call more than ganita's (§4.6).
- The numbers went to ganita's open correctly-rounded request (L1). The f32 fmod went to its open no-fmod issue
  (L22), with `_sr_f32_fmod` as the stand-in.

### 4.6 Part b speed (`taskset -c 10 cyrius bench`; two runs agreed)

| Bench | Rust | Cyrius | Ratio | Plan §11 |
|---|---|---|---|---|
| `animation/compute_joints_16` | 283 ns | 4.34 µs | 15.3× | 5–15× |
| `animation/compute_joints_64` | 1.15 µs | 17.3 µs | 15.0× | 5–15× |
| `animation/compute_joints_64_unsorted` | 1.55 µs | 18.4 µs | 11.8× | 5–15× |
| `animation/sample_rotation_64x32` | 3.42 µs | 38.5 µs | 11.3× | — |

- **Joint chains, first cut against final.** The first cut stored each joint's TRS matrix and loaded it back for the
  multiply. Taking it from locals saved 9 % on every chain: `compute_joints_16` 4.66 → 4.25 µs, `_64` 18.6 → 17.0 µs,
  `_64_unsorted` 20.0 → 18.1 µs (the standalone runs). A joint is about 270 ns of f64 arithmetic: two 4 × 4 products
  and a TRS. Rust's f32 runs at about 18 ns a joint.
- The recorded run sits at the band's edge (15.3×), about 2 % slower than the standalone runs.
- **Correct rounding's price.** `sample_rotation_64x32` (64 slerps, each one `acos` and three `sin`) runs 20.6 µs with
  ganita's functions and 38.1 µs correctly rounded: about 68 ns more a call. The fast functions fail one golden row,
  so the record keeps the correct ones.
- core 2 was idle this time, but its SMT sibling (core 3) ran another project's job at 100 %, so the record uses
  core 10 again.

### 4.7 What part b taught

- **"Within 1 ulp" in f64 is not "the same after rounding to f32"** once a formula cancels. A reference that is exact
  needs correctly rounded functions on both sides. glibc's f64 functions are not always correctly rounded, so the
  reference computes its own.
- **Rust's binary search is part of its behaviour.** `partition_point` on an unsorted track returns whatever Rust
  1.100's branch-free search finds. Reproducing that search exactly, rather than any correct search, keeps those rows
  identical.
- **Static scratch counts against a compile's static data.** 17.6 KB of `var x[N]` buffers tipped `--bounds` into a
  "large static data" warning. One `alloc` on first use avoids it. The initialized MPFR tables did not count.

### 4.8 Part c: the glTF loader

**The corpus.** 146 fixtures, written by `scripts/oracle/src/m_gltf.rs` to `tests/golden/gltf/`, two rows each:
- probe P9's 51 fixtures (`test_support`'s two among them);
- 63 schema variants, each one substring of a document that uses every collection;
- sparse accessors (with and without a base view, u8/u16/u32 indices, unsorted and repeated indices, values past the
  buffer, a sparse animation output);
- 15 matrix joints, one for each branch of `Quaternion::from_matrix`, a mirror, a zero column and a shear;
- 21 animation and 3 mesh reader cases;
- two strings bayan and serde judge differently.

| Rows | Result |
|---|---|
| 266 | identical to Rust 1.2.0: 108 accepted documents (every vertex, index, image byte, joint, key and duration) and 158 refusals |
| 1 | `float_stress` animations: identical once its 41 adversarial decimals read correctly rounded (`gltf.cr.tsv`), where serde_json misrounds |
| 25 | named under ADR 0002 #14. Rust panics on 11 inputs (13 rows); bayan refuses 4 documents serde reads (8 rows); bayan reads 2 that serde refuses (4 rows) |

- **The schema** is gltf-json 1.4.1's, transcribed field by field: required fields, `Option`s, and defaulted fields
  (serde accepts `null` for the first kind of optional field and refuses it for the second).
- **The details serde and gltf-json fix**, each a fixture row:
  - an index is `value as u32`, so 4294967297 is index 1;
  - `mode: 4294967300` is TRIANGLES;
  - `extras` takes `[]` and `[null]` but not `[1]`;
  - a `TEXCOORD_+0` key is set 0, and the last equal key wins;
  - an orthographic `zfar: null` is refused, where a perspective one is not;
  - a channel's target is not validated, so a bad node or path fails only the animations, which Rust panics on.
- The first replay matched every row but `float_stress` (its 41 values, as P9 predicted).
- `validate.py` re-derives the 117 accepted documents in Python from the fixture bytes, sharing no code with the port.
  It covers the container, accessors, sparse merge, normalization, the f32 matrix decomposition and the keyframe
  pairing. The 162 refusals and 13 panics are the oracle's verdicts.
- **Memory.** The JSON tree lives on a decode arena reset per call. 100 loads of both loaders grow the global heap by
  0 bytes (`tests/gltf.tcyr`).

### 4.9 Part c speed and size (`taskset -c 10 cyrius bench`; two runs agreed)

| Bench | Cyrius | Notes |
|---|---|---|
| `gltf/load_rich` | 389 µs | both loaders on rich.glb (a 4.8 KB JSON chunk), two parses: about 40 ns per JSON byte |
| `gltf/load_float_stress` | 9.09 ms | both loaders on 125 KB of JSON (400 joints): about 36 ns per byte |

- Rust has no glTF bench, so these are extras. The parse dominates. bayan's own measurement is 15–25 ns per byte (its
  open issue of 2026-11-14); the schema walk and the readers add the rest.
- **Size.** The `gltf` profile costs a binary ~479 KB: 121,984 → 600,760 bytes for a program calling both loaders.
  372 KB of that is bayan's tables, which a binary carries as soon as it includes bayan; the rest is parser and loader
  code. Rust's loader is unconditional; here it is opt-in, as serde is (ADR 0002 #25).

## 5. M4: lighting math and infrastructure (parts a–c, 2026-11-15)

### 5.1 Part a: shadow math

| Function | Rows | Class | Result |
|---|---|---|---|
| `directional_light_matrix` | 1,008 | E | 415 rows differ from Rust and match the f64-once value bit for bit |
| `directional_light_matrix_at` | 440 | E | 297 rows differ, and match the f64-once value |
| `compute_practical_splits` | 3,502 | E | 796 differ, and match the f64-once value, with ganita's `pow` |
| `PointShadowMap::new` | 37 | E | 31 differ, and match the f64-once value |
| atlas math, defaults | 1,985 | A | identical |

How far Rust's f32 sits from the f64-once value:
- **Ordinary inputs:** up to 29 ulp on the centred directional matrices, 18 on the splits, 3 on the cube faces.
- **Where an intermediate overflows f32, Rust's results stop being finite.**
  - A direction with 1e30 components overflows Rust's normalize.
  - `far / near` overflows on the splits with far = 3e38: 637 split values are infinite or NaN in Rust and finite in
    f64.
- **`pow`.** Plan §7 said to use ganita's f64 `pow`, with a correctly rounded stand-in only where parity needs one.
  `validate.py`'s reference uses a correctly rounded `pow` (`decimal`'s `ln` and `exp` at 60 digits). ganita's matched
  it on every one of the 3,502 split rows once narrowed, so kimiya's `pow` was not vendored. Vendoring it would add 24
  functions and ~33 KB of tables to every binary.

### 5.2 Part a speed (`taskset -c 10 cyrius bench`; two runs agreed)

| Bench | Rust | Cyrius | Ratio | Plan §11 |
|---|---|---|---|---|
| `shadow/directional_matrix` | 20.9 ns | 115 ns | 5.51× | 5–15× |
| `shadow/directional_matrix_centered` | 21.5 ns | 110 ns | 5.11× | 5–15× |
| `shadow/cascade_splits_4` | 40.5 ns | 679 ns | 16.8× | 5–15× |
| `shadow/point_shadow_6_faces` | 213 ns | 1.26 µs | 5.93× | 5–15× |

- **The splits are above the band because of `pow`.** ganita's f64 `pow` costs 200 ns a call, measured alone, and four
  splits make three calls: 600 of the 679 ns. Rust's `powf` is about 10 ns, and Rust's 40.5 ns includes a `Vec`
  allocation. The only bit-identical speed-up is a faster correctly rounded `pow` (kimiya's runs its `sin` in 81 ns),
  at the binary cost above. Splits run once per frame per cascaded light, so this is a performance item, not a fix.

### 5.3 Part b: environment's CPU half, the PBR material and the BRDF LUT

| Module | Rows | Classes | Result |
|---|---|---|---|
| `env` | 197 | A, B, C | all identical. Decoding every byte and f16 special; the box filter on every size to 9, then 16, 33, 64, a short input, overflow and NaN; sRGB encoding at every byte's rounding boundary ± 3 ulps, where no glibc misround changed a byte (`env.cr.tsv` is empty); mip chains for both encodings and sizes 1 … 64 |
| `pbr_material` | 69 | A | identical |
| `pbr_optics` | 114 | A, R | `from_ior` identical. The BRDF LUTs (sizes 0 … 32, 0 … 128 samples) are prakash 2.15.2's: the 49 whose bytes differ from Rust's are class R (`pbr_optics.r.tsv`), all bit-identical to the independent evaluation; the other 15 (size 0, or 0 samples) are Rust's bytes. Replayed by the features harness |

- **prakash 1.2.0 against 2.15.2.** Rust calls prakash 1.2.0's `ior_to_f0` and `integrate_brdf_lut`, and Cyrius
  prakash 2.15.2 changed both:
  - `pbr_ior_to_f0` returns NaN unless the IOR is finite and positive. 1.2.0 returns `((1−ior)/(1+ior))²` for any IOR:
    1 for 0, +∞ for −1.
  - `pbr_integrate_brdf_lut` uses the image-based-lighting remap k = α/2, where 1.2.0 used the direct-lighting
    k = (r + 1)²/8. prakash's own notes call that a split-sum correctness fix. It also returns zeros for 0 samples,
    where 1.2.0 produces NaN, which quantizes to 0.

  On a 32² LUT at 256 samples, the two differ on 1,557 of 2,048 channel bytes, by up to 229.
- **The LUT is 2.15.2's (adopted 2026-11-15).** Part b first transcribed 1.2.0's formula, to keep Rust's bytes; the
  maintainer then adopted 2.15.2's, and `sr_generate_brdf_lut` now calls `pbr_integrate_brdf_lut`. F0 stays 1.2.0's
  formula, which agrees with 2.15.2's wherever 2.15.2 gives a number.
  - **Why it is better.** As roughness → 0, the half-vector is the normal and G → 1, so the split sum's
    scale + bias → 1. At roughness 1/32 (a 16² LUT, 64 samples) red + green is 247–255 of 255 across n·v with
    2.15.2's k. With 1.2.0's k it is 9 at the grazing texel, rising to 253 (`tests/pbr_material.tcyr` pins the
    band).
  - **How it is checked (class R).** `validate.py` evaluates 2.15.2's formula independently, operation for operation
    in Python's f64 with glibc's `sin`/`cos`, and writes `pbr_optics.r.tsv`: the 49 LUTs whose bytes differ from
    Rust's. The harness has a third side table for it (`.r.tsv`, beside `.e.tsv` and `.cr.tsv`), and a pass is
    *accepted (R)*. All 49 match bit for bit.
- **The LUT's trigonometry.** The port uses ganita's f64 `sin`/`cos`, which differ from glibc's on about 2.6 % of
  inputs. Both evaluations still quantize to the same bytes, as 1.2.0's did against Rust: a 1-ulp difference in a sum
  of hundreds of samples moves a channel by about 1e-16, nowhere near a byte boundary.
- **Rounding a LUT channel.** Rust's `round` is half away from zero. `floor(v + 0.5)` rounds k + 0.49999999999999994 up,
  so the port takes the fraction exactly, and so does the Python reference.

### 5.4 Part b speed

| Bench (extra: no Rust twin) | Cyrius | Notes |
|---|---|---|
| `env/face_mip_chain_128_srgb` | 1.40 ms | decode 16,384 texels, then 7 box filters and encodes |
| `pbr/brdf_lut_32x32_256` | 66.0 ms | 262,144 samples, about 252 ns each (`sqrt`, `sin`, `cos`, divisions); prakash 2.15.2's integration. 1.2.0's transcription recorded 60.8 ms |

- **Adopting 2.15.2's LUT costs about 5 %.** The recorded run is slower on untouched benches too (2–6 %), so the two
  were also timed interleaved in one binary, three rounds each on one core: 1.2.0's transcription 61.7–63.3 ms, the
  prakash call 64.4–67.3 ms (means 62.5 and 65.5). prakash evaluates G1(n·v) and α² every sample where the
  transcription hoisted them, calls `f64_max`, `f64_clamp` and its range-guarded `sin`/`cos` as functions, and passes
  the half-vector through a 24-byte buffer. A 32² LUT is built once at load, so the 3 ms stays as it is.

- **sRGB encoding, first cut against final.** M1 left this to M4 (§2.2): a correctly rounded `pow` costs about 270 ns,
  and encoding calls it three times a texel. M4 chose the threshold search. Each byte is the count of 255 thresholds
  at or below the input. Each threshold is the smallest f32 encoding to at least that byte, found once by binary
  search over the f32 bit patterns with the correctly rounded path (255 × 31 steps, about 2 ms, lazily). The byte is
  monotone in the input, so the result is exact. `tests/env_cpu.tcyr` compares it with the direct path over every
  threshold ± 4 ulps, every 2¹⁶th non-negative float, the knee ± 64 ulps, NaN, ±0 and ±∞. The 128² mip chain went
  from 4.97 to 1.40 ms. Scaled by texel count, six 512² faces now take about 0.13 s, down from M1's estimated 1.2 s.

### 5.5 Part c: the render graph; the profiler and capabilities moved to M6

| Module | Rows | Class | Result |
|---|---|---|---|
| `render_graph` | 435 | A | all identical: the four starting graphs, five changes to each forward graph, Rust's ten tests as scripts, cycles (two, three, a self-dependency), missing and disabled dependencies, a chain through a disabled pass (`[a, c]`, as Rust orders it), escaped and empty names, a 300-pass chain, a 100-pass fan-in, and 400 random scripts. `validate.py` re-derives every row with an independent recursive model |

- **Not over mabda's `rg_*`.** mabda 4.2.0's render graph records GPU work: compute, render and copy nodes, with the
  resources each reads and writes. Rust soorat's graph orders named passes with enable flags and name dependencies, so
  it is soorat's own code (ADR 0002 #28). The search is Rust's, made iterative: a pass's scratch (mark, next
  dependency, parent) stands in for the call stack, so the order matches with no recursion. A 3,000-pass chain is
  tested.
- **Lookup is a scan**, by length and then bytes, where Rust hashes. Ordering costs one lookup per dependency.

**The profiler and capabilities moved to M6** (decided at M4's close, 2026-11-15). Rust soorat re-exports Rust mabda
1.0's `FrameProfiler`, `PassTiming`, `ProfileScope`, `GpuTimestamps` and `GpuCapabilities`, and mabda becomes a
dependency at M6. What M4 found against mabda 4.2.0's Cyrius API, for M6:
- **Capabilities: everything is there.** `gpu_caps_max_texture_2d`, `_max_uniform_buffer` and `_max_bind_groups`
  serve `meets_soorat_requirements`. `webgpu_uniform_fits` and `WEBGPU_MAX_*` serve `check_soorat_uniforms`, whose
  eight uniform sizes are in `src/layout.cyr` (M1).
- **The profiler differs from 1.0's:**
  - no pass timings: 1.0 has `pass_times`, `record_pass`, `total_pass_time_ms`, and `begin_frame` clears them;
    4.2.0 documents a `PassTiming` layout but has none of these;
  - `end_frame` without `begin_frame`: 1.0 times it as 0 ms (it forgets the start after each frame); 4.2.0 times it
    from the last start, or from 0 ns (clock zero) before any;
  - `best_frame_ms` with no history: 1.0 gives `f64::MAX`, 4.2.0 gives 0;
  - no `frame_history`, `export_json` or `export_history_csv`; `ProfileScope` is `profile_begin`/`profile_end`.

  Four of soorat's eight profiler tests use the first two (`profiler_record_pass`, `profiler_begin_clears_passes`,
  `pass_timing_fields`, `profiler_end_without_begin`). All four gaps are filed in mabda (ledger L24).
- **mabda is not a dependency yet.** The plan brings it in at M6, where `error`'s `From<GpuError>` waits for it.
  Probe P5 built a minimal mabda consumer at 0.72 MB through the stdlib fold and 1.52 MB through `[deps.mabda]`
  (§1.4), and the latter pulls samvada and chitra into every compile until L12 lands.

### 5.6 Part c speed

| Bench (extra: no Rust twin) | Cyrius | Notes |
|---|---|---|
| `render_graph/build_forward_with_effects` | 1.98 µs | 7 passes and 8 dependencies: 15 name copies from the freelist, then `_drop` |
| `render_graph/order_forward_with_effects` | 327 ns | 8 dependency lookups and the search, no allocation |

- **`_sr_debug` measured its message before sakshi checked the level.** `execution_order` logs at DEBUG, as Rust's
  does, and each call ran `strlen` over a 60-byte message that DEBUG (off by default) then dropped. It now checks the
  level first, as a disabled `tracing::debug!` does: the order went from 371 to 319 ns (two runs each), and every
  DEBUG site gains the same.

### 5.7 M4's close: the milestone gate on three targets

§7's gate asks for the CPU classes on x86_64 ELF, x86_64 PE and aarch64. Both parity harnesses and the render-graph
tests ran on each, from the same tree:

| Target | How | Core harness (20 modules) | Features harness (3) | `render_graph.tcyr` |
|---|---|---|---|---|
| x86_64 ELF | `cyrius test --bounds` | 20,555 ok, 3,303 accepted, 7 pending, 0 failed | 1,829 ok, 49 accepted (R) | 23 passed |
| aarch64 | `cyrius test --aarch64 --bounds`, under qemu-aarch64 | the same counts | the same | 23 passed |
| x86_64 PE | `cyrius build --win`, under wine | the same counts | the same | 23 passed |

Every row's outcome is the same on all three targets, so the f64-once, correctly rounded and adopted rows hold
bit for bit across them, as pravash's do. `cyrius test` has no `--win`, and `cyrius build` prepends only the
production stdlib leaves. So the PE binaries were built from a wrapper that includes `lib/assert.cyr`,
`lib/overflow.cyr` and `lib/bayan.cyr` before the test file. `cyrius vet` is clean (8 dependencies, none untrusted or
missing).

