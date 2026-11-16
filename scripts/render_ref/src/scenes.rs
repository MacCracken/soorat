//! The scene definitions: `tests/golden/render/scenes.json`, deserialized.
//!
//! The JSON is the definition the Cyrius side reproduces. Every number in it
//! must be exactly representable as an f32 (integers up to 2^24), so a
//! correctly rounding parser in any language recovers the same bits;
//! [`validate_numbers`] enforces that before anything is rendered.

use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub type Rgba = [f32; 4];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneFile {
    pub schema: u32,
    /// Free text for readers.
    #[allow(dead_code)]
    pub about: Vec<String>,
    pub cameras: BTreeMap<String, Camera>,
    pub scenes: Vec<Scene>,
    pub skipped: Vec<Skipped>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skipped {
    pub name: String,
    pub reason: String,
}

/// Column-major matrices (index = col * 4 + row). `view`, `fovy_deg` etc.
/// document the derivation; the renderer uploads `view_proj`, `eye` and (for
/// SSAO) `projection`/`inv_projection`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub fovy_deg: f32,
    pub near: f32,
    pub far: f32,
    pub view: [f32; 16],
    pub projection: [f32; 16],
    pub inv_projection: [f32; 16],
    pub view_proj: [f32; 16],
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum Format {
    Rgba8UnormSrgb,
    Rgba8Unorm,
}

impl Format {
    pub fn wgpu(self) -> wgpu::TextureFormat {
        match self {
            Self::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Rgba8UnormSrgb => "Rgba8UnormSrgb",
            Self::Rgba8Unorm => "Rgba8Unorm",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub name: String,
    pub description: String,
    pub width: u32,
    pub height: u32,
    /// Format of the recorded target (the readback is RGBA8 either way).
    pub format: Format,
    pub render: Render,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Render {
    Sprites(SpriteScene),
    Lines(LineScene),
    Mesh(MeshScene),
    BloomTonemap(BloomScene),
    Ssao(SsaoScene),
    Particles(ParticleScene),
}

// ── Textures and environment maps ───────────────────────────────────────────

/// An RGBA8 texture. `white_pixel` is `Texture::white_pixel`; the others go
/// through `Texture::from_rgba` (sRGB, the default nearest/clamp sampler).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TextureDef {
    WhitePixel,
    /// Texel (x, y) is `a` when (x + y) is even, `b` otherwise.
    Checker {
        width: u32,
        height: u32,
        a: [u8; 4],
        b: [u8; 4],
    },
    /// Row-major texels.
    Rgba {
        width: u32,
        height: u32,
        texels: Vec<[u8; 4]>,
    },
}

/// A cube map through `EnvironmentMap` (sRGB faces, CPU mip chain).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum EnvMapDef {
    /// `EnvironmentMap::solid_color(rgba)`.
    Solid { rgba: [u8; 4] },
    /// `from_faces`: every texel of face f is `colors[f]` (faces +X −X +Y −Y +Z −Z).
    FlatFaces { size: u32, colors: [[u8; 4]; 6] },
    /// `from_faces`: texel (x, y) of face f is
    /// `[min(255, base[f].r + step.x·x), min(255, base[f].g + step.y·y), base[f].b, 255]`
    /// (integer arithmetic), rows top to bottom.
    GradientFaces {
        size: u32,
        base: [[u8; 3]; 6],
        step: [u32; 2],
    },
}

/// The BRDF LUT bound in IBL group 3.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LutDef {
    /// `Texture::from_rgba_linear(texels, w, h, create_default_sampler)`.
    RgbaLinear {
        width: u32,
        height: u32,
        texels: Vec<[u8; 4]>,
    },
    /// `pbr_material::create_brdf_lut_texture(size, samples)` (prakash).
    Prakash { size: u32, samples: u32 },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Env {
    pub irradiance: EnvMapDef,
    pub prefiltered: EnvMapDef,
    pub brdf_lut: LutDef,
}

// ── Mesh scenes ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshScene {
    pub camera: String,
    /// `CameraUniforms::set_model(model)` when present; otherwise the
    /// `CameraUniforms::default()` model and normal matrix (identity).
    pub model: Option<[f32; 16]>,
    pub parts: Vec<Part>,
    pub albedo: TextureDef,
    pub material: MaterialDef,
    pub lighting: Lighting,
    pub env: Env,
    pub shadow: Option<ShadowDef>,
    pub clear: Rgba,
}

/// One generator call. When present, `scale`/`translate` map every position
/// p to `p * scale + translate` (per component, two rounded f32 ops, no FMA;
/// normals are copied unchanged), and `color` replaces every vertex color.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    pub generator: Generator,
    pub scale: Option<[f32; 3]>,
    pub translate: Option<[f32; 3]>,
    pub color: Option<Rgba>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Generator {
    /// `primitives::sphere(segments, rings)`.
    Sphere { segments: u32, rings: u32 },
    /// `primitives::cube()`.
    Cube {},
    /// `primitives::plane(subdivisions)`.
    Plane { subdivisions: u32 },
    /// Three identical vertices at the origin (normal +Y, uv 0, white) and
    /// indices [0, 1, 2]: rasterizes nothing, so the draw only clears.
    Degenerate {},
    /// `acoustic_render::mode_pattern_to_mesh`.
    AcousticModePattern {
        frequency_hz: f32,
        mode_indices: [u32; 3],
        pattern_dimensions: [usize; 2],
        pattern: Vec<f32>,
        width: f32,
        depth: f32,
        height_scale: f32,
        color_mode: String,
        base_color: Rgba,
        alpha: f32,
    },
    /// `thermo_render::thermal_grid_to_mesh`.
    ThermalGrid {
        values: Vec<f64>,
        dimensions: [usize; 2],
        origin: [f64; 2],
        spacing: [f64; 2],
        min_temp: f64,
        max_temp: f64,
        y_pos: f32,
        color_mode: String,
        base_color: Rgba,
        alpha: f32,
    },
    /// `ShallowWater::new(nx, ny, dx, water_height)`, then `height` replaced,
    /// then `fluid_render::shallow_water_to_mesh(water, y_scale, color)`.
    ShallowWater {
        nx: usize,
        ny: usize,
        dx: f64,
        water_height: f64,
        height: Vec<f64>,
        y_scale: f32,
        color: Rgba,
    },
    /// `em_render::field_slice_2d_to_mesh`.
    #[serde(rename = "em_field_slice_2d")]
    EmFieldSlice2d {
        values: Vec<f64>,
        dimensions: [usize; 2],
        origin: [f64; 2],
        spacing: f64,
        component: String,
        step: usize,
        y_pos: f32,
        color_mode: String,
        base_color: Rgba,
        alpha: f32,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialDef {
    pub base_color_factor: Rgba,
    pub metallic: f32,
    pub roughness: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lighting {
    /// `LightArrayUniforms::set_ambient(color, intensity)`.
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
    /// `LightArrayUniforms::set_lights`, in order (index 0 takes the shadow
    /// when it is directional).
    pub lights: Vec<LightDef>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LightDef {
    Directional {
        direction: [f32; 3],
        color: [f32; 3],
        intensity: f32,
    },
    Point {
        position: [f32; 3],
        color: [f32; 3],
        intensity: f32,
        range: f32,
    },
    Spot {
        position: [f32; 3],
        direction: [f32; 3],
        color: [f32; 3],
        intensity: f32,
        range: f32,
        inner_cone: f32,
        outer_cone: f32,
    },
}

/// A directional shadow pass: `directional_light_matrix_at(direction, center,
/// extent, near, far)` into a `ShadowMap::new(map_size)`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShadowDef {
    pub map_size: u32,
    pub direction: [f32; 3],
    pub center: [f32; 3],
    pub extent: f32,
    pub near: f32,
    pub far: f32,
}

// ── 2D, lines, post, particles ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpriteScene {
    pub clear: Rgba,
    pub texture: TextureDef,
    pub sprites: Vec<SpriteDef>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpriteDef {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: Rgba,
    pub z_order: i32,
    pub rotation: f32,
    /// `UvRect::from_pixel_rect(x, y, w, h, atlas_w, atlas_h)`; absent → `UvRect::FULL`.
    pub uv_pixel_rect: Option<[u32; 4]>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineScene {
    pub camera: String,
    /// Clears color and depth (to 1.0) before the lines, through the mesh pipeline.
    pub clear_pass: MeshScene,
    pub ops: Vec<LineOp>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum LineOp {
    Line {
        a: [f32; 3],
        b: [f32; 3],
        color: Rgba,
    },
    Grid {
        half_extent: f32,
        spacing: f32,
        color: Rgba,
    },
    WireBox {
        min: [f32; 3],
        max: [f32; 3],
        color: Rgba,
    },
    WireSphere {
        center: [f32; 3],
        radius: f32,
        segments: u32,
        color: Rgba,
    },
    WireCircle {
        center: [f32; 3],
        radius: f32,
        segments: u32,
        color: Rgba,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BloomScene {
    /// Rendered into an `HdrFramebuffer` (Rgba16Float) with a mesh pipeline of that format.
    pub hdr_pass: MeshScene,
    /// `BloomUniforms::new(threshold, soft_threshold, intensity, width, height)`.
    pub threshold: f32,
    pub soft_threshold: f32,
    pub bloom_uniform_intensity: f32,
    /// `PostProcessUniforms::new(exposure, tone_map).with_bloom_intensity(..)`.
    pub exposure: f32,
    pub tone_map: String,
    pub bloom_intensity: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SsaoScene {
    /// Its depth buffer is the SSAO input; its color target is not recorded.
    pub scene_pass: MeshScene,
    /// The 1×1 normal texture (`Texture::from_rgba_linear`, default sampler).
    pub normal_texel: [u8; 4],
    /// `SsaoUniforms { params: [radius, bias, intensity, sample_count], projection, inv_projection }`
    /// with the scene camera's `projection` and `inv_projection`.
    pub radius: f32,
    pub bias: f32,
    pub intensity: f32,
    pub sample_count: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticleScene {
    pub capacity: u32,
    pub count: u32,
    /// xorshift32 state, hex.
    pub seed: String,
    /// Per particle, in this order, each entry consumes one draw r and sets
    /// `field = lo + r * (hi - lo)`.
    pub draws: Vec<ParticleDraw>,
    /// Fields not drawn.
    pub constants: Vec<ParticleConst>,
    pub steps: u32,
    pub delta_time: f32,
    pub gravity_y: f32,
    pub damping: f32,
    /// Instance → sprite mapping (see scenes.md).
    pub pixel_origin: [f32; 2],
    pub pixels_per_unit: f32,
    pub clear: Rgba,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticleDraw {
    pub field: String,
    pub lo: f32,
    pub hi: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticleConst {
    pub field: String,
    pub value: f32,
}

// ── Loading ─────────────────────────────────────────────────────────────────

/// Parse and validate `scenes.json`.
pub fn load(text: &str) -> Result<SceneFile, String> {
    let value: Value = serde_json::from_str(text).map_err(|e| format!("scenes.json: {e}"))?;
    let mut bad = Vec::new();
    validate_numbers(&value, "$", &mut bad);
    if !bad.is_empty() {
        return Err(format!(
            "scenes.json: {} number(s) are not exactly representable as f32 \
             (write the exact decimal of the f32):\n  {}",
            bad.len(),
            bad.join("\n  ")
        ));
    }
    let file: SceneFile = serde_json::from_value(value).map_err(|e| format!("scenes.json: {e}"))?;
    if file.schema != 1 {
        return Err(format!("scenes.json: unsupported schema {}", file.schema));
    }
    for (name, cam) in &file.cameras {
        check_camera(cam).map_err(|e| format!("scenes.json: camera {name}: {e}"))?;
    }
    let mut names = std::collections::BTreeSet::new();
    for s in &file.scenes {
        if !names.insert(s.name.as_str()) {
            return Err(format!("scenes.json: duplicate scene {}", s.name));
        }
        if s.name.is_empty()
            || !s
                .name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err(format!(
                "scenes.json: scene name {:?} must be [a-z0-9_]+",
                s.name
            ));
        }
        if s.description.contains('\t') || s.description.contains('\n') {
            return Err(format!(
                "scenes.json: {}: description must be one line",
                s.name
            ));
        }
    }
    Ok(file)
}

/// Catch typos: the matrices must agree with each other and with the
/// derivation fields to f32 precision (the f32 entries were each rounded from
/// an f64 computation, so the checks allow a few ulp).
fn check_camera(c: &Camera) -> Result<(), String> {
    let m = |a: &[f32; 16], b: &[f32; 16]| -> [f64; 16] {
        let mut out = [0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                out[col * 4 + row] = (0..4)
                    .map(|k| f64::from(a[k * 4 + row]) * f64::from(b[col * 4 + k]))
                    .sum();
            }
        }
        out
    };
    let near = |what: &str, got: f64, want: f64| {
        if (got - want).abs() <= 1e-5 * (1.0 + want.abs()) {
            Ok(())
        } else {
            Err(format!("{what}: {got} vs {want}"))
        }
    };
    let pv = m(&c.projection, &c.view);
    for (i, (&got, &want)) in c.view_proj.iter().zip(&pv).enumerate() {
        near(
            &format!("view_proj[{i}] vs projection·view"),
            f64::from(got),
            want,
        )?;
    }
    let id = m(&c.projection, &c.inv_projection);
    for (i, &v) in id.iter().enumerate() {
        near(
            &format!("(projection·inv_projection)[{i}]"),
            v,
            if i % 5 == 0 { 1.0 } else { 0.0 },
        )?;
    }
    let (n, f) = (f64::from(c.near), f64::from(c.far));
    let g = 1.0 / (f64::from(c.fovy_deg).to_radians() / 2.0).tan();
    near("projection[0] vs fovy", f64::from(c.projection[0]), g)?;
    near(
        "projection[10] vs near/far",
        f64::from(c.projection[10]),
        f / (n - f),
    )?;
    near(
        "projection[14] vs near/far",
        f64::from(c.projection[14]),
        n * f / (n - f),
    )?;
    // The view maps the eye to the origin and the target onto -Z.
    let apply = |p: [f32; 3]| -> [f64; 3] {
        let v = &c.view;
        [0, 1, 2].map(|r| {
            (0..3)
                .map(|k| f64::from(v[k * 4 + r]) * f64::from(p[k]))
                .sum::<f64>()
                + f64::from(v[12 + r])
        })
    };
    let e = apply(c.eye);
    let t = apply(c.target);
    for (i, &x) in e.iter().enumerate() {
        near(&format!("view·eye[{i}]"), x, 0.0)?;
    }
    near("view·target x", t[0], 0.0)?;
    near("view·target y", t[1], 0.0)?;
    if t[2] >= 0.0 {
        return Err("the target is not in front of the camera".into());
    }
    Ok(())
}

/// Every float must survive f64 → f32 → f64 unchanged; integers must fit
/// f32's 24-bit significand.
fn validate_numbers(v: &Value, path: &str, bad: &mut Vec<String>) {
    match v {
        Value::Number(n) => {
            let ok = if let Some(i) = n.as_i64() {
                i.unsigned_abs() <= 1 << 24
            } else if let Some(u) = n.as_u64() {
                u <= 1 << 24
            } else {
                n.as_f64()
                    .is_some_and(|f| f.is_finite() && f64::from(f as f32) == f)
            };
            if !ok {
                bad.push(format!("{path} = {n}"));
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                validate_numbers(x, &format!("{path}[{i}]"), bad);
            }
        }
        Value::Object(o) => {
            for (k, x) in o {
                validate_numbers(x, &format!("{path}.{k}"), bad);
            }
        }
        _ => {}
    }
}
