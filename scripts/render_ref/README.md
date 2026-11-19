# render_ref: the class-G scene definitions

**No Rust is built or run here** ([ADR 0003](../../docs/adr/0003-correctness-is-the-oracle.md)). The Rust renderer that
drew these scenes with the frozen Rust 1.2.0 (`src/*.rs`, a cargo crate on `rust-old/` and wgpu 29) is deleted; its
source is in git history at `3fee87b`.

What stays:
- **`tests/golden/render/scenes.json`**: the scene definitions (13 scenes, 128×128), and `scenes.md`, the meaning of
  every field and each scene kind's call sequence. These describe *what* to render, which is functionality.
- **`derive_camera.py`** (Python 3 standard library only): the scenes' camera matrices.

What is **not an oracle**: the PNGs, `manifest.tsv` and `uploads.tsv` under `tests/golden/render/`. They are Rust
1.2.0's output on one adapter (AMD Radeon Vega, RADV, Mesa 26.2.6), and Rust may be wrong. They stay as data until
M6 decides class G's correctness reference (see `docs/development/handoff.md`); no test may take its expected pixels
or uploads from them.
