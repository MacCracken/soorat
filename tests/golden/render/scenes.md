# Class-G reference scenes

The render-reference corpus for parity class **G** (`docs/development/soorat-port-plan.md` §2.2, §4 step 6, §5).
`scripts/render_ref` renders these scenes offscreen with the frozen Rust soorat 1.2.0 oracle (`rust-old/`),
and the Cyrius port renders the same scenes through mabda and compares pixels, on the same machine.

| File | What it is |
|---|---|
| `scenes.json` | **The definition.** Every input value lives here, and `render_ref` reads it, so what is documented is what renders. Hand-maintained. |
| `scenes.md` | This file: what each field means and the exact call sequence. It holds no values. |
| `<scene>.png` | The readback: lossless RGBA8, rows top to bottom, written by `render_ref`. |
| `manifest.tsv` | The platform header, then `scene width height format sha256 description`. `sha256` is over the raw RGBA bytes, not the PNG file. |
| `uploads.tsv` | Every CPU-side buffer each scene hands to the GPU, in upload order: `scene upload bytes sha256 hex`. `hex` holds the bytes when there are at most 256 of them, otherwise `-`. |

## Conventions

- **Numbers.** Every number in `scenes.json` is exactly representable as an f32. Integers are at most 2^24.
  Non-dyadic values are written as the *exact* decimal expansion of the f32, for example `0.707106769084930419921875`.
  A correctly rounding parser therefore recovers the same bits. `render_ref` refuses a file that breaks this rule.
  Seeds are hex strings.
- **Matrices** are column-major: index = col·4 + row, translation in 12..14. The convention is right-handed, with
  wgpu clip depth in [0, 1].
- **Colors.** `[r, g, b, a]` f32 values are linear and go through `Color::new`. `[u8; 4]` values are texels.
- **Cube faces** are ordered +X, −X, +Y, −Y, +Z, −Z. Face texels are row-major, top row first.
- **CPU arithmetic.** Where the harness computes something itself (part transforms, particle draws, the particle →
  sprite mapping), each operation is one IEEE f32 operation with round-to-nearest-even. There is no FMA contraction:
  Rust never contracts. Expressions are evaluated in the order written.
- **Isolation.** Scenes render one after another on one device. Each scene creates its own pipelines, buffers and
  textures, and nothing carries over between scenes. No scene reads a texel that it has not first cleared or written.
- **Determinism.** `render_ref` renders every scene twice, on two separately created devices, and writes nothing unless
  both runs are byte-identical, uploads included. `--check` is a third run in a new process.

## `cameras`

`eye`, `target`, `fovy_deg`, `near` and `far` record the derivation from `derive_camera.py`: look-at with up = +Y,
aspect 1, and a perspective projection computed in f64, each entry rounded once to f32. The renderer uploads:

- `view_proj`, as `CameraUniforms.view_proj` and `LinePipeline::update_view_proj`;
- `eye`, as `CameraUniforms.camera_pos = [eye.x, eye.y, eye.z, 0]`;
- `projection` and `inv_projection`, into `SsaoUniforms` only.

On load, the matrices are checked against each other and against the derivation fields to about 1e-5. That only catches typos. The values stay the definition.

## The mesh pass (`mesh`, and the `clear_pass`, `hdr_pass` and `scene_pass` blocks)

One `MeshPipeline::draw` into a color view and a `DepthBuffer` of the scene size. In order:

1. `MeshPipeline::new(device, color_format)`. Its color format is the scene format, or `Rgba16Float` for `hdr_pass`
   and `Rgba8UnormSrgb` for `scene_pass`. The pipeline is fixed by 1.2.0: alpha blending, back-face culling (CCW
   front), depth `Less` with depth write, `Depth32Float`.
2. **Geometry.** Each entry of `parts` calls its generator, then optionally transforms the result:
   - **Generators:**
     - `sphere {segments, rings}` → `primitives::sphere`;
     - `cube {}` → `primitives::cube()`;
     - `plane {subdivisions}` → `primitives::plane`;
     - `degenerate {}` → three identical vertices: position 0, normal +Y, uv 0, color white. The indices are
       `[0, 1, 2]`. It rasterizes nothing, so the draw only clears;
     - the four science generators, described below.
   - **Transform:** when `scale` is present, `p[c] = p[c] * scale[c]`; then, when `translate` is present,
     `p[c] = p[c] + translate[c]`, for c = x, y, z. Normals are copied unchanged. When `color` is present, it replaces
     every vertex color. Parts are concatenated in order, and each part's indices are offset by the vertex count before
     it. `Mesh::new(device, &vertices, &indices)` makes the mesh (u32 indices).
