# Contributing to Soorat

Thank you for your interest in contributing to Soorat.

## Development Workflow

1. Fork and clone the repository
2. Create a feature branch from `main`
3. Make your changes
4. Run the Cyrius checks (`cyrius test --bounds`, `cyrius audit`). For the frozen Rust oracle, run `make check` inside `rust-old/` (see CLAUDE.md: `rust-old/` is never edited, only proved green)
5. Open a pull request

## Prerequisites

- Rust stable (MSRV 1.100), edition 2024
- Components: `rustfmt`, `clippy`
- Optional: `cargo-audit`, `cargo-deny`, `cargo-llvm-cov`
- A GPU (Vulkan/Metal/DX12) to run `tests/gpu_smoke.rs` for real — without an
  adapter those tests print a notice and pass

## Makefile Targets (the Rust oracle, inside `rust-old/`)

| Command | Description |
|---------|-------------|
| `make check` | fmt + clippy + test + audit |
| `make all` | `check` + deny + doc |
| `make fmt` | Check formatting |
| `make clippy` | Lint all features/targets with `-D warnings` |
| `make test` | Run the test suite (all features) |
| `make audit` | Security audit |
| `make deny` | Supply-chain checks (all features) |
| `make bench` | Run benchmarks with history tracking (`bench-history.csv`) |
| `make coverage` | Generate coverage report |
| `make doc` | Build documentation with `-D warnings` |

## Adding a Module

1. Create `src/module_name.rs` with a module doc comment
2. Add `pub mod module_name;` to `src/lib.rs`; gate code that needs a new
   dependency behind a feature flag
3. Re-export key types from `lib.rs`
4. Add tests in the module (and GPU coverage in `tests/gpu_smoke.rs` if it
   creates pipelines)
5. Add new WGSL files to `tests/shaders.rs` so CI validates them without a GPU
6. Update the README and `docs/architecture/overview.md`

## Code Style

- `cargo fmt` — mandatory
- `cargo clippy --all-features --all-targets -- -D warnings` — zero warnings
- No `unwrap()`, `expect()` or `panic!()` in library code — return `Result`,
  or sanitize and log with `tracing`
- `#[non_exhaustive]` on public enums, `#[must_use]` on pure functions,
  `#[inline]` on hot paths
- Meshes wind counter-clockwise around their normals (front faces under
  back-face culling); `viz::assert_winding_matches_normals` checks this in tests
- Treat model files, images and simulation data as untrusted input (see
  `docs/development/threat-model.md`)

## Testing

- Unit tests colocated in modules (`#[cfg(test)] mod tests`)
- Integration tests in `tests/` (`integration.rs`, `shaders.rs`, `gpu_smoke.rs`)
- Feature-gated tests with `#[cfg(feature = "...")]`
- Target: 80%+ line coverage

## Commits

- Use conventional-style messages
- One logical change per commit
- Behavior changes get a CHANGELOG entry; performance claims need benchmark numbers

## License

By contributing, you agree that your contributions will be licensed under GPL-3.0-only.
