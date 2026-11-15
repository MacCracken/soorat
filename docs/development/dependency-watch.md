# Dependency Watch

Direct dependencies, their pinned ranges, and what to watch for.

Last reviewed: 2026-11-14 (Rust 1.100, edition 2024 — still the latest stable edition)

## Pinned by mabda (cannot move independently)

| Crate | Version | Notes |
|-------|---------|-------|
| mabda | 1.0 | GPU foundation. **1.0.0 is the final Rust release** — mabda now lives in Cyrius. Soorat passes wgpu types to and from mabda, so mabda's `wgpu ^29` requirement pins soorat's wgpu. |
| wgpu | 29.0.4 | Latest is 30.x. Blocked until mabda (or a soorat-owned replacement for the mabda types it uses) moves to wgpu 30. |
| naga (dev) | 29 | Must match wgpu's naga major (used by `tests/shaders.rs`). |

## Runtime

| Crate | Version | Notes |
|-------|---------|-------|
| winit | 0.30.13 | Latest stable. Pulls `ttf-parser` (RUSTSEC-2026-0192, unmaintained — not a vulnerability) via `sctk-adwaita`; ignored in `deny.toml` until winit drops ab_glyph. |
| bytemuck | 1.25.2 | |
| pollster | 1.0.1 | mabda still depends on 0.4, so both versions are in the tree (tiny crate). |
| serde | 1.0.229 | |
| thiserror | 2.0.21 | |
| tracing | 0.1.44 | |
| image | 0.25.10 | `png`, `jpeg`, `bmp` codecs (bmp is required by `ScreenshotFormat::Bmp`). |
| gltf | 1.4.1 | Last release 2024-05. Accessor iterators use checked slicing, but some arithmetic (e.g. `count - 1` for an accessor with `count = 0`) can panic in **debug** builds on malformed files; release builds wrap and the checked slicing rejects the range. Watch for a 1.5 or a maintained fork. |

## Optional (feature-gated)

| Crate | Version | Feature | Notes |
|-------|---------|---------|-------|
| hisab | 1.4 | `acoustics` | Only used for goonj's `Vec3`; must match goonj's hisab. Was a mandatory dependency before 2026-11. |
| prakash | 1.2 | `optics` | |
| ranga | 1.0 | `ranga` | 1.0 made `PixelBuffer` fields private (accessors). |
| impetus | 1.4 | `physics-debug` | MSRV 1.99. |
| pravash | 1.3 | `fluids` | MSRV 1.99. `ShallowWater` fields are public/deserializable — soorat validates them. |
| goonj | 1.4.3 | `acoustics` | |
| bijli | 1.2 | `em` | Do **not** enable bijli's `compute-wgpu` feature: it requires wgpu 30. |
| ushma | 1.4 | `thermo` | MSRV 1.99. |
| selah | 0.30 | `screenshot` | MSRV 1.100 (sets the crate MSRV floor with `full`). Last Rust release. Pulls reqwest/TLS. |
| uuid | 1.29.2 | `screenshot` | |
| chrono | 0.4.45 | `screenshot` | |
| agnosys | git tag 0.29.3 | `drm` | Repository renamed to **agnodrm** and no longer Rust; GitHub redirects the old URL. Tag 0.29.3 is the last Rust version — vendor or replace if the redirect ever breaks. |

## Dev

| Crate | Version | Notes |
|-------|---------|-------|
| criterion | 0.8.2 | |
| serde_json | 1.0.154 | |
