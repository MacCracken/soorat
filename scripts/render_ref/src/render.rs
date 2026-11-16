//! Scene renderers. Each scene is built only from soorat 1.2.0's public API
//! (plus `wgpu` resources that API takes as arguments) and read back with
//! `RenderTarget::read_pixels`.
//!
//! Every CPU-side byte buffer a scene hands to the GPU (vertices, indices,
//! uniforms, texels) is also recorded in [`Uploads`], so the Cyrius side can
//! check its uploads before it compares pixels.

use crate::scenes::{
    BloomScene, Camera, Env, EnvMapDef, Format, Generator, LightDef, LineOp, LineScene, LutDef,
    MeshScene, Part, ParticleScene, Render, Rgba, Scene, SceneFile, SpriteScene, SsaoScene,
    TextureDef,
};
use soorat::acoustic_render::{AcousticColorMode, AcousticVisParams, mode_pattern_to_mesh};
use soorat::em_render::{EmColorMode, EmVisParams, field_slice_2d_to_mesh};
use soorat::environment::{EnvironmentMap, IblBindGroup};
use soorat::fluid_render::shallow_water_to_mesh;
use soorat::hdr::{BloomPipeline, BloomTargets, BloomUniforms, HdrFramebuffer};
use soorat::mesh_pipeline::{CameraUniforms, DepthBuffer, Mesh, MeshDrawParams, MeshPipeline};
use soorat::thermo_render::{ThermalColorMode, ThermalVisParams, thermal_grid_to_mesh};
use soorat::{
    Color, GpuContext, GpuLight, GpuParticle, GpuParticleSystem, LightArrayUniforms, LineBatch,
    LinePipeline, MaterialUniforms, PostProcessPipeline, PostProcessUniforms, RenderTarget,
    ShadowMap, ShadowPassUniforms, ShadowPipeline, ShadowUniforms, SimParams, Sprite, SpriteBatch,
    SpritePipeline, SsaoPipeline, SsaoUniforms, Texture, ToneMapMode, UvRect, Vertex3D,
    batch_to_vertices, create_default_sampler,
};

const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// Size of the shadow map bound by scenes without a shadow pass. An empty
/// `ShadowPipeline::render` clears it to depth 1.0, so the PBR shader's
/// shadow test passes (wgpu zero-initializes depth, which would otherwise
/// shadow light 0 everywhere).
pub const UNSHADOWED_MAP_SIZE: u32 = 16;

/// The CPU-side bytes a scene uploads, in upload order.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct Uploads(pub Vec<(String, Vec<u8>)>);

impl Uploads {
    fn add(&mut self, prefix: &str, name: &str, bytes: &[u8]) {
        self.0.push((format!("{prefix}{name}"), bytes.to_vec()));
    }
}

/// One scene's readback (tightly packed RGBA8 rows, top row first).
#[derive(Clone, PartialEq, Eq)]
pub struct Rendered {
    pub rgba: Vec<u8>,
    pub uploads: Uploads,
}

type R<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn color(c: Rgba) -> Color {
    Color::new(c[0], c[1], c[2], c[3])
}

fn camera<'a>(file: &'a SceneFile, name: &str) -> R<&'a Camera> {
    file.cameras
        .get(name)
        .ok_or_else(|| format!("unknown camera {name:?}"))
}

/// Render one scene.
pub fn render_scene(ctx: &GpuContext, file: &SceneFile, scene: &Scene) -> R<Rendered> {
    let mut up = Uploads::default();
    let (w, h, fmt) = (scene.width, scene.height, scene.format);
    let rgba = match &scene.render {
        Render::Sprites(s) => sprites(ctx, s, w, h, fmt, &mut up)?,
        Render::Lines(s) => lines(ctx, file, s, w, h, fmt, &mut up)?,
        Render::Mesh(m) => {
            let target = RenderTarget::new(&ctx.device, w, h, fmt.wgpu());
            let depth = DepthBuffer::new(&ctx.device, w, h);
            mesh(ctx, file, m, &target.view, fmt.wgpu(), &depth, "", &mut up)?;
            target.read_pixels(&ctx.device, &ctx.queue).map_err(err)?
        }
        Render::BloomTonemap(s) => bloom_tonemap(ctx, file, s, w, h, fmt, &mut up)?,
        Render::Ssao(s) => ssao(ctx, file, s, w, h, fmt, &mut up)?,
        Render::Particles(s) => particles(ctx, s, w, h, fmt, &mut up)?,
    };
    let expected = (w as usize) * (h as usize) * 4;
    if rgba.len() != expected {
        return Err(format!(
            "readback is {} bytes, expected {expected}",
            rgba.len()
        ));
    }
    Ok(Rendered { rgba, uploads: up })
}

