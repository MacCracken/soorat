# Soorat — Claude Code Instructions

## Project Identity

**Soorat** (Arabic/Urdu: image/form, Farsi: face) — GPU rendering engine — wgpu, sprites, meshes, shaders, PBR pipeline, window management

- **Type**: Library: Cyrius at the root (the 2.0.0 port in progress); the Rust 1.2.0 oracle in `rust-old/`
- **License**: GPL-3.0-only
- **Toolchains**: Cyrius 6.7.18 (`cyrius.cyml`); Rust MSRV 1.100 (the oracle only)
- **Version**: SemVer 1.2.0 (`VERSION`; the Cyrius port ships as 2.0.0)

## Consumers

kiran (game rendering), salai (editor viewport), joshua (visualization)

**Note**: Integrates prakash for physically-based optics math.

## Cyrius Port (in progress, 2.0.0)

soorat is being ported to Cyrius. Plan of record: `docs/development/soorat-port-plan.md`. Measurements:
`docs/development/port-parity.md`. Decisions: `docs/adr/0001`, `docs/adr/0002`. Volatile status:
`docs/development/state.md`.

**Layout.**
- The **repository root is the Cyrius project**: `cyrius.cyml`, `src/`, `tests/`.
- **`rust-old/` is the frozen Rust 1.2.0 oracle.**
  - **Never edit it.** A wrong oracle value is fixed as a Rust 1.2.x release, and the corpus is regenerated.
  - The `cargo` commands in the processes below run **inside `rust-old/`**, and only to prove the oracle is
    still green.

**Cyrius cleanliness check** (every work-loop step that says "cleanliness check"):
- `cyrius deps --locked --features optics,physics-debug,acoustics,thermo` (the tests use every feature), `cyrius build`,
  `cyrius distlib --check` (the core bundle and the `optics`, `serde` and `physics-debug` profiles);
- `cyrius test --bounds`, with **zero warnings**: the f32 and f64 type diagnostics appear only there;
- `cyrius fmt --check`, `cyrius lint`;
- `cyrius audit`, run directly and not through a pipe, so its exit status counts.

**Rules learned from the M0 probes** (port-parity §1):
- **f32 arithmetic goes through helpers with `: f32` parameters.** An operator takes its type from its left
  operand.
- `fn f(): f32` is refused, so return the bits.
- Write f32 literals as their bits from rustc.
- Rust's `round` is half away from zero; ganita's is ties to even.
- Cyrius has no `&s.field`, and no array fields: generate indexed fields.
- **Names:** `Sr` types, `sr_` functions, `SR_`/`SR_ERR_*` constants. **Never `soorat_`**, because goonj
  defines those.
- **Memory:** per-frame data goes in a frame arena. The global `alloc` never frees.
- Generate vertices straight into the upload buffer; `memcpy` is a byte loop.
- **Class-E transcendentals are correctly rounded:** f64 `sin`/`cos`/`acos` from `src/crmath.cyr`, and `exp`/`log10`
  from `src/crmath_ext.cyr` in the `acoustics` profile (kimiya's, vendored by `scripts/vendor_crmath.py`; never edit
  them). ganita's are up to 2 ulp off, which shows wherever a formula cancels.
- **GPU access goes through mabda only.** No wgpu-native FFI in soorat.
- **Optional dependencies are profiles** (M1): the code is a file outside `[lib]`, bundled by `[lib.<feature>]`
  (`optics`: prakash; `serde` and `gltf`: bayan, whose tables cost every including binary ~372 KB; `physics-debug`:
  impetus; `acoustics`: goonj and hisab, ~329 KB; `thermo`: ushma, hisab and the bayan ushma reads JSON with, ~712 KB).
  Measure a new stdlib leaf's binary cost before making it a production dependency.
- **A top-level bare `var g[N]` is N i64 slots** (N × 8 bytes); a local one is N bytes. Write `var g: u8[N]` for an
  N-byte static buffer. Static data counts against the compiler's 128 KiB advisory, which is a gate warning.

## Development Process

### P(-1): Scaffold Hardening (before any new features)

0. Read roadmap, CHANGELOG, and open issues — know what was intended before auditing what was built
1. Test + benchmark sweep of existing code
2. Cleanliness check. Cyrius: see *Cyrius Port* above. The oracle, in `rust-old/`: `cargo fmt --check`, `cargo clippy --all-features --all-targets -- -D warnings`, `cargo audit`, `cargo deny check`, `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps`
3. Get baseline benchmarks (`./scripts/bench-history.sh`)
4. Internal deep review — gaps, optimizations, security, logging/errors, docs
5. External research — domain completeness, missing capabilities, best practices, world-class accuracy
6. Cleanliness check — must be clean after review
7. Additional tests/benchmarks from findings
8. Post-review benchmarks — prove the wins
9. Repeat if heavy

### Work Loop / Working Loop (continuous)

