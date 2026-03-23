# Soorat

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for AGNOS

wgpu-based rendering engine designed for the [Kiran](https://github.com/MacCracken/kiran) game engine. Provides 2D sprite rendering, 3D mesh rendering, window management, and GPU pipeline abstraction.

## Architecture

```
soorat (rendering engine)
  ├── gpu         — wgpu device, adapter, queue management
  ├── window      — winit window configuration
  ├── color       — RGBA color types with hex/lerp/conversion
  ├── vertex      — 2D/3D vertex types with wgpu buffer layouts
  └── sprite      — sprite instances, batching, z-ordering
```

## Quick Start

```rust
use soorat::{WindowConfig, color::Color, sprite::Sprite};

let config = WindowConfig::new("My Game", 1280, 720);

let sprite = Sprite::new(100.0, 200.0, 64.0, 64.0)
    .with_color(Color::RED)
    .with_texture(1)
    .with_z_order(5);
```

## Building

```sh
cargo build
cargo test
```

## License

GPL-3.0 — see [LICENSE](LICENSE).