// ── Resources ───────────────────────────────────────────────────────────────

/// A texture and the texel bytes it was made from.
fn texture(ctx: &GpuContext, def: &TextureDef) -> R<(Texture, Vec<u8>, (u32, u32))> {
    let (device, queue) = (&ctx.device, &ctx.queue);
    let (bytes, w, h) = match def {
        TextureDef::WhitePixel => {
            let t = Texture::white_pixel(device, queue).map_err(err)?;
            return Ok((t, vec![255; 4], (1, 1)));
        }
        TextureDef::Checker {
            width,
            height,
            a,
            b,
        } => {
            let mut bytes = Vec::new();
            for y in 0..*height {
                for x in 0..*width {
                    bytes.extend_from_slice(if (x + y) % 2 == 0 { a } else { b });
                }
            }
            (bytes, *width, *height)
        }
        TextureDef::Rgba {
            width,
            height,
            texels,
        } => {
            if texels.len() != (*width as usize) * (*height as usize) {
                return Err(format!(
                    "texture: {} texels for {width}×{height}",
                    texels.len()
                ));
            }
            (texels.concat(), *width, *height)
        }
    };
    let t = Texture::from_rgba(device, queue, &bytes, w, h, "render_ref_texture").map_err(err)?;
    Ok((t, bytes, (w, h)))
}

/// An environment cube map and its six level-0 faces, concatenated.
fn env_map(ctx: &GpuContext, def: &EnvMapDef) -> R<(EnvironmentMap, Vec<u8>)> {
    let (device, queue) = (&ctx.device, &ctx.queue);
    let (faces, size): (Vec<Vec<u8>>, u32) = match def {
        EnvMapDef::Solid { rgba } => {
            let map = EnvironmentMap::solid_color(device, queue, *rgba).map_err(err)?;
            return Ok((map, rgba.repeat(6)));
        }
        EnvMapDef::FlatFaces { size, colors } => {
            let n = (*size as usize) * (*size as usize);
            (colors.iter().map(|c| c.repeat(n)).collect(), *size)
        }
        EnvMapDef::GradientFaces { size, base, step } => {
            let faces = base
                .iter()
                .map(|b| {
                    let mut face = Vec::new();
                    for y in 0..*size {
                        for x in 0..*size {
                            let r = (u32::from(b[0]) + step[0] * x).min(255) as u8;
                            let g = (u32::from(b[1]) + step[1] * y).min(255) as u8;
                            face.extend_from_slice(&[r, g, b[2], 255]);
                        }
                    }
                    face
                })
                .collect();
            (faces, *size)
        }
    };
    let refs: [&[u8]; 6] = [
        &faces[0], &faces[1], &faces[2], &faces[3], &faces[4], &faces[5],
    ];
    let map = EnvironmentMap::from_faces(device, queue, refs, size).map_err(err)?;
    Ok((map, faces.concat()))
}

