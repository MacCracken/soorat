# scripts/oracle: the golden corpus and its correctness reference

**No Rust is built or run here** ([ADR 0003](../../docs/adr/0003-correctness-is-the-oracle.md)). Rust 1.2.0 in
[`rust-old/`](../../rust-old/) is the functionality reference: it says what each function does. It is **not** the
oracle, and its outputs are never the expected value. **The oracle is correctness**: each expected output is the
function's mathematics evaluated exactly (or at high precision for roots and transcendentals) and rounded once to the
output type, or, for non-numeric behaviour, what the governing specification requires.

**Where the corpus stands (2026-11-18).** The Rust generator that recorded `tests/golden/*.tsv` (`src/m_*.rs`, a
cargo crate driving `rust-old/`) is deleted; its source is in git history at `3fee87b`. The rows it wrote are still
the corpus, so today most expected values are still **Rust's outputs**, with the correct value swapped in only where
a side table says so (`.e.tsv`, `.cr.tsv`, `.r.tsv`). That is the defect ADR 0003 removes: module by module, the
expected outputs are re-derived from the correctness reference (`validate.py`), the inputs kept, and every row where
Rust's recorded value is wrong is listed as a Rust defect. The tracking is in `docs/development/handoff.md`.

```sh
python3 -I -B scripts/oracle/validate.py .                                         # re-derive every row; check the side tables
python3 -I -B scripts/oracle/validate.py . --write                                 # (re)write the .e.tsv, .cr.tsv and .r.tsv side tables
python3 -I -B scripts/oracle/gen_layout.py . [--check]                              # (re)write src/layout.cyr, tests/layout_probe.cyr
python3 -I -B scripts/oracle/gen_tables.py . [--check]                              # (re)write src/tables_data.cyr
cyrius test --bounds tests/parity.tcyr                                             # replay the corpus against the port
```

## Rules

- **`rust-old/` is never edited, and never deleted until the port is finished.** It is read for functionality only.
- **Rust may be wrong.** A row's expected value comes from the correctness reference, never from Rust. Where Rust's
  recorded output disagrees, the reference is checked first (a defect there is fixed in the reference); a confirmed
  disagreement is a Rust defect, recorded as an ADR 0002 row. Rust 1.2.x is never released to change the corpus.
- **The output is deterministic.** Two runs give byte-identical files: inputs from fixed grids and seeded xorshift64,
  no timestamp, no hash-map order.
- **Every header records the platform** where a row depends on it (glibc's libm, for the rows that document where
  glibc misrounds).
- **Modules arrive with their milestone**, with their inputs built and their outputs derived in Python here.

The modules below were recorded by the deleted Rust generator; each is re-derived from the correctness reference in
turn.

