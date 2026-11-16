# scripts/oracle: the golden corpus from the Rust oracle

`soorat-oracle` drives the frozen Rust soorat **1.2.0** in [`rust-old/`](../../rust-old/), with its hidden
`oracle` hooks (port plan D1), and writes the golden corpus, `tests/golden/<module>.tsv`: one row per call, every
float as its IEEE-754 bit pattern. The Cyrius parity harness (`tests/parity.tcyr`, on `tests/parity_core.cyr`
and `tests/ulp.cyr`) replays every row against the port. `validate.py` re-derives the rows independently of Rust
and writes the class-E rows.

```sh
cargo run --release --locked --manifest-path scripts/oracle/Cargo.toml             # write tests/golden/*.tsv + modules.txt
cargo run --release --locked --manifest-path scripts/oracle/Cargo.toml -- --check  # verify; exit 1 on any difference, 2 on another platform
python3 -I -B scripts/oracle/validate.py .                                         # re-derive every row; check the .e.tsv
python3 -I -B scripts/oracle/validate.py . --write                                 # (re)write the .e.tsv and .cr.tsv side tables
python3 -I -B scripts/oracle/gen_layout.py . [--check]                              # (re)write src/layout.cyr, tests/layout_probe.cyr
python3 -I -B scripts/oracle/gen_tables.py . [--check]                              # (re)write src/tables_data.cyr
cyrius test --bounds tests/parity.tcyr                                             # replay the corpus against the port
```

The format, the comparison rules and the harness are pravash's, which are impetus's, kimiya's and tanmatra's.
Every port's harness reads the same files. This README records what is specific to soorat.

## Rules

- **`rust-old/` is never edited.** This crate depends on it by path. `Cargo.lock` was seeded from
  `rust-old/Cargo.lock`. Building this crate pruned the packages it doesn't need and added only
  `soorat-oracle`; no shared dependency changed version.
- **The output is deterministic.** Two runs give byte-identical files. Grids use only + − × ÷, exact conversions
  and the probes' seeded xorshift64. They never use a transcendental std call, a timestamp or a hash-map order.
- **Every header records the platform** (`libm: glibc 2.44 x86_64 fma`), and `--check` refuses a corpus recorded
  on another. Today's modules call no libm function, but class-C modules (from M1) will.
- **A row is never patched.** A wrong value is fixed in Rust as 1.2.x and recorded again.
- **Modules arrive with their milestone.** Each one adds `src/m_<module>.rs` with the grid from the port plan
  §7, registers it in `src/main.rs`, and adds its re-derivation to `validate.py`.

