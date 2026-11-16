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
python3 -I -B scripts/oracle/validate.py . --write                                 # (re)write tests/golden/<module>.e.tsv
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
| `s:<percent-encoded UTF-8>` | a string |
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

At M0 it re-derives all 1,436 `math` rows bit for bit, and checks `constants` against `rust-old/src`.
