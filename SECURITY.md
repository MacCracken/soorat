# Security Policy

## Scope

Soorat is a wgpu-based rendering library: sprites, PBR meshes, shadows,
skeletal animation, post-processing, and scientific visualization. It contains
no `unsafe` code of its own. It parses untrusted data in a few places — glTF
models, PNG/JPEG images, and simulation data handed over by sibling crates —
and it drives the GPU, where invalid resource usage is a crash (wgpu's default
error handler panics).

## Attack Surface

| Area | Risk | Mitigation |
|------|------|------------|
| glTF loading | Malformed buffers/indices, out-of-range vertex references | gltf validation; buffer-view ranges bounds-checked; primitives with out-of-range indices or non-triangle modes skipped |
| Image decoding | Decompression bombs, oversized textures | `image` decoder allocation limits; dimensions checked against the device's `max_texture_dimension_2d` before upload |
| Visualization data | Grid headers larger than their data, NaN/inf samples | Grids must be backed by their data (checked products); non-finite samples map to neutral colors; segment counts clamped |
| GPU resource sizes | Zero-size textures/buffers, dispatch limits | Sizes clamped (≥1); particle capacity clamped to the dispatch limit; sprite draws clamped to u16 index range |
| Screenshot / clipboard (opt-in) | File/clipboard I/O | Feature-gated (`screenshot`); caller supplies paths |
| DRM queries (opt-in) | Kernel ioctls via agnosys | Feature-gated (`drm`), Linux only, read-only queries |
| Dependencies | Supply-chain compromise | `cargo-deny` (all features) and `cargo-audit` in CI; see `docs/development/dependency-watch.md` |

See `docs/development/threat-model.md` for details.

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.x | Yes |
| < 1.0 | No |

## Reporting

- Contact: **security@agnos.dev**
- Do not open public issues for security vulnerabilities
- 48-hour acknowledgement SLA
- 90-day coordinated disclosure
