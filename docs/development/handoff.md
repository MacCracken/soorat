# Agent hand-over — soorat (2026-11-18)

For the next agent picking this repository up. Read this first, then `CLAUDE.md` (the rules) and
[ADR 0003](../adr/0003-correctness-is-the-oracle.md) (the oracle). The [port plan](soorat-port-plan.md) is still the
milestone map (§7 milestones, §8 mabda's render tier, §10 the upstream ledger, §14 tracking), but wherever it makes
Rust's output the expectation (D1, D4 rule a, §5's classes), ADR 0003 overrides it. Keep this file current when you
hand over. `CHANGELOG.md` stays the record of what changed.

## The direction: read this before touching anything

We are porting **away from Rust to Cyrius**.
- **`rust-old/` is the functionality reference**: it says *what* to port (the surface, the behaviour, the tests). It
  is **never edited** and **never deleted until the port is finished**.
- **It is not the oracle. Rust may be wrong.** The oracle is **accuracy and correctness**.
  - An expected value is the function's mathematics evaluated exactly and rounded once, or for non-numeric behaviour,
    what its specification requires.
  - Rust's recorded output is never the expected value.
- **No Rust in the test path**: no `cargo`, no Rust crates in `scripts/`, no Rust jobs in CI.
- **Do this repository's job on its own terms.** Sibling repositories answer factual questions (a tag, an API, a
  filed issue). They do not decide soorat's method; don't copy another port's approach.

## Where things stand

- **Rust 1.2.0** (tag `1.2.0`) is frozen in `rust-old/` as the functionality reference.
- **The Cyrius port (2.0.0) has M0–M5 ported: the whole CPU track** (last commit `3fee87b`, "handoff").
  - **What "passing" means today is not yet correctness.** The corpus (29 golden modules, 27,017 rows) was recorded
    from Rust by a Rust generator. After 0c's `color` bite, 19,069 rows still expect **Rust's outputs**. The other
    7,948 expect the correct value:
    - the 1,436 `math` rows, re-derived (0b);
    - the 3,439 `color` rows, re-derived (0c), 42 of them emitted;
    - 3,073 swapped in from side tables: 2,965 class E, 34 correctly rounded, 49 adopted (R) and 25 under ADR 0002
      #14.
    - So "all pass" means that the port reproduces Rust where no side table overrides it.
    - Probe P2 found Rust's f32 matrix math correctly rounded on only 37–78 % of its outputs.
    - Re-deriving the corpus from correctness is **Part 0** below, and it comes before M6.
  - The Rust-test ledger counts 502 tests: 316 ported, 28 n/a, 40 deferred and 118 pending (M6 64, M7 4, M8 35, M9
    15). This is functionality, and it stays.
  - Surface: 499 of 912 public items ported. Benches: 34 of 35 Rust benches twinned, plus 36 extras. The Rust
    column is recorded data; no Rust runs.
- **`em_render` follows in 2.1.0 (D7).** Its tests and items are `deferred` in both ledgers.
- **mabda: 4.4.0 and 4.5.0 have shipped** (tag 4.5.0 is `506c2cf`; mabda's main is at `d90d61b` "handoff
  clearout"). On 2026-11-18 soorat's GPU probes passed against 4.5.0 (the logs are in
  `scripts/probes/gpu/build/logs/`):
  - P6/MB1 builds the pipelines, checks the error cases and draws;
  - P6/MB2-5 passes 8 of 8 with soorat's shaders;
  - P7 uploads, and reads back with `bytes_match=1`;
  - P8 runs 100,000 frames at a flat 109.9 MB RSS and 0 B a frame.
- **P12 is set up but not measured:** `scripts/probes/bundle/p12_mabda450/` (mabda 4.5.0 as a `[deps.mabda]`
  dependency). It has its consumer, `tests/p12_mabda.tcyr`, committed in `c137b6f`. Its binary and static-data cost
  is not recorded yet. That measurement is step 2 of M6.
- **Committed by the maintainer:** Part 0a and 0b (`c137b6f`, `9b95cb5`).
- **Uncommitted in this tree:** Part 0c's `color` bite.
  - New: `src/crsum.cyr`, registered after `src/math.cyr` in every `[lib]` profile and in `tests/lib.cyr`.
  - Changed:
    - `src/color.cyr`, plus `src/env_cpu.cyr` and `src/acoustic_render.cyr`'s `PART0` calls;
    - `validate.py`: `rederive_module`, `color_reference` and `color_extra_rows`;
    - `tests/golden/color.tsv`;
    - ADR 0002 #33, and ADR 0003's constants rule;
    - port-parity §2.1 and §2.2, `state.md`, `CHANGELOG.md`, the oracle README and this file;
    - the regenerated `dist/` bundles and the bench record.

