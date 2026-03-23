# Soorat Architecture

## Overview

Soorat is a wgpu-based GPU rendering engine for the Kiran game engine. It provides the rendering backend that Kiran's `Renderer` trait implementations connect to.

## Module Structure

```
src/
├── lib.rs      — crate root, re-exports
├── error.rs    — RenderError enum
├── color.rs    — RGBA color type
├── vertex.rs   — Vertex2D, Vertex3D with GPU layouts
├── sprite.rs   — Sprite instances, SpriteBatch
├── gpu.rs      — GpuContext (wgpu device/queue)
└── window.rs   — WindowConfig (winit)
```

## Consumers

- **kiran** — game engine, uses soorat as rendering backend
- **joshua** — headless simulation, may use soorat for visualization

## Dependencies

- `wgpu` — GPU abstraction (Vulkan/Metal/DX12/WebGPU)
- `winit` — window management
- `glam` — math types
- `bytemuck` — vertex type casting
- `image` — texture loading
