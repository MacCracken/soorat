# 0003 — Correctness is the oracle; Rust 1.2.0 is the functionality reference

**Status**: Accepted (the maintainer's decision). Supersedes ADR 0001's oracle clause and D4's "keep Rust's f32
sequence".
**Date**: 2026-11-18

## Context

ADR 0001 made the frozen Rust 1.2.0 the oracle. A cargo crate (`scripts/oracle/`) drove `rust-old/` and recorded
its outputs as the golden corpus, and the port had to reproduce them. Where the correct value differed from Rust's,
the correct value entered as an exception: a class-E, class-C or class-R side table. A second crate
(`scripts/render_ref/`) recorded Rust's renders as M6's pixel reference, and a third (`scripts/probes/rust/`) held
the M0 probes. By M5's close, that meant:
- **The tests depended on Rust.** There were 12,936 lines of Rust in `scripts/`, a `cargo run` step in the gate, and
  six CI jobs building `rust-old/`, all in a project whose purpose is to leave Rust.
- **Rust's bits were the default truth.** Of the corpus's 26,975 rows, 22,753 expected Rust's recorded outputs, and
  4,222 were the "accepted" exceptions. Yet probe P2 had measured Rust's f32 matrix math as correctly rounded on only
  37–78 % of its outputs, and glibc's f32 transcendentals misround 1 call in 75. So "all rows pass" meant that the
  port reproduces Rust, not that it is correct.
- **Rust's defects were to be fixed in Rust:** "a wrong oracle value is fixed as a Rust 1.2.x release."

The maintainer's decision: Rust is for functionality, and accuracy and correctness are the oracle. No oracle script
may contain Rust. `rust-old/` stays until the port is finished.

## Decision

1. **`rust-old/` is the functionality reference.** It says what to port: the public surface, each function's
   behaviour, the tests and the benches. It is read, never edited, never built in the test path, and **never deleted
   until the port is finished**.
2. **The oracle is correctness.**
   - A numeric output's expected value is the function's mathematics evaluated exactly and **rounded once** to the
     output type. Rational operations are exact (`fractions`); roots and transcendentals use high precision
     (`decimal`, ≥ 60 digits).
   - Where Rust's formula *is* the definition, the exact value of that formula is the expectation.
   - A non-numeric output (render-graph order, glTF and JSON readers, struct layout) follows its governing
     specification (glTF 2.0, RFC 8259, WGSL's layout rules), checked by an independent model.
3. **Rust may be wrong.** Rust's outputs are never the expected value.
   - Where Rust's recorded output disagrees with the reference, the reference is checked first.
   - A confirmed disagreement is a **Rust defect**, recorded as an ADR 0002 row, and the port does the correct thing.
   - Rust 1.2.x is never released to change the tests.
4. **No Rust in the test path.** Deleted: `scripts/oracle/{Cargo.toml,Cargo.lock,src/}`,
   `scripts/render_ref/{Cargo.toml,Cargo.lock,src/}` and `scripts/probes/rust/`, all in git history at `3fee87b`.
   The `cargo` step is out of the gate, and the `oracle-*` jobs are out of CI.
5. **The corpus is re-derived, module by module.** Its inputs stay valid. Its outputs are rewritten from the
   reference, and the side tables (`.e.tsv`, `.cr.tsv`, `.r.tsv`) fold into the rows. Each module lists the rows
   where Rust's recorded value was wrong.
6. **The comparison rule.** The port's output is compared with the correctly rounded value, **bit for bit by
   default**. A function whose method cannot reach that gets a measured ulp bound, recorded in plan §5 before it is
   accepted.
7. **Class G (pixels) gets its own correctness reference at M6.** Rust's renders under `tests/golden/render/` are
   data, not an oracle.

## Consequences

- **Positive**
  - No Rust toolchain anywhere in the test path.
  - An expected value is right, not merely what Rust did.
  - Rust's defects are found and documented instead of reproduced.
- **Negative**
  - The 29 modules must be re-derived before the corpus can be trusted, and that comes before M6.
  - Port code that reproduces Rust's f32 sequence (D4 rule a: colours, quantized outputs) changes wherever the
    correct value differs.
  - A new row has no second real implementation behind it, so the reference itself must be reviewed against the
    mathematics and the specifications.
  - M6 loses its ready-made pixel corpus.
- **Neutral**
  - `rust_test_ledger.py`, `surface_parity.py` and `bench_record.py` keep reading `rust-old/`'s source and its
    recorded bench numbers. That is the functionality reference, and they run no Rust.
  - `validate.py`'s check of the constants against `rust-old/src` stays, because constants are functionality.

## Alternatives considered

- **Keep the Rust generator until the port closes** (ADR 0001 as written). Rejected by the maintainer: it makes the
  tests depend on the language being left.
- **Freeze the Rust-recorded corpus as the truth and only drop the generator.** Rejected: Rust may be wrong, and the
  frozen rows would keep its errors as the expectation.
- **Delete `rust-old/` now.** Rejected by the maintainer: it is the functionality reference until the port is
  finished.
