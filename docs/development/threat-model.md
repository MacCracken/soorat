# Threat Model

Soorat runs inside games, editors and visualization tools that load content
they did not author. The relevant failure modes are crashes (panics, wgpu
validation errors, which panic by default), unbounded allocation, and silent
corruption (NaN geometry, out-of-bounds GPU reads).

## Untrusted inputs

### glTF models (`gltf_loader`, `animation`)

- Parsed with `gltf::Gltf::from_slice`, which validates index references.
- Only embedded data is read (GLB BIN chunk, buffer-view images); external
  URIs are never fetched.
- Image buffer-view ranges use checked addition and `get()`; invalid ranges
  produce an empty slot (white fallback texture).
- Primitives are skipped unless they are triangle lists with positions and all
  indices `< vertex_count`, which prevents out-of-bounds vertex fetches on the GPU.
- Animation channels that target nodes outside the skin are dropped (previously
  they animated joint 0). Joint hierarchies may be unsorted or cyclic; cycles
  are broken, never looped.
- Residual risk: the `gltf` crate itself has debug-build arithmetic panics on
  some malformed accessors (see dependency-watch).

### Images (`texture`)

- Decoded by `image::load_from_memory` (default allocation limits apply).
- Dimensions are validated against the device's `max_texture_dimension_2d`
  before `create_texture`, and buffer sizes are overflow-checked.

### Simulation data (`acoustic_render`, `em_render`, `thermo_render`, `fluid_render`)

These structs are `Deserialize` with public fields, so headers can disagree
with their data.

- A grid must be fully backed by its data (`product(dims) <= len`, overflow-checked)
  or nothing is generated. Output size therefore stays proportional to input size.
- NaN/±inf samples map to neutral colors and finite positions; normalization
  ignores non-finite values.
- Tessellation counts are clamped (1024 segments), and quad counts are kept
  within `u32` index range.
- Stored maxima (`max_magnitude`, `max_gain`) are re-measured when unusable, and
  arrow lengths are clamped.

## GPU resource limits

- Zero-size textures are clamped to 1×1 (`DepthBuffer`, `HdrFramebuffer`,
  `ShadowMap`, `ShadowAtlas`); zero-size environment maps are rejected.
- Particle capacity is clamped to `1..=MAX_PARTICLES` (dispatch limit), and the
  live count is clamped to capacity before dispatch and readback.
- Sprite draws are limited to the u16 index range (`MAX_SPRITES_PER_BATCH`).

## Out of scope

- Shader-level denial of service (very large draw counts or resolutions chosen by the host).
- `screenshot` file paths and clipboard (the caller chooses them).
- `drm` ioctls (read-only, via agnosys).
