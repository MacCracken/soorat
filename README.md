# Soorat

> **Soorat** (Arabic/Urdu: صورت — form, image, appearance) — GPU rendering engine for AGNOS

wgpu-based rendering engine designed for the [Kiran](https://github.com/MacCracken/kiran) game engine. Provides 2D sprites, 3D PBR meshes, shadows, skeletal animation, debug wireframes, post-processing, terrain, text, UI, and fluid rendering.

## Features

- **2D Sprites** — rotation, atlas UV regions, batched multi-texture draw, persistent GPU buffers, u16/u32 index paths
- **3D PBR** — Cook-Torrance/GGX/Fresnel-Schlick (ported from [prakash](https://github.com/MacCracken/prakash)), multi-light loop (8 lights), metallic-roughness materials, inverse-transpose normals
- **Skinned Meshes** — `SkinnedVertex3D` with 4 joint weights, vertex skinning shader, tangent-space normal mapping
- **Shadows** — directional PCF 3x3, cascaded shadow maps (1–4 cascades), shadow atlas, point light cube shadows
- **Lighting** — directional, point (range attenuation), and spot (cone falloff) lights
- **Animation** — skeleton/joint hierarchy, glTF skin loading, keyframe interpolation
- **Debug** — wireframe lines, boxes, circles, spheres, capsules, grids, physics collider shapes
- **Post-Processing** — HDR framebuffer (Rgba16Float), Reinhard + ACES tone mapping, bloom (threshold + Gaussian blur), SSAO
- **Terrain** — heightmap mesh generation with computed normals
- **Text** — bitmap font glyph atlas with text batching
- **UI** — screen-space panels, labels, and HUD overlays
- **Fluids** — SPH particle quads with velocity/density/pressure color mapping, shallow water surface meshes (via [pravash](https://github.com/MacCracken/pravash))
- **glTF** — model + animation loading with zero-copy buffer borrowing
- **Multi-window** — shared GPU context across windows
- **Profiling** — CPU frame timing (EMA + FPS), GPU timestamp queries, per-pass timing
- **Capabilities** — GPU feature/limit reporting, WebGPU compatibility validation

## Architecture

```
src/
├── Core:        color, vertex, error, gpu, window, profiler, capabilities, math_util
├── 2D:          pipeline (sprites), sprite, texture, text, ui
├── 3D:          mesh_pipeline (PBR), shadow, animation, terrain, fluid_render
├── Debug:       debug_draw (lines, shapes, grid)
├── Post:        postprocess, hdr, ssao
├── Loading:     gltf_loader, texture
├── Lights:      lights (directional/point/spot)
├── Materials:   material, pbr_material
├── Shaders:     sprite.wgsl, pbr.wgsl, pbr_skinned.wgsl, shadow.wgsl,
│                line.wgsl, postprocess.wgsl, bloom.wgsl, ssao.wgsl
└── Targets:     render_target
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
| `fluids` | [pravash](https://github.com/MacCracken/pravash) | SPH particle + shallow water rendering |
| `full` | all above | Everything |

## Building

```sh
cargo build
cargo test
cargo test --features full
cargo bench
```

## Stats

236 tests, 28 benchmarks, 31 modules, 8 WGSL shaders.

## License

GPL-3.0 — see [LICENSE](LICENSE).
