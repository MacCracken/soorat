# Soorat

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for AGNOS

wgpu-based rendering engine designed for the [Kiran](https://github.com/MacCracken/kiran) game engine. Provides 2D sprite rendering, 3D PBR mesh rendering, shadow mapping, skeletal animation, debug wireframes, post-processing, terrain, text, and UI.

## Features

- **2D Sprites** — rotation, atlas UV regions, batched multi-texture draw, persistent GPU buffers
- **3D PBR** — Cook-Torrance/GGX/Fresnel-Schlick (ported from [prakash](https://github.com/MacCracken/prakash)), metallic-roughness materials
- **Shadows** — directional shadow maps with PCF 3x3 soft shadows
- **Lighting** — directional, point, and spot lights (up to 8)
- **Animation** — skeletal animation with glTF skin/joint loading, keyframe interpolation
- **Debug** — wireframe lines, boxes, circles, spheres, capsules, grids, physics collider shapes
- **Post-Processing** — Reinhard + ACES filmic tone mapping, exposure control
- **Terrain** — heightmap mesh generation with computed normals
- **Text** — bitmap font glyph atlas with text batching
- **UI** — screen-space panels, labels, and HUD overlays
- **glTF** — model loading with zero-copy buffer borrowing and embedded textures
- **Multi-window** — shared GPU context across windows
- **Profiling** — CPU frame timing with EMA smoothing and FPS counter

## Architecture

```
src/
├── Core:        color, vertex, error, gpu, window, profiler
├── 2D:          pipeline (sprites), sprite, texture, text, ui
├── 3D:          mesh_pipeline (PBR), shadow, animation, terrain
├── Debug:       debug_draw (lines, shapes, grid)
├── Post:        postprocess (tone mapping)
├── Loading:     gltf_loader, texture
├── Lights:      lights (directional/point/spot)
├── Materials:   material, pbr_material
├── Shaders:     sprite.wgsl, pbr.wgsl, shadow.wgsl, line.wgsl, postprocess.wgsl
└── Util:        math_util, render_target
```

## Quick Start

```rust
use soorat::{WindowConfig, Color, Sprite, SpriteBatch};
use soorat::window::run;

fn main() {
    let config = WindowConfig::new("My Game", 1280, 720);
    run(config, |_window| {}, move |window| {
        // render sprites, meshes, etc.
        true
    }).unwrap();
}
```

## Optional Features

| Feature | Crate | Provides |
|---|---|---|
| `optics` | [prakash](https://github.com/MacCracken/prakash) | Spectral color, PBR math, BRDF LUT |
| `ranga` | [ranga](https://github.com/MacCracken/ranga) | PixelBuffer texture loading |
| `physics-debug` | [impetus](https://github.com/MacCracken/impetus) | Collider shape wireframes |
| `full` | all above | Everything |

## Building

```sh
cargo build
cargo test
cargo test --features full
cargo bench
```

## License

GPL-3.0 — see [LICENSE](LICENSE).
