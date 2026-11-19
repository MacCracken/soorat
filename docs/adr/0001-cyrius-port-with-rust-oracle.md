# 0001 — Port soorat to Cyrius, with the frozen Rust 1.2.0 crate as the oracle

**Status**: Accepted; the oracle clause (Rust 1.2.0's outputs as the expectation, fixed by 1.2.x releases) and
D4's "keep Rust's f32 sequence" are superseded by [ADR 0003](0003-correctness-is-the-oracle.md)
**Date**: 2026-11-15

## Context

soorat's dependencies have moved to Cyrius: mabda, prakash, ranga, impetus, pravash, goonj, ushma, selah,
agnodrm and hisab, with bijli in progress. The AGNOS ecosystem now ports Rust crates by one method,
tanmatra's ADR 0006, as adapted by impetus, kimiya, pravash, bijli and selah:
- the last Rust release is frozen as an oracle;
- a generator records its outputs as golden rows;
- the Cyrius port must reproduce them under named tolerance classes, or diverge only through an accepted ADR.

soorat adds two things the sibling ports didn't have:
- **its GPU half depends on mabda features that don't exist yet** (the render tier, MB1–MB5);
- **half its output is pixels**, not numbers.

## Decision

Port soorat to Cyrius as **2.0.0**, by the sibling method. **Rust 1.2.0** is the final Rust release and the
oracle: tag `1.2.0` (`6625dc2`), the 1.1.0 audit plus the hidden `oracle` feature and two value fixes. It
lives in `rust-old/`, moved there by `cyrius port` at M0 step 2, and is **never edited**. A wrong oracle value
is fixed as a Rust 1.2.x release and the corpus is regenerated.

The details are in [the port plan](../development/soorat-port-plan.md). Decisions D1–D9 were taken
2026-11-15:
- D1: freeze the oracle with hidden read-only hooks;
- D2: fix value defects before the freeze;
- D3: WGSL on mabda's wgpu backend for 2.0;
- D4: precision policy:
  - quantized outputs keep Rust's f32 sequence;
  - GPU-bound geometry is computed in f64 and rounded once;
  - science inputs stay f64;
- D5: the application owns the window;
- D6: a soorat-owned glTF loader on bayan;
- D7: `em_render` waits for bijli 2.0;
- D8: reshaped modules;
- D9: shared dependency pins.

The parity classes, A–D plus the new E (improved) and G (image), are in the plan's §5. The probe measurements
behind them are in [port-parity.md](../development/port-parity.md).

## Consequences

- **Positive.** Every Cyrius row is checked against what 1.2.0 actually does, so the port can't silently
  drift. Accuracy gains, the E rows, are measured, not claimed: P2 found Rust's f32 matrix math correctly
  rounded on only 37–78 % of outputs, and P3 found ganita's f32 tier correct on all 226,050 of soorat's
  transcendental inputs, where glibc misrounds 1 in 75.
- **Negative.**
  - The repository carries two toolchains until 2.0.0: Cyrius at the root, Rust in `rust-old/`, with CI
    checking both.
  - The GPU track (M6–M9) waits on mabda's render tier, which is filed in mabda as
    `2026-11-15-render-tier-for-geometry-consumers.md`.
  - The full bundle doesn't compile until dependency name clashes are resolved upstream (ledger L18–L20).
- **Neutral.** No Rust 2.0. API-shape fixes land in the Cyrius API only, through
  [ADR 0002](0002-port-api-divergences.md).

## Alternatives considered

- **Rewrite without an oracle.** Rejected: no sibling port did, and every one found Rust defects, Cyrius
  defects or both through the oracle (tanmatra's hand-transcribed tables: 17 of 32 AME rows and about 30 of
  118 EA rows wrong; kimiya's misrounded glibc; pravash's 43 corrected math rows).
- **Keep soorat in Rust.** Rejected: its dependencies' Rust lines are final (mabda 1.0.0, ranga 1.0.1, …), and
  its consumers are moving to Cyrius.
- **Call wgpu-native directly from soorat** to avoid waiting on mabda. Rejected by the maintainer
  (2026-11-15): soorat reaches wgpu only through mabda.