3. **Albedo** (bind group 1): `albedo` → texture → `Texture::bind_group(material layout)`.
   - `"white_pixel"` is `Texture::white_pixel`, one sRGB texel `[255, 255, 255, 255]`.
   - `checker {width, height, a, b}`: texel (x, y) = `a` if (x + y) is even, else `b`.
   - `rgba {width, height, texels}`: row-major texels.

   Both of the last two go through `Texture::from_rgba`: `Rgba8UnormSrgb`, sampled with `create_default_sampler`
   (nearest, clamp-to-edge).
4. **Shadow** (bind group 2): `ShadowPipeline::new(device)`. The shadow pass is depth-only, culls back faces, uses
   depth `LessEqual` and depth bias constant 2, slope 2.0, and clears to 1.0.
   - `shadow` present:
     - `ShadowMap::new(device, map_size)`, with a comparison sampler (`LessEqual`, linear, clamp);
     - `L = shadow::directional_light_matrix_at(direction, center, extent, near, far)`;
     - `ShadowPipeline::update_uniforms(ShadowUniforms { light_view_proj: L, model: identity })`;
     - `ShadowPipeline::render(map, &[&mesh])`, the whole combined mesh with the identity model;
     - `MeshPipeline::update_shadow_pass(ShadowPassUniforms { light_view_proj: L, shadow_map_size: [map_size, 0, 0, 0] })`.
   - `shadow` null:
     - `ShadowMap::new(device, 16)`;
     - `ShadowPipeline::render(map, &[])`, which only clears it to 1.0. wgpu zero-initializes depth, and a zero map
       would shadow light 0 everywhere;
     - the shadow-pass uniforms keep their defaults: identity, size 2048.
   - Either way, the bind group comes from `create_shadow_bind_group(map)`.
5. **IBL** (bind group 3): `IblBindGroup::new(ibl layout, irradiance, prefiltered, brdf_lut)`.
   - `irradiance` and `prefiltered` are each one of:
     - `solid {rgba}` → `EnvironmentMap::solid_color(rgba)`, a 1×1 cube;
     - `flat_faces {size, colors}`: every texel of face f is `colors[f]`;
     - `gradient_faces {size, base, step}`: texel (x, y) of face f is
       `[min(255, base[f].r + step.x·x), min(255, base[f].g + step.y·y), base[f].b, 255]`, in integer arithmetic.

     The last two go through `EnvironmentMap::from_faces`: `Rgba8UnormSrgb` with the full mip chain, which 1.2.0 builds
     on the CPU with a 2×2 box filter in linear space. Sampling is linear/linear/linear.
   - `brdf_lut` is one of:
     - `rgba_linear {width, height, texels}` → `Texture::from_rgba_linear(…, create_default_sampler)`;
     - `prakash {size, samples}` → `pbr_material::create_brdf_lut_texture(size, samples)`: prakash's
       `integrate_brdf_lut` in f64, quantized to u8, `Rgba8Unorm`, linear sampler. `uploads.tsv` holds the sha256 of its
       texels, from `generate_brdf_lut(size, samples)`.
6. **Uniforms:**
   - `CameraUniforms { view_proj, camera_pos, ..Default }`, then `set_model(model)` when `model` is not null. That call
     computes the normal matrix on the CPU. A null `model` keeps the default identity model and normal matrix.
     → `update_camera`.
   - `LightArrayUniforms::default()`, then `set_ambient(ambient_color, ambient_intensity)`, then `set_lights([...])`, in
     order. → `update_lights`. The light constructors are:
     - `directional` → `GpuLight::directional(direction, color, intensity)`;
     - `point` → `GpuLight::point(position, color, intensity, range)`;
     - `spot` → `GpuLight::spot(position, direction, color, intensity, range, inner_cone, outer_cone)`, which takes cos
       of both cone angles on the CPU.

     The PBR shader applies the shadow map to light 0 when it is directional.
   - `MaterialUniforms { base_color_factor, metallic, roughness, _pad0: 0, _pad1: 0 }` → `update_material`.
7. `MeshPipeline::draw(MeshDrawParams { …, clear_color: Some(Color::new(clear)) })`. This clears color to `clear` and
   depth to 1.0, then draws.

## Scene kinds

### `sprites` (`sprite_grid`)

