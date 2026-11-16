# Agent hand-over — soorat (2026-11-16)

For the next agent picking this repository up. The port is paused at the end of the CPU track while the maintainer
finishes mabda's next releases. **Pick it up when the maintainer says mabda's work is done.**

Read this first. Then read `CLAUDE.md` (the rules), [state.md](state.md) (the numbers) and the
[port plan](soorat-port-plan.md): §7 for milestones, §8 for mabda's render tier, §10 for the upstream ledger, §14
for tracking. Keep this file current when you hand over again. `CHANGELOG.md` stays the record of what changed.

## Where things stand

- **Rust 1.2.0** (tag `1.2.0`) is the final Rust release and the oracle, frozen in `rust-old/`. Never edit it.
- **The Cyrius port (2.0.0): M0–M5 are done, which is the whole CPU track.** The last commit is `1c7afb1` ("m5 done").
  - Corpus: 29 golden modules and 26,975 rows. All pass except 7 pending (3 constants of later milestones, 4 bayan
    rows). 4,222 are accepted: 4,114 class E, 34 correctly rounded, 49 adopted (R), 25 under ADR 0002 #14.
  - Rust tests: 502 in all. 316 are ported, 28 n/a, 40 deferred and 118 pending (M6 64, M7 4, M8 35, M9 15).
  - Surface: 499 of 912 public items are ported. Benches: 34 of 35 Rust benches are twinned, plus 36 extras.
  - M5's gate gave the same outcome for every row on x86_64 ELF, aarch64 and x86_64 PE (port-parity §6.9).
- **`em_render` follows in 2.1.0 (D7).** bijli 2.0.0 had not shipped at M5's close, so 2.0.0 goes without the `em`
  feature. Its tests and items are `deferred` in both ledgers.
- **mabda already has everything soorat asked for.** mabda 4.3.0 (MB1) and 4.3.1 (MB2–MB5), released 2026-11-16,
  shipped the render tier. They also resolved the render-loop leak (L17), the profiler gaps (L24) and the pin
  (L12). The ledger rows point at mabda's `docs/development/issues/archived/`. soorat's GPU probes pass against the
  4.3.1 bundle:
  - P6/MB1 builds all 11 pipelines, checks the error cases and draws the sprite frame;
  - P6/MB2-5 runs 8 of 8 with soorat's own shaders: the shadow map, bloom, the IBL cube, sRGB, MSAA and particles;
  - P8 runs 100,000 frames with flat RSS and 0 B a frame.
