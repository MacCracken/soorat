# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-23

### Added
- `Color` type with constants, hex/rgba8 constructors, lerp, wgpu conversion.
- `Vertex2D` and `Vertex3D` with bytemuck Pod/Zeroable and wgpu buffer layouts.
- `Sprite` with builder API (position, size, color, rotation, texture, z-order).
- `SpriteBatch` for batched sprite rendering with z-sort.
- `GpuContext` for wgpu adapter/device/queue management.
- `WindowConfig` with winit integration (vsync, fullscreen, present mode).
- `RenderError` enum with `#[non_exhaustive]`.
- Criterion benchmark scaffold.