| Module | Milestone | Rows | Emitter | What |
|---|---|---|---|---|
| `constants` | M0 | 18 | `src/m_constants.rs` | public constants (`MAX_*`, `IDENTITY_MAT4`, the `Color` constants, `UvRect::FULL`) and the oracle's `MAX_SEGMENTS`, `MAX_QUADS`, `XZ_QUAD_INDICES`. `validate.py` checks the integers against the literals in `rust-old/src` |
| `layout` | M0 | 19 | `src/m_layout.rs` | size and field offsets of every GPU-facing `#[repr(C)]` type (probe P1) |
| `math` | M0 | 1,436 | `src/m_math.rs` | `math_util` on probe P2's grid: `mul_mat4`, `normalize3`, `cross`, `perspective_90`, `look_at`, `compose_trs`, `normal_to_basis` (class E) and `flatten_mat4` (class A). `math.e.tsv`: 1,149 rows where f64-once differs |
| `color` | M1 | 3,397 | `src/m_color.rs` | `Color` without its `optics` half: every byte in `from_rgba8`; `to_rgba8` at every k/510 and every halfway point (Rust rounds half away from zero); the colour maps across and beyond their domain, each stop ± 1 ulp, ±0, ±∞, NaN (A, B) |
| `color_optics` | M1 | 1,704 | `src/m_color_optics.rs` | `Color`'s `optics` half (the oracle crate turns on soorat's `optics` feature and depends on prakash 1.2.0): `from_prakash` at f32 midpoints and the clamp's edges, `to_prakash`, `from_temperature` every 100 K, `from_wavelength` every 0.5 nm, `n:` for `None` (A). Replayed by tests/parity_features.tcyr |
| `json` | M1 | 180 | `src/m_json.rs` | serde_json 1.0.154 (rust-old's dev-dependency, at its version) reading `Color` documents: its own output for a colour grid, and hand-written ones for serde's derive semantics; `E:Deserialize` for a refusal (A). The three `Color::from_json_bayan_*` rows are pending (ADR 0002 #19) |
| `lights` | M1 | 1,066 | `src/m_lights.rs` | the three light constructors and the light array; `GpuLight::spot`'s cone cosines are class C, and `lights.cr.tsv` holds the 28 rows where glibc's `cosf` misrounds (ADR 0002 #16) |
| `lod` | M1 | 770 | `src/m_lod.rs` | `LodChain` and `TerrainLod`: every threshold ± 1 ulp, NaN, ±∞, empty chains, mismatched lengths (`E:Pipeline`) (A) |
| `sprite` | M2 | 638 | `src/m_sprite.rs` | `UvRect`, `Sprite`, `sort_by_z`'s permutation, `batch_to_vertices` (C: rotated quads, `sprite.cr.tsv`), and the limit batches as FNV-1a digests |
| `text` | M2 | 479 | `src/m_text.rs` | glyph rectangles, and the sprites of text and UI batches (A) |
| `debug` | M2 | 151 | `src/m_debug.rs` | `LineBatch`'s lines and wire shapes (A; circles C, `debug.cr.tsv`) |
| `debug_collider` | M2 | 60 | `src/m_debug.rs` | `LineBatch::collider` over impetus 1.4.0's shapes (the oracle turns on `physics-debug`), C. Replayed by tests/parity_features.tcyr |
| `primitives` | M3 | 28 | `src/m_geom.rs` | the cube (A); spheres from 0 × 0 (clamped) to 32 × 16, planes and cylinders up to 100 segments (E). `primitives.e.tsv`: 23 rows where f64-once differs |
| `viz` | M3 | 599 | `src/m_geom.rs` | `unit_t`, `signed_t`, `magnitude_t` on a grid with NaN, ±∞, ±0 and empty ranges; `finite_min_max`, the grid checks, `with_alpha` (A); `heat_rgba`, `signed_rgba` (B); XZ heatmaps and height fields with non-finite heights, zero spacing and single rows (E); `lattice_indices` (A). `viz.e.tsv`: 8 rows |
| `terrain` | M3 | 10 | `src/m_geom.rs` | `TerrainConfig::default`, `flat_heightmap` (A); six terrains from 0 × 5 to 32 × 32, one with a short height map and one with a NaN height (E). `terrain.e.tsv`: 5 rows |
| `animation` | M3 | 1,777 | `src/m_anim.rs` | `Joint` and `JointUniforms` defaults, `set_joints` (A); slerp across the 0.9995 switch (each side ± 3 ulp), random and near-switch pairs, degenerate quaternions (E); brackets, linear and rotation interpolation on 14 tracks (duplicate, unsorted, NaN and infinite times, keys an ulp apart) at every key, midpoint and edge (A, rotation E); sampler pairing for every mode (A); local and world transforms and joint matrices of chains of 1–130 joints, leaf-first, cyclic and random (E); clip sampling with wrapped, non-finite and huge times, odd durations, STEP rotations and the bench clip (E). `animation.e.tsv`: 520 rows |
| `tables` | M1 | 5,215 | `src/m_tables.rs` | `environment`'s f16 and sRGB conversions, through the oracle hooks. f16 ⇄ f32 (A, 64 values a row): every subnormal half, every exponent's mantissa edges, every rounding midpoint ± 1 ulp, NaN payloads. sRGB (C): dense grids, each knee ± 2 ulp; `tables.cr.tsv` holds the 3 rows where glibc's `powf` misrounds. The 256-entry decode table (A), which `gen_tables.py` turns into `src/tables_data.cyr` |

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

Reference (R) checks are ported tests' own tolerances, not golden rows. Image parity (class G) uses its own
corpus, `tests/golden/render/` (M0 step 6).

The **note** is a human rendering of the call, for review only; the harness ignores it.

## `validate.py`

Python's floats are IEEE binary64, and an f64 `+ − × ÷ √` of two f32 values rounded once to f32 **is** the f32
operation (f64 carries more than 2·24 + 2 bits). So `validate.py`:
- re-derives Rust's f32 rows bit for bit, by evaluating each formula with every intermediate rounded to f32, in
  Rust's order. That checks the generator without Rust;
- derives the class-E outputs: the same formula with the f32 inputs and constants widened exactly, no
  intermediate rounding, and the result rounded once.

It re-derives every row of `math`, `color`, `lights`, `lod` and `tables` bit for bit, checks `constants` against
`rust-old/src`, and writes three side tables: `math.e.tsv` (class E); `lights.cr.tsv` (a spot light's cone cosines,
correctly rounded as glibc's f64 `cos` rounded once, where Rust's f32 `cosf` differs); and `tables.cr.tsv` (the sRGB
transfer functions with `powf` correctly rounded). A transfer-function row that differs from the correctly rounded
value must be Rust's formula with `powf` exactly 1 ulp off, or it fails. f16 rows are checked against Python's IEEE
half (`struct` format `e`).

## `gen_tables.py`

`src/tables_data.cyr` is generated from `tables.tsv`'s `srgb8_decode_table` row: Rust's own 256 values, written as
`store32` lines that fill a static table at load. `--check` (CI) fails when it is stale.

## `gen_layout.py`

`src/layout.cyr` is generated: one table of the 19 GPU-facing `#[repr(C)]` types with their fields' Cyrius types
(probe P1's), checked against `layout.tsv`'s sizes and offsets before anything is written. It also writes
`tests/layout_probe.cyr`, which measures each compiled struct with P1's marker technique, so the replay checks what
the compiler lays out rather than the table. `--check` (CI) fails when either file is stale.
