// PBR mesh shader — Cook-Torrance/GGX/Fresnel-Schlick.
// Ported from prakash (f64 CPU) to WGSL (f32 GPU).

const PI: f32 = 3.14159265358979323846;

// ── Uniforms ────────────────────────────────────────────────────────────────

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    camera_pos: vec4<f32>,
}

struct LightUniforms {
    ambient_color: vec4<f32>,      // RGB + intensity in alpha
    light_direction: vec4<f32>,    // normalized direction (w unused)
    light_color: vec4<f32>,        // RGB + intensity in alpha
}

struct MaterialUniforms {
    base_color_factor: vec4<f32>,  // RGBA
    metallic: f32,
    roughness: f32,
    _pad0: f32,
    _pad1: f32,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> light: LightUniforms;
@group(0) @binding(2) var<uniform> material: MaterialUniforms;

@group(1) @binding(0) var t_base_color: texture_2d<f32>;
@group(1) @binding(1) var s_base_color: sampler;
@group(1) @binding(2) var t_brdf_lut: texture_2d<f32>;
@group(1) @binding(3) var s_brdf_lut: sampler;

// ── Vertex I/O ──────────────────────────────────────────────────────────────

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = camera.model * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    out.world_position = world_pos.xyz;
    out.world_normal = normalize((camera.model * vec4<f32>(in.normal, 0.0)).xyz);
    out.tex_coords = in.tex_coords;
    out.color = in.color;
    return out;
}

// ── PBR Functions (ported from prakash::pbr) ────────────────────────────────

// Fresnel-Schlick: F(θ) = F0 + (1 - F0) · (1 - cosθ)⁵
fn fresnel_schlick(f0: vec3<f32>, cos_theta: f32) -> vec3<f32> {
    let ct = clamp(cos_theta, 0.0, 1.0);
    let factor = pow(1.0 - ct, 5.0);
    return f0 + (vec3<f32>(1.0) - f0) * factor;
}

// GGX/Trowbridge-Reitz NDF: D(h) = α² / (π · ((n·h)²·(α²-1) + 1)²)
fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let ndh = clamp(n_dot_h, 0.0, 1.0);
    let denom = ndh * ndh * (a2 - 1.0) + 1.0;
    return a2 / (PI * denom * denom + 1e-7);
}

// Schlick-GGX geometry (single direction)
fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    let ndv = clamp(n_dot_v, 0.0, 1.0);
    return ndv / (ndv * (1.0 - k) + k + 1e-7);
}

// Smith geometry: G(l, v) = G1(n·v) · G1(n·l)
fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    return geometry_schlick_ggx(n_dot_v, roughness) * geometry_schlick_ggx(n_dot_l, roughness);
}

// ── Fragment Shader ─────────────────────────────────────────────────────────

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample base color
    let tex_color = textureSample(t_base_color, s_base_color, in.tex_coords);
    let base_color = tex_color * in.color * material.base_color_factor;
    let albedo = base_color.rgb;

    let metallic = material.metallic;
    let roughness = max(material.roughness, 0.04); // clamp to avoid division by zero

    // Surface vectors
    let n = normalize(in.world_normal);
    let v = normalize(camera.camera_pos.xyz - in.world_position);
    let l = -normalize(light.light_direction.xyz);
    let h = normalize(v + l);

    let n_dot_v = max(dot(n, v), 0.001);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_h = max(dot(n, h), 0.0);
    let h_dot_v = max(dot(h, v), 0.0);

    // F0: dielectrics ~0.04, metals use albedo
    let f0 = mix(vec3<f32>(0.04), albedo, metallic);

    // Cook-Torrance specular BRDF
    let d = distribution_ggx(n_dot_h, roughness);
    let f = fresnel_schlick(f0, h_dot_v);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    let specular = (d * g) * f / (4.0 * n_dot_v * max(n_dot_l, 0.001));

    // Energy conservation: diffuse is what's NOT reflected
    let k_s = f;
    let k_d = (vec3<f32>(1.0) - k_s) * (1.0 - metallic);

    // Lambert diffuse
    let diffuse = k_d * albedo / PI;

    // Direct lighting
    let light_radiance = light.light_color.rgb * light.light_color.a;
    let direct = (diffuse + specular) * light_radiance * n_dot_l;

    // Ambient (simple, non-IBL)
    let ambient = light.ambient_color.rgb * light.ambient_color.a * albedo;

    let color = ambient + direct;

    // Simple Reinhard tone mapping
    let mapped = color / (color + vec3<f32>(1.0));

    return vec4<f32>(mapped, base_color.a);
}
