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
| M1–M10 | ☐ |

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
- **Cyrius:** scaffold only (`src/main.cyr`, `tests/soorat.{tcyr,bcyr,fcyr}`). M1 adds the first modules.
- **Port machinery (M0):**
  - `scripts/oracle/`: the golden generator and `validate.py` (`tests/golden/`: constants, layout, math, 1,473 rows; `math.e.tsv` holds 1,149 class-E rows);
  - `tests/parity.tcyr` with `parity_core.cyr` and `ulp.cyr`: the replay; all rows are pending until M1;
  - `scripts/render_ref/`: the class-G corpus (`tests/golden/render/`, 12 scenes, adapter-pinned);
  - `scripts/{surface_parity,rust_test_ledger,bench_record,doc_examples,upstream_ledger}.py`, in CI.

## Tests

- **Rust oracle:** 501 tests pass with all features (460 unit, 12 integration, 9 GPU smoke, 18 oracle-hook,
  2 shader-validation). CI runs them under `rust-old/`.
- **Cyrius:**
  - `cyrius test` passes: the smoke test, plus the parity replay of 1,473 rows, all pending;
  - `cyrius audit` exits 0.
- **Ledgers:** 912 public items, 827 to port; 502 Rust tests, 471 to port (`docs/development/rust-test-ledger.md`); 35 benches (`docs/benchmarks-rust-v-cyrius.md`).

## Dependencies

Declared in `cyrius.cyml` today: the stdlib only (`string`, `fmt`, `alloc`, `vec`, `str`, `syscalls`, `io`,
`args`, `math`, `ganita`; test: `assert`, `bench`). Each milestone adds its own; the full list and its order
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

**M1 (foundation)**:
- error, mathx, the correctly rounded stand-ins, generated tables, math, color, layout, lights, lod, material;
- the first real replay: the `math` rows in class E, `layout` and `constants` in class A.