## Part 0: correctness becomes the oracle (before M6)

This is large, so take small bites (CLAUDE.md, *Task Sizing*), each gated.

- **0a. Rust out of the test path: done 2026-11-18.**
  - Deleted `scripts/oracle/{Cargo.toml,Cargo.lock,src/}` (the generator, 22 `.rs` files),
    `scripts/render_ref/{Cargo.toml,Cargo.lock,src/}` (the Rust renderer) and `scripts/probes/rust/` (the M0
    probes, whose results are in port-parity.md). They are recoverable from `3fee87b`.
  - Removed the gate's `cargo run` step and CI's six `oracle-*` jobs. `rust-old/` is untouched.
- **0b. `math`, end to end: done 2026-11-18.**
  - `validate.py`'s `math_reference` derives every output with no floating point.
    - The f32 inputs are exact `Fraction`s.
    - Every output is a rational z or z / √W.
    - `round_sqrt_f32` rounds √(z²/W) to f32 by integer arithmetic.
    - The sign of an exact zero follows IEEE 754's rules, and the two NaN-input rows follow its special-value
      rules.
    - The rounding was checked on 23,015 values with a known f32.
  - `validate.py . --write` rewrote `math.tsv`'s outputs (inputs and notes kept) as class A, and removed
    `math.e.tsv`. The check fails when a row differs from the reference.
  - **Rust 1.2.0 was wrong on 1,149 of the 1,436 rows** (4,493 of 15,638 outputs, by up to 796 ulp). Its
    `normalize3` and `normal_to_basis` of (1e30, 1e30, 1e30) return zeros. This is ADR 0002 #32, with the
    per-function table in port-parity §2.1.
  - **The port's f64-once method is correctly rounded on all 15,638 outputs**, so no port change and no bound were
    needed. The harness shows ulp 0 on every `math` output.
  - **The pattern for 0c:**
    - write a `<module>_reference` and a `check_<module>` like `check_math`;
    - `--write` the rows;
    - measure Rust and the port against it;
    - record Rust's defects as an ADR 0002 row and the table in port-parity;
    - then delete the module's Rust-emulation path in `validate.py`, once no other module uses it. Today the shadow,
      geometry and animation re-derivations still call `normalize3`/`look_at` with `r32`.