fn brdf_lut(ctx: &GpuContext, def: &LutDef) -> R<(Texture, Vec<u8>)> {
    let (device, queue) = (&ctx.device, &ctx.queue);
    match def {
        LutDef::RgbaLinear {
            width,
            height,
            texels,
        } => {
            if texels.len() != (*width as usize) * (*height as usize) {
                return Err(format!(
                    "brdf_lut: {} texels for {width}×{height}",
                    texels.len()
                ));
            }
            let bytes = texels.concat();
            let t = Texture::from_rgba_linear(
                device,
                queue,
                &bytes,
                *width,
                *height,
                "render_ref_brdf_lut",
                create_default_sampler(device),
            )
            .map_err(err)?;
            Ok((t, bytes))
        }
        LutDef::Prakash { size, samples } => {
            let bytes = soorat::pbr_material::generate_brdf_lut(*size, *samples);
            let t = soorat::pbr_material::create_brdf_lut_texture(device, queue, *size, *samples)
                .map_err(err)?;
            Ok((t, bytes))
        }
    }
}

fn ibl(
    ctx: &GpuContext,
    pipeline: &MeshPipeline,
    env: &Env,
    prefix: &str,
    up: &mut Uploads,
) -> R<IblBindGroup> {
    let (irradiance, irr_bytes) = env_map(ctx, &env.irradiance)?;
    let (prefiltered, pre_bytes) = env_map(ctx, &env.prefiltered)?;
    let (lut, lut_bytes) = brdf_lut(ctx, &env.brdf_lut)?;
    up.add(prefix, "env.irradiance_faces", &irr_bytes);
    up.add(prefix, "env.prefiltered_faces", &pre_bytes);
    up.add(prefix, "env.brdf_lut_texels", &lut_bytes);
    Ok(IblBindGroup::new(
        &ctx.device,
        pipeline.ibl_bind_group_layout().map_err(err)?,
        &irradiance,
        &prefiltered,
        &lut,
    ))
}

// ── Geometry ────────────────────────────────────────────────────────────────

fn generate(g: &Generator) -> R<(Vec<Vertex3D>, Vec<u32>)> {
    Ok(match g {
        Generator::Sphere { segments, rings } => soorat::primitives::sphere(*segments, *rings),
        Generator::Cube {} => soorat::primitives::cube(),
        Generator::Plane { subdivisions } => soorat::primitives::plane(*subdivisions),
        Generator::Degenerate {} => {
            let v = Vertex3D {
                position: [0.0; 3],
                normal: [0.0, 1.0, 0.0],
                tex_coords: [0.0; 2],
                color: [1.0; 4],
            };
            (vec![v; 3], vec![0, 1, 2])
        }
        Generator::AcousticModePattern {
            frequency_hz,
            mode_indices,
            pattern_dimensions,
            pattern,
            width,
            depth,
            height_scale,
            color_mode,
            base_color,
            alpha,
        } => {
            let mode = goonj::integration::soorat::ModeVisualization {
                frequency_hz: *frequency_hz,
                mode_indices: *mode_indices,
                pattern: pattern.clone(),
                pattern_dimensions: *pattern_dimensions,
            };
            let params = AcousticVisParams {
                color_mode: match color_mode.as_str() {
                    "Pressure" => AcousticColorMode::Pressure,
                    "SignedPressure" => AcousticColorMode::SignedPressure,
                    "Energy" => AcousticColorMode::Energy,
                    other => return Err(format!("acoustic color_mode {other:?}")),
                },
                base_color: color(*base_color),
                alpha: *alpha,
            };
            mode_pattern_to_mesh(&mode, *width, *depth, *height_scale, &params)
        }
        Generator::ThermalGrid {
            values,
            dimensions,
            origin,
            spacing,
            min_temp,
            max_temp,
            y_pos,
            color_mode,
            base_color,
            alpha,
        } => {
            let grid = ushma::integration::soorat::ThermalGridVisualization {
                values: values.clone(),
                dimensions: *dimensions,
                origin: *origin,
                spacing: *spacing,
                min_temp: *min_temp,
                max_temp: *max_temp,
            };
            let params = ThermalVisParams {
                color_mode: match color_mode.as_str() {
                    "Temperature" => ThermalColorMode::Temperature,
                    "AlphaBlend" => ThermalColorMode::AlphaBlend,
                    other => return Err(format!("thermal color_mode {other:?}")),
                },
                base_color: color(*base_color),
                alpha: *alpha,
            };
            thermal_grid_to_mesh(&grid, *y_pos, &params)
        }
        Generator::ShallowWater {
            nx,
            ny,
            dx,
            water_height,
            height,
            y_scale,
            color: c,
        } => {
            let mut water =
                pravash::shallow::ShallowWater::new(*nx, *ny, *dx, *water_height).map_err(err)?;
            if height.len() != water.height.len() {
                return Err(format!(
                    "shallow_water: {} heights for a {nx}×{ny} grid",
                    height.len()
                ));
            }
            water.height.clone_from(height);
            shallow_water_to_mesh(&water, *y_scale, color(*c))
        }
        Generator::EmFieldSlice2d {
            values,
            dimensions,
            origin,
            spacing,
            component,
            step,
            y_pos,
            color_mode,
            base_color,
            alpha,
        } => {
            let slice = bijli::integration::soorat::FieldSlice2D {
                values: values.clone(),
                dimensions: *dimensions,
                origin: *origin,
                spacing: *spacing,
                component: component.clone(),
                step: *step,
            };
            let params = EmVisParams {
                color_mode: match color_mode.as_str() {
                    "Magnitude" => EmColorMode::Magnitude,
                    "Signed" => EmColorMode::Signed,
                    other => return Err(format!("em color_mode {other:?}")),
                },
                base_color: color(*base_color),
                alpha: *alpha,
            };
            field_slice_2d_to_mesh(&slice, *y_pos, &params)
        }
    })
}