1. `SpritePipeline::new(format)` (alpha blending, no depth), then `update_projection(width, height)`.
2. `texture` → `Texture::from_rgba` → `bind_group`.
3. **The batch.** For each entry of `sprites`, in order:
   `Sprite::new(x, y, w, h).with_color(color).with_z_order(z_order).with_rotation(rotation)`, then
   `.with_uv(UvRect::from_pixel_rect(rect…, atlas_w, atlas_h))` when `uv_pixel_rect` is not null. Each sprite is
   pushed onto a `SpriteBatch`.
4. `SpriteBatch::sort_by_z()`, a stable sort.
5. `SpritePipeline::draw(view, batch, bind_group, Some(clear))`, with u16 indices.

Rotation is about the sprite center, with the CPU's `sin`/`cos` of the angle, and only when the angle is non-zero.

### `lines` (`debug_lines`)

1. Run `clear_pass` as a mesh pass. With a `degenerate` part, its only effect is clearing color to `clear` and depth
   to 1.0.
2. `LinePipeline::new(format)` (LineList, alpha blending, depth `LessEqual`, no depth write, color and depth loaded).
3. `update_view_proj(cameras[camera].view_proj)`.
4. Every op is applied to one `LineBatch`, in order:
   - `line {a, b, color}` → `line`;
   - `grid {half_extent, spacing, color}` → `grid`;
   - `wire_box {min, max, color}` → `wire_box`;
   - `wire_sphere {center, radius, segments, color}` → `wire_sphere`;
   - `wire_circle {center, radius, segments, color}` → `wire_circle`.
5. `LinePipeline::draw(view, depth, batch)`, using the depth buffer from step 1.

### `mesh` (`pbr_lit`, `shadow_mesh`, `ibl_sphere`, `science_*`)

The mesh pass into a scene-sized `RenderTarget` of the scene format, then `read_pixels`.

### `bloom_tonemap` (`post_bloom_tonemap`)

1. `hdr_pass`, as a mesh pass, into `HdrFramebuffer::new(w, h)` (`Rgba16Float`, linear sampler).
2. `BloomPipeline::new(Rgba16Float)`, with three more `HdrFramebuffer`s: bright, temp and out.
3. Bind groups from `create_bind_group` for (hdr), (bright) and (temp), each with its framebuffer's own view and
   sampler.
4. `update_uniforms(BloomUniforms::new(threshold, soft_threshold, bloom_uniform_intensity, w, h))`.
5. `render(BloomTargets { … })`: threshold → horizontal blur → vertical blur into out.
6. `PostProcessPipeline::new(format)`, `set_input(hdr.view, hdr.sampler)`, `set_bloom(out.view, out.sampler)`.
7. `update_uniforms(PostProcessUniforms::new(exposure, tone_map).with_bloom_intensity(bloom_intensity))`.
   `tone_map` is `Reinhard` or `AcesFilmic`.
8. `render(target)`: a full-screen triangle that composites bloom, then applies exposure and the tone map.

### `ssao` (`post_ssao`)

1. `scene_pass`, as a mesh pass, into a scratch `Rgba8UnormSrgb` target that is not recorded. Only its depth is used.
2. `SsaoPipeline::new(format)`.
3. The normal input is a 1×1 texture, `Texture::from_rgba_linear(normal_texel, create_default_sampler)`. The shader
   decodes it as `rgb·2 − 1`.
4. `create_bind_group(depth.view, create_default_sampler, normal.view, normal.sampler)`.
5. `SsaoUniforms::new(radius, bias, intensity, sample_count)`, with `projection` and `inv_projection` set from the
   scene_pass camera, then `update_uniforms`.
6. `render(target, bind_group)`, which clears to white.

1.2.0 has no normal-buffer pass, which is why the normal is a constant.

### `particles` (`gpu_particles`)

1. **Initial state.** Each of `count` particles is built in order. Each entry of `draws` takes the next random number
   `r` and sets `field = lo + r * (hi - lo)`, with `hi - lo` evaluated first. `constants` set the remaining fields of
   `position.xyzw`, `velocity.xyzw` and `color.rgba`. Every field is set exactly once.
2. **The random numbers** are xorshift32, starting from state `seed`:
   - `x ^= x << 13; x ^= x >> 17; x ^= x << 5` (u32);
   - `r = f32(x >> 8) * 2^-24`, which is exact.
3. `GpuParticleSystem::new(capacity)`, then `upload(particles)`.
4. `steps` × `simulate_with_params(SimParams { delta_time, gravity_y, damping, particle_count: live_count })`. Each
   step is one compute dispatch. Life is `position.w` and size is `velocity.w`.