| Module | Milestone | Rows | What |
|---|---|---|---|
| `constants` | M0 | 18 | public constants (`MAX_*`, `IDENTITY_MAT4`, the `Color` constants, `UvRect::FULL`) and the oracle's `MAX_SEGMENTS`, `MAX_QUADS`, `XZ_QUAD_INDICES`. `validate.py` checks the integers against the literals in `rust-old/src` |
| `layout` | M0 | 19 | size and field offsets of every GPU-facing `#[repr(C)]` type (probe P1) |
| `math` | M0 | 1,436 | `math_util` on probe P2's grid: `mul_mat4`, `normalize3`, `cross`, `perspective_90`, `look_at`, `compose_trs`, `normal_to_basis` and `flatten_mat4`. **Re-derived (Part 0b, 2026-11-18)**: class A, every output the correct value from `math_reference`. Rust 1.2.0 was wrong on 1,149 rows (ADR 0002 #32) |
| `color` | M1 | 3,439 | `Color` without its `optics` half: every byte in `from_rgba8`; `to_rgba8` at every k/510 and every halfway point (Rust rounds half away from zero); the colour maps across and beyond their domain, each stop ± 1 ulp, ±0, ±∞, NaN. **Re-derived (Part 0c, 2026-11-18)**: class A from `color_reference`, plus 42 emitted rows for the exact path of `src/crsum.cyr`. Rust 1.2.0 was wrong on 239 rows (ADR 0002 #33) |
| `color_optics` | M1 | 1,704 | `Color`'s `optics` half (the oracle crate turns on soorat's `optics` feature and depends on prakash 1.2.0): `from_prakash` at f32 midpoints and the clamp's edges, `to_prakash`, `from_temperature` every 100 K, `from_wavelength` every 0.5 nm, `n:` for `None` (A). Replayed by tests/parity_features.tcyr |
| `json` | M1 | 180 | serde_json 1.0.154 (rust-old's dev-dependency, at its version) reading `Color` documents: its own output for a colour grid, and hand-written ones for serde's derive semantics; `E:Deserialize` for a refusal (A). The three `Color::from_json_bayan_*` rows are pending (ADR 0002 #19) |
| `lights` | M1 | 1,077 | the three light constructors and the light array; `GpuLight::spot`'s cone cosines. **Re-derived (Part 0c, 2026-11-18)**: class A from `lights_reference`, each cosine from `cos_f32_ref` (decimal, no libm); `lights.cr.tsv` is folded in, plus 11 emitted rows for probe P13's midpoint arguments and the range edges. Rust 1.2.0 (glibc `cosf`) was wrong on 28 rows (ADR 0002 #34) |
| `lod` | M1 | 770 | `LodChain` and `TerrainLod`: every threshold ± 1 ulp, NaN, ±∞, empty chains, mismatched lengths (`E:Pipeline`) (A) |
| `sprite` | M2 | 638 | `UvRect`, `Sprite`, `sort_by_z`'s permutation, `batch_to_vertices` (C: rotated quads, `sprite.cr.tsv`), and the limit batches as FNV-1a digests |
| `text` | M2 | 479 | glyph rectangles, and the sprites of text and UI batches (A) |
| `debug` | M2 | 151 | `LineBatch`'s lines and wire shapes (A; circles C, `debug.cr.tsv`) |
| `debug_collider` | M2 | 60 | `LineBatch::collider` over impetus 1.4.0's shapes (the oracle turns on `physics-debug`), C. Replayed by tests/parity_features.tcyr |
| `primitives` | M3 | 28 | the cube (A); spheres from 0 × 0 (clamped) to 32 × 16, planes and cylinders up to 100 segments (E). `primitives.e.tsv`: 23 rows where f64-once differs |
| `viz` | M3 | 599 | `unit_t`, `signed_t`, `magnitude_t` on a grid with NaN, ±∞, ±0 and empty ranges; `finite_min_max`, the grid checks, `with_alpha` (A); `heat_rgba`, `signed_rgba` (B); XZ heatmaps and height fields with non-finite heights, zero spacing and single rows (E); `lattice_indices` (A). `viz.e.tsv`: 8 rows |
| `terrain` | M3 | 10 | `TerrainConfig::default`, `flat_heightmap` (A); six terrains from 0 × 5 to 32 × 32, one with a short height map and one with a NaN height (E). `terrain.e.tsv`: 5 rows |
| `shadow` | M4 | 6,972 | `directional_light_matrix(_at)` (directions across the 0.99 up switch, every guard on both sides), `compute_practical_splits` (sanitized and overflowing inputs), `PointShadowMap::new` (E); the atlas tile math at its zero and saturation edges, the uniforms' defaults (A). `shadow.e.tsv`: 1,539 rows |
| `env` | M4 | 197 | environment's CPU half through the oracle hooks: `decode_face` (every byte, f16 specials; A), `encode_face` (linear values at every sRGB byte boundary ± 3 ulps, specials; C, `env.cr.tsv`), `downsample` (sizes 0 … 64, short input; B), `face_mip_chain` (both encodings, sizes 1 … 64; C/B), mabda's `mip_level_count` (A) |
| `pbr_material` | M4 | 69 | `MaterialUniforms::default`, `dielectric`, `metal` (A) |
| `acoustic` | M5 | 81 | `acoustic_heat_map` at its edges, NaN, ±∞ and ±1 ulp around each band; `AcousticVisParams` and `DecayCurveParams` defaults (A) |
| `acoustic_render` | M5 | 752 | the `acoustics` renderers on goonj 1.4.3's types built from f32-representable values (E): `ray_paths_to_lines` (empty, one and many bounces, NaN, infinite, negative, overflowing and −0 band energies, 200 random), `portal_to_lines` (axis and oblique normals, the 0.99 switch ± 3 ulps, zero and NaN normals, 200 random), `coupled_decay_to_lines` (every guard, step counts 0 … 500, 100 random); the meshes (part b): `pressure_map_slice` (empty, unbacked and out-of-range grids, non-finite values in every mode, 60 random), `mode_pattern_to_mesh` (empty, short, single-column, non-finite, 60 random), `directivity_balloon_to_mesh` (the segment minimums, missing tables, missing and NaN gains, zero and NaN fronts, 40 random). `acoustic_render.e.tsv`: 564 rows. Replayed by tests/parity_acoustics.tcyr |
| `thermo` | M5 | 12 | `thermal_heat_map`; `ThermalVisParams` and `CycleDiagramParams` defaults (A) |
| `thermo_render` | M5 | 236 | the `thermo` renderers on ushma 1.4.0's types, any f64 (`f:`): `thermal_grid_to_mesh` (empty, short, out-of-range and NaN values in both modes, equal and NaN bounds, inexact origin and spacing, 40 random), `temperature_profile_to_lines`, `cycle_diagram_to_lines` (non-finite points, degenerate ranges), `thermal_network_to_lines` (missing nodes, short conductances, non-finite values), `heat_flux_to_arrows` (NaN, stale and infinite maxima, f32 overflow), 40 random each (E). `thermo_render.e.tsv`: 189 rows. Replayed by tests/parity_thermo.tcyr |
| `fluid` | M5 | 8 | `fluid_heat_map` (0, ½, 1, out of range, NaN, an inexact value); `ParticleColorParams::default` (A) |
| `fluid_render` | M5 | 143 | the `fluids` renderers on pravash 1.3.0's types, any f64 (`f:`): `particles_to_quads` (no particles, every colour mode, zero and NaN maxima, an inexact speed ratio, odd values in each mode, 40 random), `particles_to_billboards` (no particles, a turned camera, skew and zero bases, odd values in each mode, 40 random), `shallow_water_to_mesh` (one column or row, dx 0, negative, NaN, past and below f32, short heights, non-finite heights, flat and centred surfaces, 30 random) (E). `fluid_render.e.tsv`: 117 rows. Replayed by tests/parity_fluids.tcyr |
| `render_graph` | M4 | 435 | `RenderGraph` driven by scripts of its public calls (add, depend, enable), with the order, the counts and passes queried along the way: the four starting graphs and changes to the forward ones, Rust's ten tests, cycles, missing and disabled dependencies, escaped names, a 300-pass chain, a 100-pass fan-in and 400 random scripts (A). `validate.py` re-derives every row with an independent model |
| `pbr_optics` | M4 | 114 | the `optics` half: `MaterialUniforms::from_ior` over every IOR edge (A), and `generate_brdf_lut` at sizes 0 … 32 and 0 … 128 samples (prakash 1.2.0's, in the corpus). The port adopts prakash 2.15.2's LUT: `pbr_optics.r.tsv` holds the 49 LUTs whose bytes change (class R, ADR 0002 #27). Replayed by tests/parity_features.tcyr |
| `gltf` | M3 | 292 | `load_gltf_meshes` and `load_gltf_animations` on 146 fixtures written to `tests/golden/gltf/` (a row's input names its file): probe P9's 51, and variants for the schema, sparse accessors, matrix joints and the reader's rules. `E:Model` for an error, `s:panic` where Rust panics (ADR 0002 #14). `gltf.cr.tsv`: 1 row (correctly rounded decimals). `validate.py` re-derives the accepted documents |
| `animation` | M3 | 1,777 | `Joint` and `JointUniforms` defaults, `set_joints` (A); slerp across the 0.9995 switch (each side ± 3 ulp), random and near-switch pairs, degenerate quaternions (E); brackets, linear and rotation interpolation on 14 tracks (duplicate, unsorted, NaN and infinite times, keys an ulp apart) at every key, midpoint and edge (A, rotation E); sampler pairing for every mode (A); local and world transforms and joint matrices of chains of 1–130 joints, leaf-first, cyclic and random (E); clip sampling with wrapped, non-finite and huge times, odd durations, STEP rotations and the bench clip (E). `animation.e.tsv`: 520 rows |
| `tables` | M1 | 5,215 | `environment`'s f16 and sRGB conversions, through the oracle hooks. f16 ⇄ f32 (A, 64 values a row): every subnormal half, every exponent's mantissa edges, every rounding midpoint ± 1 ulp, NaN payloads. sRGB (C): dense grids, each knee ± 2 ulp; `tables.cr.tsv` holds the 3 rows where glibc's `powf` misrounds. The 256-entry decode table (A), which `gen_tables.py` turns into `src/tables_data.cyr` |

## Format 1

Five header lines, then one row per call:

```text
fn <TAB> class <TAB> inputs <TAB> outputs <TAB> note
```

**Tokens** (space-separated; `-` for an empty list):

| Token | Meaning |
|---|---|
| `g:<8 hex>` | an **f32** as its bit pattern, lowercase. **soorat's addition**: most of its outputs are GPU data |
| `f:<16 hex>` | an f64 as its bit pattern, lowercase |
| `i:<decimal>` | an integer, or a boolean as 0 / 1 |
| `e:<Enum>.<Variant>` | an enum variant |
| `s:<percent-encoded UTF-8>` | a string: every byte outside 0x21–0x7e, and `%`, as `%XX` (uppercase) |
| `n:` | `None` |
| `E:<Variant>` | `Err(RenderError::<Variant>)`, with the payload not compared |
| `v:<len>` | a sequence; its element tokens follow |

**Classes** (port plan §5):

| Class | Rule |
|---|---|
| **A** | exact; NaN matches any NaN |
| **B** | ≤ 2 ulp, on the 32-bit ordered line for `g:` |
| **C** | the function's ulp bound (`_par_c_bound`); measured until set. Where glibc misrounds, the expectation is the correctly rounded value in `<module>.cr.tsv` (ADR 0002 #16) |
| **D** | relative ≤ 1e-12, or a bound set before the first comparison |
| **E** | *improved* (D4 rule b, ADR 0002 #4). The port evaluates Rust's formula in f64 and rounds once. Where that differs from Rust's f32 outputs, the expectation is the row in `<module>.e.tsv` (`validate.py --write`), compared **bit for bit**, and a pass is *accepted (E)*. Elsewhere it equals Rust's outputs, bit for bit |
| **S** | a solver step from a state: per field, its path's class |

Reference (R) checks are mostly ported tests' own tolerances, not golden rows. The exception is an **adopted**
formula: where the port takes a dependency's corrected formula in place of Rust 1.2.0's (ADR 0002 #27, prakash 2.15.2's
BRDF LUT), `validate.py` evaluates it independently and writes `<module>.r.tsv` for the rows whose outputs change. The
harness compares those rows against it bit for bit, and a pass is *accepted (R)*. Image parity (class G) uses its own
corpus, `tests/golden/render/` (M0 step 6).

The **note** is a human rendering of the call, for review only; the harness ignores it.

## `validate.py`

**The correctness reference** (ADR 0003). For a re-derived module, `validate.py` computes the expected outputs
without floating point: the f32 inputs as exact rationals (`fractions`), the formula's + − × ÷ exactly, and each
output rounded once to f32 (ties to even) by integer arithmetic. `math` is the first: every output there is a
rational z or z / √W, so `round_sqrt_f32` rounds √(z²/W) exactly, and the sign of an exact zero follows IEEE 754's
rules. A row with a non-finite input follows IEEE 754's special-value rules. `color` is the second (Rec. 709's exact
luma weights, under ADR 0003's constants rule). `lights` is the third: its cosines come from `cos_f32_ref`, a decimal
cosine whose precision grows with the argument (π to 90 + log10|x| digits), rounded once by integer arithmetic.

`rederive_module` does the writing for every re-derived module. `--write` rewrites the module's outputs (the inputs
and notes kept), appends its **emitted rows** and removes its retired side tables. The check fails when a row differs
from the reference. An emitted row is one whose inputs `validate.py` builds, for a case the recorded grid misses: a
tie, a cancellation, a signed zero. Its note starts `ADR 0003:`, and every write replaces it. `check_math` also
reports where the port's method (f64, rounded once) misses the reference: 0 of 15,638 for `math`.

**The modules not yet re-derived.** Python's floats are IEEE binary64, and an f64 `+ − × ÷ √` of two f32 values
rounded once to f32 **is** the f32 operation (f64 carries more than 2·24 + 2 bits). So, for them, `validate.py`:
- re-derives Rust's f32 rows bit for bit, by evaluating each formula with every intermediate rounded to f32, in
  Rust's order. That checks the generator without Rust;
- derives the class-E outputs: the same formula with the f32 inputs and constants widened exactly, no
  intermediate rounding, and the result rounded once.

It re-derives every module's rows but `layout`'s (gen_layout.py checks those), the glTF documents from their fixture
bytes; checks `constants` against `rust-old/src`; and writes the side tables:
- class E, Rust's formula in f64 rounded once where that differs from Rust's f32 outputs:
  `primitives.e.tsv`, `viz.e.tsv`, `terrain.e.tsv`, `animation.e.tsv`, `shadow.e.tsv`, `acoustic_render.e.tsv`,
  `thermo_render.e.tsv` and `fluid_render.e.tsv` (their exp, log10, sin, cos and acos correctly rounded by `decimal`),
  each a fresh token list, since an f32 overflow can change an output's length;
- correctly rounded, where a glibc f32 function misrounds (ADR 0002 #16): `lights.cr.tsv` (cone cosines), `tables.cr.tsv`
  (sRGB `powf`), `sprite.cr.tsv`, `debug.cr.tsv` and `debug_collider.cr.tsv` (`sinf`/`cosf`), `env.cr.tsv` (empty), and
  `gltf.cr.tsv` (a JSON decimal serde_json misreads, #14);
- adopted (class R, #27): `pbr_optics.r.tsv`, prakash 2.15.2's BRDF LUT.

A row whose Rust outputs use a glibc f32 transcendental re-derives with that function correctly rounded or 1 ulp off
(per call where the calls are independent: `powf` in the splits, `expf`/`log10f` per decay step), or it fails. f16
rows are checked against Python's IEEE half (`struct` format `e`).

## `gen_tables.py`

`src/tables_data.cyr` is generated from `tables.tsv`'s `srgb8_decode_table` row: Rust's own 256 values, written as
`store32` lines that fill a static table at load. `--check` (CI) fails when it is stale.

## `gen_layout.py`

`src/layout.cyr` is generated: one table of the 19 GPU-facing `#[repr(C)]` types with their fields' Cyrius types
(probe P1's), checked against `layout.tsv`'s sizes and offsets before anything is written. It also writes
`tests/layout_probe.cyr`, which measures each compiled struct with P1's marker technique, so the replay checks what
the compiler lays out rather than the table. `--check` (CI) fails when either file is stale.