/// Concatenate the parts into one mesh (indices offset by the vertices before them).
fn build_parts(parts: &[Part]) -> R<(Vec<Vertex3D>, Vec<u32>)> {
    let (mut verts, mut idx) = (Vec::new(), Vec::new());
    for (k, part) in parts.iter().enumerate() {
        let (mut v, i) = generate(&part.generator)?;
        if v.is_empty() || i.is_empty() {
            return Err(format!("part {k}: the generator returned an empty mesh"));
        }
        for vert in &mut v {
            for c in 0..3 {
                if let Some(s) = part.scale {
                    vert.position[c] *= s[c];
                }
                if let Some(t) = part.translate {
                    vert.position[c] += t[c];
                }
            }
            if let Some(col) = part.color {
                vert.color = col;
            }
        }
        let base = u32::try_from(verts.len()).map_err(err)?;
        idx.extend(i.iter().map(|&x| x + base));
        verts.extend(v);
    }
    Ok((verts, idx))
}

// ── Scenes ──────────────────────────────────────────────────────────────────

/// The PBR mesh pass: one `MeshPipeline::draw` that clears `color_view` and
/// `depth` (to 1.0) and draws the combined parts.
#[allow(clippy::too_many_arguments)]
fn mesh(
    ctx: &GpuContext,
    file: &SceneFile,
    m: &MeshScene,
    color_view: &wgpu::TextureView,
    color_format: wgpu::TextureFormat,
    depth: &DepthBuffer,
    prefix: &str,
    up: &mut Uploads,
) -> R<()> {
    let (device, queue) = (&ctx.device, &ctx.queue);
    let cam = camera(file, &m.camera)?;
    let pipeline = MeshPipeline::new(device, color_format).map_err(err)?;

    let (verts, idx) = build_parts(&m.parts)?;
    up.add(prefix, "mesh.vertices", bytemuck::cast_slice(&verts));
    up.add(prefix, "mesh.indices", bytemuck::cast_slice(&idx));
    let mesh = Mesh::new(device, &verts, &idx);

    let (albedo, albedo_bytes, _) = texture(ctx, &m.albedo)?;
    up.add(prefix, "albedo_texels", &albedo_bytes);
    let material_bg =
        albedo.bind_group(device, pipeline.material_bind_group_layout().map_err(err)?);

    let shadow_pipeline = ShadowPipeline::new(device);
    let shadow_map = match &m.shadow {
        Some(s) => {
            let map = ShadowMap::new(device, s.map_size);
            let light = soorat::shadow::directional_light_matrix_at(
                s.direction,
                s.center,
                s.extent,
                s.near,
                s.far,
            );
            let su = ShadowUniforms {
                light_view_proj: light,
                model: IDENTITY,
            };
            up.add(prefix, "shadow_uniforms", bytemuck::bytes_of(&su));
            shadow_pipeline.update_uniforms(queue, &su);
            shadow_pipeline.render(device, queue, &map, &[&mesh]);
            let spu = ShadowPassUniforms {
                light_view_proj: light,
                shadow_map_size: [s.map_size as f32, 0.0, 0.0, 0.0],
            };
            up.add(prefix, "shadow_pass_uniforms", bytemuck::bytes_of(&spu));
            pipeline.update_shadow_pass(queue, &spu);
            map
        }
        None => {
            let map = ShadowMap::new(device, UNSHADOWED_MAP_SIZE);
            shadow_pipeline.render(device, queue, &map, &[]);
            map
        }
    };
    let shadow_bg = pipeline
        .create_shadow_bind_group(device, &shadow_map)
        .map_err(err)?;
    let ibl = ibl(ctx, &pipeline, &m.env, prefix, up)?;

    let mut cu = CameraUniforms {
        view_proj: cam.view_proj,
        camera_pos: [cam.eye[0], cam.eye[1], cam.eye[2], 0.0],
        ..Default::default()
    };
    if let Some(model) = m.model {
        cu.set_model(model);
    }
    up.add(prefix, "camera_uniforms", bytemuck::bytes_of(&cu));
    pipeline.update_camera(queue, &cu);

    let mut lights = LightArrayUniforms::default();
    lights.set_ambient(m.lighting.ambient_color, m.lighting.ambient_intensity);
    let list: Vec<GpuLight> = m
        .lighting
        .lights
        .iter()
        .map(|l| match l {
            LightDef::Directional {
                direction,
                color,
                intensity,
            } => GpuLight::directional(*direction, *color, *intensity),
            LightDef::Point {
                position,
                color,
                intensity,
                range,
            } => GpuLight::point(*position, *color, *intensity, *range),
            LightDef::Spot {
                position,
                direction,
                color,
                intensity,
                range,
                inner_cone,
                outer_cone,
            } => GpuLight::spot(
                *position,
                *direction,
                *color,
                *intensity,
                *range,
                *inner_cone,
                *outer_cone,
            ),
        })
        .collect();
    lights.set_lights(&list);
    up.add(prefix, "light_array_uniforms", bytemuck::bytes_of(&lights));
    pipeline.update_lights(queue, &lights);

    let material = MaterialUniforms {
        base_color_factor: m.material.base_color_factor,
        metallic: m.material.metallic,
        roughness: m.material.roughness,
        ..Default::default()
    };
    up.add(prefix, "material_uniforms", bytemuck::bytes_of(&material));
    pipeline.update_material(queue, &material);

    pipeline.draw(
        device,
        queue,
        &MeshDrawParams {
            color_view,
            depth,
            mesh: &mesh,
            material_bind_group: &material_bg,
            shadow_bind_group: &shadow_bg,
            ibl_bind_group: &ibl.bind_group,
            clear_color: Some(color(m.clear)),
        },
    );
    Ok(())
}

