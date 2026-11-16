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
| M1 part b2: colour `optics` + serde (prakash, bayan) | ☐ |
| M2–M10 | ☐ |

**M1 parts a–d.** 11,912 golden rows over seven modules, every one passing:
- 1,149 class-E rows are bit-identical to Rust's formula in f64 rounded once;
- the 31 rows where glibc misrounds give the correctly rounded value: 28 spot-light `cosf` rows and 3 sRGB `powf`
  rows.
`material` moved to M6: it owns a GPU texture and bind group. Measurements: `port-parity.md` §2.

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
- **Cyrius:** the library is `[lib]` in `cyrius.cyml`, bundled as `dist/soorat.cyr` (`cyrius distlib`):
  `src/{error,log,mathx,tables_data,tables,math,layout,color,lights,lod}.cyr`. `src/layout.cyr` and
  `src/tables_data.cyr` are generated (`scripts/oracle/gen_layout.py`, `gen_tables.py`). `src/main.cyr` is still the
  scaffold's smoke program.
- **Port machinery (M0):**
  - `scripts/oracle/`: the golden generator, `validate.py`, `gen_layout.py` and `gen_tables.py` (`tests/golden/`:
    constants, layout, math, color, lights, lod, tables: 11,921 rows; `math.e.tsv` holds 1,149 class-E rows,
    `lights.cr.tsv` 28 and `tables.cr.tsv` 3 correctly rounded rows);
  - `tests/parity.tcyr` with `parity_core.cyr`, `ulp.cyr` and the generated `layout_probe.cyr`: the replay;
  - `scripts/render_ref/`: the class-G corpus (`tests/golden/render/`, 12 scenes, adapter-pinned);
  - `scripts/{surface_parity,rust_test_ledger,bench_record,doc_examples,upstream_ledger}.py`, in CI.

## Tests

- **Rust oracle:** 501 tests pass with all features (460 unit, 12 integration, 9 GPU smoke, 18 oracle-hook,
  2 shader-validation). CI runs them under `rust-old/`.
- **Cyrius:**
  - `cyrius test` and `cyrius test --bounds` pass with zero warnings: 9 test files (62 ported Rust tests), and the
    parity replay of 11,921 rows (11,912 passing, 9 constants pending until their modules arrive);
  - `cyrius audit` exits 0; `cyrius distlib --check` is current.
- **Ledgers:**
  - surface: 912 public items, 161 ported (M1: 137 of 141; the 4 left are colour's `optics` functions);
  - Rust tests: 502, 62 ported (M1: 62 of 67; the 5 left are b2's; `docs/development/rust-test-ledger.md`);
  - benches: 6 of 35 twinned, plus 11 extras, `math/*` and `tables/*` (`docs/benchmarks-rust-v-cyrius.md`).

## Dependencies

Declared in `cyrius.cyml` today: the stdlib only (`string`, `fmt`, `alloc`, `vec`, `str`, `syscalls`, `io`,
`args`, `math`, `ganita`, `tagged`, `result`, `freelist`, `sakshi`; test: `assert`, `bench`, `overflow`). The
bundle's sidecar (`dist/soorat.deps`) needs `sakshi`, `string`, `ganita`, `freelist` and `result`. Each milestone adds its own; the full list and its order
are in the plan's §4 step 3.

**Open upstream requests** (ledger, plan §10):
- mabda: the render tier and the render-loop leaks (L2–L6, L17), and tidy-up (L12);
- name clashes: hisab/ranga (L18), agnodrm/sigil (L19), goonj/pravash (L20);
- cyrius: f32 types (L8);
- selah: raw-RGBA encoders (L9);
- agnodrm: display queries (L10);
- ushma: the hisab pin (L11);
- soorat's numbers added to the open ganita and cyrius issues (L1, L13).

## Consumers

kiran (rendering), salai (editor viewport), joshua (visualization): all still Rust, and they stay on soorat
1.2.0 until 2.0.0.

## Next

**The rest of M1**:
- b2: colour's `optics` half (`from_prakash`, `to_prakash`, `from_temperature`, `from_wavelength`; prakash) and
  `color_serde_roundtrip` (bayan). prakash's `Rgb` is heap-allocated through the bump allocator, 24 B per call,
  and its bundle is 1.19 MB, so this needs the optional-dependency design (ADR 0002 #9);
Then M1's gate closes (§14).

**Performance items carried forward:** `mul_mat4` at about 120 ns (M4's shadow matrices call it); the correctly
rounded sRGB `pow` at 200–285 ns a call (M4's face encoding calls it three times a texel). port-parity §2.2 has the
numbers and the options.
