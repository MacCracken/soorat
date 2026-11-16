# tests/

The Cyrius test suites of the soorat 2.0 port. `cyrius test` runs the `*.tcyr` files, `cyrius bench` the `*.bcyr`
files and the fuzz harness is `soorat.fcyr`. Tests that the port carries over from the frozen Rust 1.2.0 oracle
(`rust-old/`) follow the conventions below, which `scripts/` checks in CI (port plan §4 step 7, §7).

## A ported Rust test is a `test_group` of the same name

- Each Rust `#[test]` is ported as `test_group("<the Rust test's name>")`, in one of the Cyrius files that
  `FILES` in `scripts/rust_test_ledger.py` maps its Rust file to. Those files are named after the Cyrius module of the
  plan's §3.1: `src/color.rs` → `tests/color.tcyr`, `src/math_util.rs` → `tests/math.tcyr`, `src/vertex.rs` and
  `src/instancing.rs` → `tests/layout.tcyr`, `tests/integration.rs` → `tests/integration.tcyr`, and so on. To use
  another file, change `FILES`.
- The same Rust name can appear in several modules (`heat_map_clamps` is in four), so a group counts only in its own
  Rust file's Cyrius files. Define each group name once per file.
- A doc test is the group `doc_<rust file stem>_<line of its fence>`.
- A function that holds a `test_group` must be called in its file, or the group never runs. The ledger flags one that
  isn't.
- A Rust test that the port doesn't carry gets a reason in `REASONS`: `n/a: …` if it is never ported, `deferred: …` if
  it comes after 2.0.0. The reason cites the plan or ADR 0002.
- GPU tests print a notice and exit 0 when there is no adapter, as Rust's do (plan §1.1).

`python3 -I -B scripts/rust_test_ledger.py .` prints the ledger, and `--write` writes
`docs/development/rust-test-ledger.md`.

## A Rust bench's twin has the same name

Each criterion bench `group/name` in `rust-old/benches/benchmarks.rs` has a Cyrius twin `bench_new("group/name")` in a
`*.bcyr` file. `TWINS` in `scripts/bench_record.py` lists the exceptions. A Cyrius-only bench goes in `EXTRA`.
`python3 -I -B scripts/bench_record.py . --append <cyrius bench log>` records a run in `bench-history.csv` at the
repository root, and `python3 -I -B scripts/bench_record.py .` rewrites `docs/benchmarks-rust-v-cyrius.md`.

## Milestones

A test or bench whose milestone is still open is **pending**, and that is not an error. A milestone is **done** when
its row in the plan's §14 Tracking table has ✅ in its Status cell. Once it is done, every test and bench of that
milestone must be ported or have a reason, and every public item must have its Cyrius name
(`scripts/surface_parity.py`). Every tool takes `--done M<n>` to preview that gate before the ✅ goes in.