fn sprites(
    ctx: &GpuContext,
    s: &SpriteScene,
    w: u32,
    h: u32,
    fmt: Format,
    up: &mut Uploads,
) -> R<Vec<u8>> {
    let (device, queue) = (&ctx.device, &ctx.queue);
    let target = RenderTarget::new(device, w, h, fmt.wgpu());
    let pipeline = SpritePipeline::new(device, fmt.wgpu()).map_err(err)?;
    pipeline.update_projection(queue, w as f32, h as f32);
    let (tex, bytes, (aw, ah)) = texture(ctx, &s.texture)?;
    up.add("", "texture_texels", &bytes);
    let bg = tex.bind_group(device, pipeline.texture_bind_group_layout().map_err(err)?);

    let mut batch = SpriteBatch::new();
    for d in &s.sprites {
        let mut sprite = Sprite::new(d.x, d.y, d.w, d.h)
            .with_color(color(d.color))
            .with_z_order(d.z_order)
            .with_rotation(d.rotation);
        if let Some([x, y, rw, rh]) = d.uv_pixel_rect {
            sprite = sprite.with_uv(UvRect::from_pixel_rect(x, y, rw, rh, aw, ah));
        }
        batch.push(sprite);
    }
    batch.sort_by_z();
    let (v, i) = batch_to_vertices(&batch);
    up.add("", "sprite.vertices", bytemuck::cast_slice(&v));
    up.add("", "sprite.indices", bytemuck::cast_slice(&i));
    pipeline.draw(
        device,
        queue,
        &target.view,
        &batch,
        &bg,
        Some(color(s.clear)),
    );
    target.read_pixels(device, queue).map_err(err)
}