1. Work phase — new features, roadmap items, bug fixes
2. Cleanliness check. Cyrius: see *Cyrius Port* above. The oracle, in `rust-old/`: `cargo fmt --check`, `cargo clippy --all-features --all-targets -- -D warnings`, `cargo audit`, `cargo deny check`, `RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps`
3. Test + benchmark additions for new code
4. Run benchmarks (`./scripts/bench-history.sh`)
5. Internal review — performance, memory, security, throughput, correctness
6. Cleanliness check — must be clean after audit
7. Deeper tests/benchmarks from audit observations
8. Run benchmarks again — prove the wins
9. If audit heavy → return to step 5
10. Documentation — update CHANGELOG, roadmap, docs
11. Version check — VERSION, Cargo.toml, recipe all in sync
12. Return to step 1

### Task Sizing

- **Low/Medium effort**: Batch freely — multiple items per work loop cycle
- **Large effort**: Small bites only — break into sub-tasks, verify each before moving to the next. Never batch large items together
- **If unsure**: Treat it as large. Smaller bites are always safer than overcommitting

### Refactoring

- Refactor when the code tells you to — duplication, unclear boundaries, performance bottlenecks
- Never refactor speculatively. Wait for the third instance before extracting an abstraction
- Refactoring is part of the work loop, not a separate phase. If a review (step 5) reveals structural issues, refactor before moving to step 6
- Every refactor must pass the same cleanliness + benchmark gates as new code

### Key Principles

- **Never skip benchmarks.** Numbers don't lie. The CSV history is the proof.
- **Tests + benchmarks are the way.** Minimum 80%+ coverage target.
- **Own the stack.** If an AGNOS crate wraps an external lib, depend on the AGNOS crate.
- **No magic.** Every operation is measurable, auditable, traceable.
- **`#[non_exhaustive]`** on all public enums.
- **`#[must_use]`** on all pure functions.
- **`#[inline]`** on hot-path functions.
- **`write!` over `format!`** — avoid temporary allocations.
- **Cow over clone** — borrow when you can, allocate only when you must.
- **Vec arena over HashMap** — when indices are known, direct access beats hashing.
- **Feature-gate optional deps** — consumers pull only what they need.
- **tracing on all operations** — structured logging for audit trail.

## DO NOT
- **Do not commit or push** — the user handles all git operations (commit, push, tag)

- **NEVER use `gh` CLI** — use `curl` to GitHub API only
- Do not add unnecessary dependencies — keep it lean
- Do not `unwrap()` or `panic!()` in library code
- Do not skip benchmarks before claiming performance improvements
- Do not commit `target/` or `Cargo.lock` (library crates only)

## Documentation Structure

```
Root files (required):
  README.md          — quick start, features, dependency stack, consumers, license
  CHANGELOG.md       — per-version changes (Added/Changed/Fixed/Removed)
  CLAUDE.md          — this file (development process, principles, DO NOTs)
  CONTRIBUTING.md    — fork, branch, make check, PR workflow
  SECURITY.md        — supported versions, scope, reporting
  CODE_OF_CONDUCT.md — Contributor Covenant
  LICENSE            — GPL-3.0-only

docs/ (required):
  architecture/
    overview.md      — module map, data flow, consumers, dependency stack
    math.md          — (if applicable) mathematical reference for algorithms/formulas
  development/
    roadmap.md       — completed items, backlog, future features (demand-gated), v1.0 criteria

docs/ (when earned — not scaffolded empty):
  adr/
    NNN-title.md     — architectural decision records (when non-obvious choices are made)
  development/
    threat-model.md  — attack surface, mitigations (when security-relevant)
    dependency-watch.md — deps to monitor for updates/CVEs
  guides/
    usage.md         — patterns, philosophy, code examples
    testing.md       — test count, coverage, testing patterns

ADR format:
  # NNN — Title
  ## Status: Accepted/Superseded
  ## Context: Why this decision was needed
  ## Decision: What we chose
  ## Consequences: Trade-offs, what changes
```

## CHANGELOG Format

Follow [Keep a Changelog](https://keepachangelog.com/):

```markdown
# Changelog

## [Unreleased]
### Added — new features
### Changed — changes to existing features
### Fixed — bug fixes
### Removed — removed features
### Security — vulnerability fixes
### Performance — benchmark-proven improvements (include numbers)

## [X.Y.Z] - YYYY-MM-DD
### Added
- **module_name** — what was added and why
### Changed
- item: old behavior → new behavior
### Fixed
- issue description (root cause → fix)
### Performance
- benchmark_name: before → after (−XX%)
```

Rules:
- Every PR/commit that changes behavior gets a CHANGELOG entry
- Performance claims MUST include benchmark numbers
- Breaking changes get a **Breaking** section with migration guide
- Group by module when multiple changes in one release
- Link to ADR if a change was driven by an architectural decision