- **mabda's remaining releases are its own.**
  - 4.4.0 (compilers + NVIDIA) and 4.5.0 (textures + language) come first, then v5.0, which brings native rendering
    (soorat's L7, after 2.0).
  - Nothing M6 needs waits on them. The maintainer chose to finish mabda first.
  - 4.5.0 changes some public names' kinds: S2 turns about 1,570 integer `var` constants into `const`, and S3 has
    the `*_is_*`/`*_has_*` predicates return `bool`. Check soorat's calls against that when you pin.
- **Uncommitted in this tree:** the GPU probes mabda's MB session added, all in `scripts/probes/gpu/`:
  - `src/p6_mb1.cyr`, `src/p6_mb25.cyr`, `run_p6_mb1.sh` and `run_p6_mb25.sh`;
  - a `MABDA_DIST` switch in `build.sh`;
  - run logs in `build/logs/`.

  Committing them, and whether `build/logs/` belongs in `.gitignore`, is the maintainer's call.

## When you resume: M6, the GPU core

Do these in order, and gate each part (CLAUDE.md, *Task Sizing*: M6 is large, so take small bites).

1. **Check mabda's actual state; don't assume it.** Look at its tags (`git -C ../mabda tag --sort=-creatordate`),
   its `docs/development/handoff.md`, and its CHANGELOG since 4.3.1. Pin the newest tag.
2. **Re-run soorat's GPU probes against that bundle**, from `scripts/probes/gpu/`:
   ```bash
   MABDA_DIST=/home/macro/Repos/mabda/dist/mabda.cyr sh build.sh p6_mb1 p6_mb25 p8_gpu_frame
   sh run_p6_mb1.sh
   sh run_p6_mb25.sh
   ./build/p8_gpu_frame 100000 full
   ```
   The logs go to `build/logs/`. A failure here is a mabda issue: file it in mabda and add a ledger row, as always.
3. **Decide how mabda enters soorat, and measure before declaring it** (CLAUDE.md: measure a new leaf's binary cost
   first).
   - P5 (port-parity §1.4) measured 4.2.0: 0.72 MB through the stdlib fold and 1.52 MB through `[deps.mabda]`. The
     latter also pulled in chitra and samvada, which are optional since 4.3.0, so measure again.
   - Measure the **static data** as well: add a `var _pad: i64[16384];` probe, then subtract 131,072 from the
     reported total. Today's bare test compile has 45,368 B under the 128 KiB advisory (a gate warning). mabda is the
     largest dependency yet, so plan the harness layout before writing code. Each optional feature already has its
     own replay for this reason.
   - Re-run a name-collision scan (mabda found none against soorat at 4.3.1), including the feature profiles' deps.
4. **Port M6** (plan §3.1 and §7):
   - `texture.rs`, `render_target.rs`, `compute.rs`, then `material.rs`;
   - the sprite pipeline (`pipeline.rs`), the line pipeline (`debug_draw.rs`) and `mesh_pipeline.rs`;
   - `gpu.rs`'s re-export (ADR 0002 #2, #3), and `profiler.rs` and `capabilities.rs` over mabda's `profiler_*` and
     `gpu_caps_*`;
   - `error.rs`'s three `from_gpu_error_*` tests (`sr_error_from_gpu`).

   Class G compares pixels with the Rust oracle's corpus (`scripts/render_ref/`, `tests/golden/render/`). It is
   adapter-pinned to this box (Cezanne, AMD Radeon Vega, RADV): `render_ref --check` refuses another adapter.
5. **Close each part** with the full gate below, then the docs (state, plan §14, port-parity, CHANGELOG, ADR rows).
   Close M6 with the three-target check.

## The maintainer's standing rules

These come from `CLAUDE.md` and from corrections. Break none of them.

- **Never commit, push or tag.** Never use the `gh` CLI; curl the GitHub API if you must.
- **Never edit `rust-old/`.** A wrong oracle value is fixed as a Rust 1.2.x release, and the corpus is regenerated.
- **GPU access goes through mabda only**: no wgpu-native FFI in soorat (D3).
- **Every upstream gap is filed upstream.** Write the issue as a file in the sibling repository's
  `docs/development/issues/`, add a row to plan §10, and put an `UPSTREAM(Ln)` marker at any local stand-in.
  `scripts/upstream_ledger.py --check` verifies the filed paths. When upstream archives an issue, move the row's
  path to `archived/` and say what shipped, or the check fails.
- **Check git, tags and the sibling repositories before asking** anything they can answer.
- **Scope is the maintainer's.** Don't cut or defer work silently. If something can't be done, name it, track it,
  and say so.
- **Gates run by the maintainer must log first.** A gate only they can run (a Wayland session, DRM master) must write
  `build/logs/<gate>-<timestamp>.log` before you ask, as mabda's `test-surface-wayland-e2e` recipe does.

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
cargo run --release --locked --offline --manifest-path scripts/oracle/Cargo.toml -- --check
```

- **Lint per file:** `cyrius lint f` must report 0 warnings for every `src/*.cyr` and `tests/*` file. Lines may be
  at most 120 columns.
- **Benches:**
  - Run `taskset -c 10 cyrius bench` twice and record the second run with
    `python3 -I -B scripts/bench_record.py . --append LOG --commit <sha>+wip`, then regenerate.
  - A Cyrius-only bench needs an `EXTRA` entry in `scripts/bench_record.py`.
- **At a milestone's close, run the three targets.**
  - aarch64: `cyrius test --aarch64 --bounds <files>`, under qemu.
  - x86_64 PE: `cyrius test` has no `--win`. Build a wrapper (`include` `lib/assert.cyr`, `lib/overflow.cyr`,
    `lib/bayan.cyr`, then the test file) with `cyrius build --win --locked --features <that harness's features>`,
    and run it under `wine`.
  - Compare each harness's per-function lines (port-parity §5.7, §6.9).
- **Six oracle-crate clippy lints** (`scripts/oracle`, `-D warnings`) predate M5, and none is in an M5 file. CI runs
  clippy on `rust-old/` only.

## Gotchas this arc found

- **Static data.**
  - A top-level bare `var g[N]` is N i64 slots; write `var g: u8[N]`.
  - Each optional feature has its own replay harness because no two fit under the 128 KiB advisory together:
    acoustics adds 46.9 KB, thermo 60 KB, fluids (pravash) 81.5 KB. The fluids replay is 3.2 KB under it (L25).
- **`cyrius build --features X` resolves only X** and rewrites `lib/` and `cyrius.lock` unless you pass `--locked`.
  - `thermo` alone picks ushma's hisab 3.4.1 pin (L11). goonj's 3.5.0 wins only when `acoustics` is declared first.
  - Building with every feature compiles every dependency, and goonj's and pravash's `logging_init` collide (L20).
  - To repair a rewritten lock: run `cyrius deps --features optics,physics-debug,acoustics,thermo,fluids`, drop any
    stale commit pins it leaves, and confirm with `--locked`.
- **f32 and f64 in Cyrius** (CLAUDE.md has the full rules):
  - Write float literals as bits; `1.0e300` doesn't parse.
  - An integer literal passed to an `f32` parameter goes through a variable first.
  - Assign the result of an untyped-return call to an `f64` local before doing arithmetic with it.
  - Typed-pointer casts are helper functions returning `*T` (`_sr_pv_water`, `_sr_us_grid`).
- **Class E** (f64 rounded once where Rust computed in f32: the plan's D4, rule b):
  - the oracle module is `scripts/oracle/src/m_*.rs`;
  - `validate.py`'s `check_*` re-derives every row and writes `<module>.e.tsv` with `seq_tokens`, which matters
    because an f32 overflow can change an output's length;
  - the replay harness reads the side table.

  Colours keep Rust's f32 sequence after the one narrowed value.
- **The scripts read plan §14:** a milestone is done when its Status cell has ✅. Closing one changes the ledgers'
  generated pages, so regenerate `rust-test-ledger.md` and `docs/benchmarks-rust-v-cyrius.md` afterwards.

## Open, and where it is tracked

All of these are rows in plan §10:
- **Open in upstream repositories:**
  - ganita's correctly rounded tier, with `src/crmath*.cyr` standing in (L1), and an fmod (L22);
  - cyrius f32 returns and consts (L8);
  - selah's raw-RGBA encoders, which M8's screenshot needs (L9);
  - agnodrm's display queries, which M9's drm needs (L10);
  - ushma's hisab pin (L11);
  - the name clashes hisab/ranga (L18), agnodrm/sigil (L19) and goonj/pravash (L20);
  - prakash's allocation-free colours (L21);
  - bayan's glTF verdicts (L23);
  - pravash's static data (L25).
- **After 2.0:** mabda's native render path (L7, mabda v5.0) and bijli's visualization surfaces with `em_render`
  (L16, 2.1.0).
- **soorat's numbers on open cyrius issues:** `memcpy` word copies (L13) and `cyrius port`'s doc clobbering (L14).
- **A new project:** a shared Wayland window and input library (L15).