fn lines(
    ctx: &GpuContext,
    file: &SceneFile,
    s: &LineScene,
    w: u32,
    h: u32,
    fmt: Format,
    up: &mut Uploads,
) -> R<Vec<u8>> {
    let (device, queue) = (&ctx.device, &ctx.queue);
    let target = RenderTarget::new(device, w, h, fmt.wgpu());
    let depth = DepthBuffer::new(device, w, h);
    mesh(
        ctx,
        file,
        &s.clear_pass,
        &target.view,
        fmt.wgpu(),
        &depth,
        "clear_pass.",
        up,
    )?;

    let cam = camera(file, &s.camera)?;
    let pipeline = LinePipeline::new(device, fmt.wgpu()).map_err(err)?;
    up.add("", "line.view_proj", bytemuck::cast_slice(&cam.view_proj));
    pipeline.update_view_proj(queue, &cam.view_proj);
    let mut batch = LineBatch::new();
    for op in &s.ops {
        match op {
            LineOp::Line { a, b, color: c } => batch.line(*a, *b, color(*c)),
            LineOp::Grid {
                half_extent,
                spacing,
                color: c,
            } => batch.grid(*half_extent, *spacing, color(*c)),
            LineOp::WireBox { min, max, color: c } => batch.wire_box(*min, *max, color(*c)),
            LineOp::WireSphere {
                center,
                radius,
                segments,
                color: c,
            } => batch.wire_sphere(*center, *radius, *segments, color(*c)),
            LineOp::WireCircle {
                center,
                radius,
                segments,
                color: c,
            } => batch.wire_circle(*center, *radius, *segments, color(*c)),
        }
    }
    up.add("", "line.vertices", bytemuck::cast_slice(&batch.vertices));
    pipeline.draw(device, queue, &target.view, &depth, &batch);
    target.read_pixels(device, queue).map_err(err)
}

fn bloom_tonemap(
    ctx: &GpuContext,
    file: &SceneFile,
    s: &BloomScene,
    w: u32,
    h: u32,
    fmt: Format,
    up: &mut Uploads,
) -> R<Vec<u8>> {
    let (device, queue) = (&ctx.device, &ctx.queue);
    let hdr = HdrFramebuffer::new(device, w, h);
    let depth = DepthBuffer::new(device, w, h);
    mesh(
        ctx,
        file,
        &s.hdr_pass,
        &hdr.view,
        HdrFramebuffer::FORMAT,
        &depth,
        "hdr_pass.",
        up,
    )?;

    let bloom = BloomPipeline::new(device, HdrFramebuffer::FORMAT)
        .ok_or_else(|| "BloomPipeline::new failed".to_string())?;
    let bright = HdrFramebuffer::new(device, w, h);
    let temp = HdrFramebuffer::new(device, w, h);
    let out = HdrFramebuffer::new(device, w, h);
    let hdr_bg = bloom
        .create_bind_group(device, &hdr.view, &hdr.sampler)
        .map_err(err)?;
    let bright_bg = bloom
        .create_bind_group(device, &bright.view, &bright.sampler)
        .map_err(err)?;
    let temp_bg = bloom
        .create_bind_group(device, &temp.view, &temp.sampler)
        .map_err(err)?;
    let bu = BloomUniforms::new(
        s.threshold,
        s.soft_threshold,
        s.bloom_uniform_intensity,
        w,
        h,
    );
    up.add("", "bloom_uniforms", bytemuck::bytes_of(&bu));
    bloom.update_uniforms(queue, &bu);
    bloom.render(
        device,
        queue,
        &BloomTargets {
            hdr_bind_group: &hdr_bg,
            bright_view: &bright.view,
            blur_temp_view: &temp.view,
            bloom_output_view: &out.view,
            bright_bind_group: &bright_bg,
            blur_temp_bind_group: &temp_bg,
        },
    );

    let target = RenderTarget::new(device, w, h, fmt.wgpu());
    let mut post = PostProcessPipeline::new(device, fmt.wgpu()).map_err(err)?;
    post.set_input(device, &hdr.view, &hdr.sampler)
        .map_err(err)?;
    post.set_bloom(device, &out.view, &out.sampler)
        .map_err(err)?;
    let tone_map = match s.tone_map.as_str() {
        "Reinhard" => ToneMapMode::Reinhard,
        "AcesFilmic" => ToneMapMode::AcesFilmic,
        other => return Err(format!("tone_map {other:?}")),
    };
    let pu = PostProcessUniforms::new(s.exposure, tone_map).with_bloom_intensity(s.bloom_intensity);
    up.add("", "postprocess_uniforms", bytemuck::bytes_of(&pu));
    post.update_uniforms(queue, &pu);
    post.render(device, queue, &target.view);
    target.read_pixels(device, queue).map_err(err)
}