- **0c. The other numeric modules, a few per bite, in milestone order.** Where the port reproduces Rust's f32
  sequence (D4 rule a: `color`, the quantized outputs), change the port to the correct value wherever they differ.
  - **`color`: done 2026-11-18.**
    - Rust was wrong on 239 rows: `to_rgba8` on 97 (bytes off by 1), `luminance` on 71 (up to 2 ulp) and `lerp` on
      71 (up to 4 ulp). This is ADR 0002 #33, with the table in port-parity §2.1.
    - **ADR 0003's constants rule** came out of it. A standard's constants take their exact value, so Rec. 709's
      weights are 2126/10000. A threshold the code chose (1e-10, 0.99) is a definition, in its declared type.
    - **The port changed:** `to_rgba8` scales in f64. `luminance` and `lerp` go through the new `src/crsum.cyr`,
      which gives the correctly rounded f32 of an exact sum of f64 products.
    - **Why the helper:** f64 alone misses exact ties. Rec. 709's weights over 10,000 put about 3 luminances in
      10^5 on an f32 midpoint.
    - **42 emitted rows** (`ADR 0003:` notes) pin the helper's exact path: ties, cancellations, signed zeros,
      subnormals, the largest f32 and special values.
    - **Cost:** `color/lerp` 39 → 83 ns (79× Rust, **outside plan §11's 10–50× band**), `color/luminance` 11 → 29 ns
      (38×). The maintainer decides whether the band moves or `lerp` needs more work.
  - **Rust's colour path, kept by modules not yet re-derived** (the `PART0` markers), each to switch to the correct
    function in its own bite:
    - `env`: `_sr_color_q8_f32seq`, for the face bytes;
    - `acoustic_render`: `_sr_color_lerp_f32seq`, for the ray and decay colours;
    - `animation`: `_sr_f32_lerp`, for the linear tracks.
  - **`math` is measured, not proven** (a follow-up for the maintainer):
    - its f64 methods had 0 misses on 20,000 random inputs per function, beyond the corpus, but an exact value
      within about 2^-29 ulp of a midpoint could still round wrong;
    - `mul_mat4`, `cross` and the dot products are sums of exact products, so `_sr_cr_sum_f32` can make them
      provably correct.
  - **The pattern**, extending 0b's: a `<module>_reference`, and a `check_<module>` that calls
    `rederive_module(module, expected, derive, extra)`; emitted rows for what the recorded grid misses; port
    changes where the reference disagrees.
  - **Next, in M1 order:** `lights` (its `.cr.tsv` folds in), `tables` (with 0d's `tables_data.cyr`) and
    `color_optics`. Then M2's `sprite`, `text` and `debug`.
  - Class C rows (`.cr.tsv`) become ordinary rows whose value is correctly rounded.
  - Class R (`pbr_optics.r.tsv`, prakash 2.15.2's LUT) is checked against the LUT integral, not against either
    implementation.
- **0d. The generated sources taken from Rust's values.**
  - `src/tables_data.cyr`: today it holds Rust's 256 sRGB decode values. Regenerate it as the correctly rounded
    sRGB EOTF of each byte, in `gen_tables.py`.
  - `src/layout.cyr`: today it holds Rust's measured `#[repr(C)]` offsets. Check it against the WGSL layout rules
    for each GPU-facing struct, in `gen_layout.py`.
- **0e. The non-numeric modules, against their specifications.**
  - `render_graph`: validate.py already has an independent model.
  - `gltf`: the glTF 2.0 specification, with L23 and ADR 0002 #14 on Rust's panics.
  - `json`: the documented `Color` form and RFC 8259.
  - `lod`, `text` and `sprite`'s orderings.
- **0f. Close Part 0.**
  - Every TSV header names `validate.py` as its source, with no `cargo` line.
  - Rewrite the classes in plan §5 and the "validate.py" section of `scripts/oracle/README.md`.
  - Remove the Rust-generator wording from `state.md`, `port-parity.md` and the plan, and add a CHANGELOG entry.

## Then M6, the GPU core

1. **Pin mabda 4.5.0, or a newer tag if one exists.** Check `git -C ../mabda tag --sort=-creatordate` and its
   CHANGELOG. 4.5.0 turns about 1,570 integer `var` constants into `const` (S2), and makes the `*_is_*`/`*_has_*`
   predicates return `bool` (S3), so check soorat's calls against both.
2. **Finish P12**: write its test and measure mabda's binary cost and static data. For the static data, add a
   `var _pad: i64[16384];` probe and subtract 131,072 from the reported total. Today's bare test compile has 45,368 B
   under the 128 KiB advisory. Plan the harness layout before writing code, and re-run the name-collision scan,
   including the feature profiles' dependencies.
3. **Decide class G's correctness reference before porting any pixel test.** Rust's renders in
   `tests/golden/render/` are not an oracle (ADR 0003 #7). `scenes.json` and `derive_camera.py` stay as the scene
   definitions. Options to put to the maintainer:
   - scenes whose correct pixels follow analytically (flat fills, exact coverage, known blends);
   - the shading formulas evaluated exactly at sample pixels, with the GPU tolerance stated;
   - a CPU reference rasteriser in `scripts/`.
   The uploads (vertex, index and uniform bytes) are CPU math, so they take the Part 0 reference.
4. **Port M6** (plan §3.1, §7):
   - `texture.rs`, `render_target.rs`, `compute.rs`, then `material.rs`;
   - `pipeline.rs` (sprites), `debug_draw.rs` (lines) and `mesh_pipeline.rs`;
   - `gpu.rs`'s re-export (ADR 0002 #2, #3), and `profiler.rs` and `capabilities.rs` over mabda's `profiler_*` and
     `gpu_caps_*`;
   - `error.rs`'s three `from_gpu_error_*` tests.
5. **Close each part** with the gate, then the docs (state, plan §14, port-parity, CHANGELOG, ADR rows). Close M6
   with the three-target check.

## The maintainer's standing rules

Break none of them.
- **Never commit, push or tag.** Never use the `gh` CLI; curl the GitHub API if you must.
- **Never edit `rust-old/`, and never delete it until the port is finished.**
- **Rust may be wrong.** The expected value comes from correctness, never from Rust's output, and a Rust defect is
  recorded, never fixed by a Rust release.
- **No Rust in the test path.**
- **GPU access goes through mabda only**: no wgpu-native FFI in soorat (D3).
- **Every upstream gap is filed upstream.** Write the issue as a file in the sibling repository's
  `docs/development/issues/`, add a row to plan §10, and put an `UPSTREAM(Ln)` marker at any local stand-in.
  `scripts/upstream_ledger.py --check` verifies the filed paths. When upstream archives an issue, move the row's path
  to `archived/` and say what shipped.
- **Check git, tags and the sibling repositories for facts before asking**, but don't take soorat's method from them.
- **Scope is the maintainer's.** Don't cut, defer or widen work silently. If something can't be done, name it,
  track it, and say so. A dependency update is that update, not a rework.
- **Gates run by the maintainer must log first.** A gate only they can run (a Wayland session, DRM master) must
  write `build/logs/<gate>-<timestamp>.log` before you ask.
- **Expected toolchain notices aren't bugs.** With `[deps.mabda]` at 4.5.0, every command prints
  "./lib/ shadows version-pinned … mabda 4.5.0 (pinned: 4.2.0)". That is intended. Never run
  `cyrius lib sync --full`, which would downgrade it.

## How to verify (the gate)

```bash
cyrius deps --locked --features optics,physics-debug,acoustics,thermo,fluids
cyrius build
cyrius distlib --check                       # 8 bundles: the core and 7 profiles
cyrius test --bounds                         # zero warnings, every file passed
cyrius fmt --check src/*.cyr tests/*.cyr tests/*.tcyr tests/*.bcyr
cyrius audit                                 # directly, not through a pipe
cyrius vet src/main.cyr
python3 -I -B scripts/oracle/validate.py .
python3 -I -B scripts/oracle/gen_layout.py . --check
python3 -I -B scripts/oracle/gen_tables.py . --check
python3 -I -B scripts/surface_parity.py . --check
python3 -I -B scripts/rust_test_ledger.py . --check     # --write after a test or milestone change
python3 -I -B scripts/bench_record.py . --check         # regenerate with no flag after recording
python3 -I -B scripts/upstream_ledger.py . --check
python3 -I -B scripts/doc_examples.py . --check
python3 -I -B scripts/vendor_crmath.py --check
```

- **Lint per file:** `cyrius lint f` must report 0 warnings for every `src/*.cyr` and `tests/*` file. Lines may be at
  most 120 columns.
- **Benches:** run `taskset -c 10 cyrius bench` twice, then record the second run with
  `python3 -I -B scripts/bench_record.py . --append LOG --commit <sha>+wip` and regenerate. A Cyrius-only bench needs
  an `EXTRA` entry in `scripts/bench_record.py`.
- **At a milestone's close, run the three targets.**
  - aarch64: `cyrius test --aarch64 --bounds <files>`, under qemu.
  - x86_64 PE: `cyrius test` has no `--win`. Build a wrapper (`include` `lib/assert.cyr`, `lib/overflow.cyr`,
    `lib/bayan.cyr`, then the test file) with `cyrius build --win --locked --features <that harness's features>`,
    and run it under `wine`.
  - Compare each harness's per-function lines (port-parity §5.7, §6.9).

## Gotchas this arc found

- **Static data.**
  - A top-level bare `var g[N]` is N i64 slots, so write `var g: u8[N]`.
  - Each optional feature has its own replay harness, because no two fit under the 128 KiB advisory together:
    acoustics adds 46.9 KB, thermo 60 KB and fluids (pravash) 81.5 KB. The fluids replay is 3.2 KB under (L25).
- **`cyrius build --features X` resolves only X**, and it rewrites `lib/` and `cyrius.lock` unless you pass
  `--locked`.
  - `thermo` alone picks ushma's hisab 3.4.1 pin (L11).
  - Building with every feature compiles every dependency, and goonj's and pravash's `logging_init` collide (L20).
  - To repair the lock, run `cyrius deps --features optics,physics-debug,acoustics,thermo,fluids`, drop any stale
    commit pins, then confirm with `--locked`.
- **f32 and f64 in Cyrius** (CLAUDE.md has the full rules):
  - Float literals are written as bits.
  - An integer literal passed to an `f32` parameter goes through a variable first.
  - Assign an untyped-return call to an `f64` local before doing arithmetic with it.
  - Typed-pointer casts are helpers that return `*T`.
- **The side tables are being retired.** Until a module is re-derived (Part 0), the harness reads `<module>.e.tsv`,
  `.cr.tsv` and `.r.tsv` over the Rust-recorded rows. Don't add new rows in that form.
- **The scripts read plan §14:** a milestone is done when its Status cell has ✅. Closing one changes the ledgers'
  generated pages, so regenerate `rust-test-ledger.md` and `docs/benchmarks-rust-v-cyrius.md` afterwards.

## Open, and where it is tracked

All of these are rows in plan §10:
- **Open in upstream repositories:**
  - ganita's correctly rounded tier, with `src/crmath*.cyr` standing in (L1), and an fmod (L22);
  - cyrius f32 returns and consts (L8);
  - selah's raw-RGBA encoders (L9);
  - agnodrm's display queries (L10);
  - ushma's hisab pin (L11);
  - the name clashes L18, L19 and L20;
  - prakash's allocation-free colours (L21);
  - bayan's glTF verdicts (L23);
  - pravash's static data (L25).
- **After 2.0:** mabda's native render path (L7, mabda v5.0), and bijli's visualization surfaces with `em_render`
  (L16, 2.1.0).
- **soorat's numbers on open cyrius issues:** `memcpy` word copies (L13), and `cyrius port`'s doc clobbering (L14).
- **A new project:** a shared Wayland window and input library (L15).