5. `to_instance_data()`: the live particles (life > 0), in buffer order.
6. Each instance becomes one sprite, in instance order:
   - `size = model[0] * pixels_per_unit`;
   - `cx = pixel_origin.x + model[12] * pixels_per_unit`;
   - `cy = pixel_origin.y - model[13] * pixels_per_unit`;
   - `half = size * 0.5`;
   - `Sprite::new(cx - half, cy - half, size, size).with_color(Color::new(color…))`.
7. Draw as in `sprites`, with `Texture::white_pixel` and `clear`, without sorting.

## Science generators

The inputs are small synthetic fields. Each `values`/`pattern`/`height` array is row-major, as `[row][col]`
(`index = row·ncols + col`), and every entry is a dyadic rational.

| Generator | Call | Data in this corpus (the formula that produced the JSON; the JSON is the definition) |
|---|---|---|
| `acoustic_mode_pattern` | `goonj::integration::soorat::ModeVisualization { frequency_hz, mode_indices, pattern, pattern_dimensions }` → `acoustic_render::mode_pattern_to_mesh(mode, width, depth, height_scale, AcousticVisParams { color_mode, base_color, alpha })` | 9×9, `pattern[iz][ix] = f[ix]·f[iz]`, where f = (1, 0.921875, 0.703125, 0.375, 0, −0.375, −0.703125, −0.921875, −1) approximates cos(πi/8): a (1, 0, 1) mode |
| `thermal_grid` | `ushma::integration::soorat::ThermalGridVisualization { values, dimensions, origin, spacing, min_temp, max_temp }` → `thermo_render::thermal_grid_to_mesh(grid, y_pos, ThermalVisParams { … })` | 8×8, `T = max(290, 350 − 2.5·((ix−5)² + (iy−2)²))` K |
| `shallow_water` | `pravash::shallow::ShallowWater::new(nx, ny, dx, water_height)`, then `height` replaced wholesale → `fluid_render::shallow_water_to_mesh(water, y_scale, color)` | 9×9, `h = 0.5 + 0.0625·max(0, 9 − d₁²) − 0.046875·max(0, 4 − d₂²)`, where d₁ is the distance to (5, 3) and d₂ to (2, 6). The part is translated by (0, −0.5, 0) |
| `em_field_slice_2d` | `bijli::integration::soorat::FieldSlice2D { values, dimensions, origin, spacing, component, step }` → `em_render::field_slice_2d_to_mesh(slice, y_pos, EmVisParams { … })` | 8×8 Ez, `(max(0, 8 − d_a²) − max(0, 8 − d_b²)) / 8`, where d_a is the distance to (2, 3) and d_b to (5, 4) |

## Where the CPU feeds the GPU

Class G is exact where the uploads are bit-identical (§5). `uploads.tsv` lets the Cyrius side check its uploads
before comparing pixels. The CPU paths that 1.2.0 computes, and that the port must reproduce, are listed below with
their expected class:

| Scene | CPU paths in its uploads |
|---|---|
| `sprite_grid` | quad generation (B); `UvRect::from_pixel_rect` in f64 (A); one rotated sprite, sin/cos of 0.5 (C) |
| `debug_lines` | `grid` (A/B); `wire_circle` and `wire_sphere` sin/cos (C) |
| `pbr_lit` | `sphere` vertices (E); `set_model` normal matrix (E); `GpuLight::spot` cone cosines (C) |
| `shadow_mesh` | `sphere` (E); `directional_light_matrix_at` (E); part transforms (B) |
| `ibl_sphere` | `sphere` (E); `set_model` (E); cube mip chains, using the sRGB table (A) and box filter (B); the prakash BRDF LUT (A) |
| `post_bloom_tonemap` | `sphere` (E); `BloomUniforms` texel size (B) |
| `post_ssao` | `sphere` (E); part transforms (B) |
| `gpu_particles` | xorshift draws (exact); the simulation runs on the GPU; the instance → sprite mapping (B) |
| `science_*` | the viz helpers: heat and signed colors, normalization (E) and height-field normals (B) |

## Skipped

`skinned_mesh` (a skinned mesh mid-animation): soorat 1.2.0 has no public skinned pipeline.
- `pbr_skinned.wgsl` is compiled only by the `gpu_smoke` test, against an auto-derived layout. No public type builds a
  pipeline from it or binds `JointUniforms`.
- `SkinnedVertex3D`, `Skeleton`, `AnimationClip` and `compute_joint_matrices` are CPU-only.

So no 1.2.0 reference image can exist. The joint-matrix math is CPU-side, so it belongs to the CPU golden corpus
(classes A–E). The Cyrius port's skinned pipeline, when it exists, has no class-G oracle.