fn ssao(
    ctx: &GpuContext,
    file: &SceneFile,
    s: &SsaoScene,
    w: u32,
    h: u32,
    fmt: Format,
    up: &mut Uploads,
) -> R<Vec<u8>> {
    let (device, queue) = (&ctx.device, &ctx.queue);
    let scene_format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let scratch = RenderTarget::new(device, w, h, scene_format);
    let depth = DepthBuffer::new(device, w, h);
    mesh(
        ctx,
        file,
        &s.scene_pass,
        &scratch.view,
        scene_format,
        &depth,
        "scene_pass.",
        up,
    )?;

    let cam = camera(file, &s.scene_pass.camera)?;
    let pipeline = SsaoPipeline::new(device, fmt.wgpu()).map_err(err)?;
    let depth_sampler = create_default_sampler(device);
    let normal = Texture::from_rgba_linear(
        device,
        queue,
        &s.normal_texel,
        1,
        1,
        "render_ref_normal",
        create_default_sampler(device),
    )
    .map_err(err)?;
    up.add("", "ssao.normal_texel", &s.normal_texel);
    let bg = pipeline
        .create_bind_group(
            device,
            &depth.view,
            &depth_sampler,
            &normal.view,
            &normal.sampler,
        )
        .map_err(err)?;
    let mut su = SsaoUniforms::new(s.radius, s.bias, s.intensity, s.sample_count);
    su.projection = cam.projection;
    su.inv_projection = cam.inv_projection;
    up.add("", "ssao_uniforms", bytemuck::bytes_of(&su));
    pipeline.update_uniforms(queue, &su);
    let target = RenderTarget::new(device, w, h, fmt.wgpu());
    pipeline.render(device, queue, &target.view, &bg);
    target.read_pixels(device, queue).map_err(err)
}

/// xorshift32 (13, 17, 5); a draw is `(state >> 8) · 2⁻²⁴`, exact in f32.
struct XorShift32(u32);

impl XorShift32 {
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x >> 8) as f32 * (1.0 / 16_777_216.0)
    }
}

const PARTICLE_FIELDS: [&str; 12] = [
    "position.x",
    "position.y",
    "position.z",
    "position.w",
    "velocity.x",
    "velocity.y",
    "velocity.z",
    "velocity.w",
    "color.r",
    "color.g",
    "color.b",
    "color.a",
];

fn field_index(name: &str) -> R<usize> {
    PARTICLE_FIELDS
        .iter()
        .position(|f| *f == name)
        .ok_or_else(|| format!("unknown particle field {name:?}"))
}

/// The initial particles: for each particle in order, every draw consumes one
/// random number r and sets `field = lo + r * (hi - lo)`; constants fill the rest.
fn initial_particles(s: &ParticleScene) -> R<Vec<GpuParticle>> {
    let seed = u32::from_str_radix(s.seed.trim_start_matches("0x"), 16).map_err(err)?;
    if seed == 0 {
        return Err("particle seed must be non-zero".into());
    }
    let mut set = [0u8; 12];
    let mut base = [0.0f32; 12];
    for c in &s.constants {
        let i = field_index(&c.field)?;
        set[i] += 1;
        base[i] = c.value;
    }
    let draws: Vec<(usize, f32, f32)> = s
        .draws
        .iter()
        .map(|d| Ok((field_index(&d.field)?, d.lo, d.hi)))
        .collect::<R<_>>()?;
    for (i, _, _) in &draws {
        set[*i] += 1;
    }
    if let Some(i) = set.iter().position(|&n| n != 1) {
        return Err(format!(
            "particle field {} is set {} times (must be exactly once)",
            PARTICLE_FIELDS[i], set[i]
        ));
    }
    let mut rng = XorShift32(seed);
    let mut out = Vec::with_capacity(s.count as usize);
    for _ in 0..s.count {
        let mut f = base;
        for &(i, lo, hi) in &draws {
            let r = rng.next();
            let span = hi - lo;
            f[i] = lo + r * span;
        }
        out.push(GpuParticle {
            position: [f[0], f[1], f[2], f[3]],
            velocity: [f[4], f[5], f[6], f[7]],
            color: [f[8], f[9], f[10], f[11]],
        });
    }
    Ok(out)
}

fn particles(
    ctx: &GpuContext,
    s: &ParticleScene,
    w: u32,
    h: u32,
    fmt: Format,
    up: &mut Uploads,
) -> R<Vec<u8>> {
    let (device, queue) = (&ctx.device, &ctx.queue);
    let initial = initial_particles(s)?;
    up.add("", "particles.initial", bytemuck::cast_slice(&initial));
    let mut system = GpuParticleSystem::new(device, s.capacity);
    system.upload(queue, &initial);
    // The same params every step (simulate_with_params re-derives
    // particle_count from the live count, which is this value).
    let params = SimParams {
        delta_time: s.delta_time,
        gravity_y: s.gravity_y,
        damping: s.damping,
        particle_count: system.live_count(),
    };
    up.add("", "particles.sim_params", bytemuck::bytes_of(&params));
    for _ in 0..s.steps {
        system.simulate_with_params(device, queue, params);
    }
    let instances = system.to_instance_data(device, queue);
    up.add(
        "",
        "particles.instances_after_sim",
        bytemuck::cast_slice(&instances),
    );

    let mut batch = SpriteBatch::new();
    for inst in &instances {
        let size = inst.model[0] * s.pixels_per_unit;
        let cx = s.pixel_origin[0] + inst.model[12] * s.pixels_per_unit;
        let cy = s.pixel_origin[1] - inst.model[13] * s.pixels_per_unit;
        let half = size * 0.5;
        batch.push(
            Sprite::new(cx - half, cy - half, size, size).with_color(Color::new(
                inst.color[0],
                inst.color[1],
                inst.color[2],
                inst.color[3],
            )),
        );
    }
    let (v, i) = batch_to_vertices(&batch);
    up.add("", "sprite.vertices", bytemuck::cast_slice(&v));
    up.add("", "sprite.indices", bytemuck::cast_slice(&i));

    let target = RenderTarget::new(device, w, h, fmt.wgpu());
    let pipeline = SpritePipeline::new(device, fmt.wgpu()).map_err(err)?;
    pipeline.update_projection(queue, w as f32, h as f32);
    let white = Texture::white_pixel(device, queue).map_err(err)?;
    let bg = white.bind_group(device, pipeline.texture_bind_group_layout().map_err(err)?);
    pipeline.draw(
        device,
        queue,
        &target.view,
        &batch,
        &bg,
        Some(color(s.clear)),
    );
    target.read_pixels(device, queue).map_err(err)
}
