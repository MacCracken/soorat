// PBR mesh shader — Cook-Torrance/GGX/Fresnel-Schlick.
// Multi-light support (up to 8), shadow mapping on primary directional.
// Ported from prakash (f64 CPU) to WGSL (f32 GPU).

const PI: f32 = 3.14159265358979323846;
const MAX_LIGHTS: u32 = 8u;

// Light type discriminators (matches lights.rs LightType enum)
const LIGHT_DIRECTIONAL: f32 = 0.0;
const LIGHT_POINT: f32 = 1.0;
const LIGHT_SPOT: f32 = 2.0;

// ── Uniforms ────────────────────────────────────────────────────────────────

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    camera_pos: vec4<f32>,
    // Inverse-transpose of upper-left 3x3 of model matrix for correct normals.
    // Stored as 3 vec4s (mat3 with padding) for uniform alignment.
    normal_matrix_0: vec4<f32>,
    normal_matrix_1: vec4<f32>,
    normal_matrix_2: vec4<f32>,
}

struct GpuLight {
    position_type: vec4<f32>,      // xyz = pos/dir, w = light type (0/1/2)
    direction_range: vec4<f32>,    // xyz = dir (spot), w = range
    color_intensity: vec4<f32>,    // rgb + intensity in alpha
    spot_params: vec4<f32>,        // x = inner cone cos, y = outer cone cos
}

struct LightArrayUniforms {
    ambient: vec4<f32>,            // rgb + intensity in alpha
    light_count: vec4<f32>,        // x = count
    lights: array<GpuLight, 8>,
}

struct MaterialUniforms {
    base_color_factor: vec4<f32>,
    metallic: f32,
    roughness: f32,
    _pad0: f32,
    _pad1: f32,
}

struct ShadowUniforms {
    light_view_proj: mat4x4<f32>,
    shadow_map_size: vec4<f32>,    // x = size
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> light_array: LightArrayUniforms;
@group(0) @binding(2) var<uniform> material: MaterialUniforms;
@group(0) @binding(3) var<uniform> shadow_uniforms: ShadowUniforms;

@group(1) @binding(0) var t_base_color: texture_2d<f32>;
@group(1) @binding(1) var s_base_color: sampler;

@group(2) @binding(0) var t_shadow_map: texture_depth_2d;
@group(2) @binding(1) var s_shadow_map: sampler_comparison;

// IBL (Image-Based Lighting) — optional, group 3
@group(3) @binding(0) var t_irradiance: texture_cube<f32>;
@group(3) @binding(1) var s_irradiance: sampler;
@group(3) @binding(2) var t_prefiltered: texture_cube<f32>;
@group(3) @binding(3) var s_prefiltered: sampler;
@group(3) @binding(4) var t_brdf_lut: texture_2d<f32>;
@group(3) @binding(5) var s_brdf_lut: sampler;

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
    @location(4) shadow_coords: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = camera.model * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    out.world_position = world_pos.xyz;

    // Use inverse-transpose normal matrix for correct normals under non-uniform scale
    let nm0 = camera.normal_matrix_0.xyz;
    let nm1 = camera.normal_matrix_1.xyz;
    let nm2 = camera.normal_matrix_2.xyz;
    out.world_normal = normalize(vec3<f32>(
        dot(nm0, in.normal),
        dot(nm1, in.normal),
        dot(nm2, in.normal),
    ));

    out.tex_coords = in.tex_coords;
    out.color = in.color;

    // Shadow coords from primary directional light
    let light_pos = shadow_uniforms.light_view_proj * world_pos;
    out.shadow_coords = vec3<f32>(
        light_pos.x * 0.5 + 0.5,
        -light_pos.y * 0.5 + 0.5,
        light_pos.z,
    );

    return out;
}

// ── PBR Functions (ported from prakash::pbr) ────────────────────────────────

fn fresnel_schlick(f0: vec3<f32>, cos_theta: f32) -> vec3<f32> {
    let ct = clamp(cos_theta, 0.0, 1.0);
    return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - ct, 5.0);
}

// Roughness-aware Fresnel for IBL ambient (Epic/UE4 variant).
// At grazing angles on rough surfaces, reduces specular reflection to avoid halo artifacts.
fn fresnel_schlick_roughness(f0: vec3<f32>, cos_theta: f32, roughness: f32) -> vec3<f32> {
    let ct = clamp(cos_theta, 0.0, 1.0);
    let max_reflect = max(vec3<f32>(1.0 - roughness), f0);
    return f0 + (max_reflect - f0) * pow(1.0 - ct, 5.0);
}

fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let ndh = clamp(n_dot_h, 0.0, 1.0);
    let denom = ndh * ndh * (a2 - 1.0) + 1.0;
    return a2 / (PI * denom * denom + 1e-7);
}

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    let ndv = clamp(n_dot_v, 0.0, 1.0);
    return ndv / (ndv * (1.0 - k) + k + 1e-7);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    return geometry_schlick_ggx(n_dot_v, roughness) * geometry_schlick_ggx(n_dot_l, roughness);
}

// ── Shadow sampling (PCF 3x3) ──────────────────────────────────────────────

fn sample_shadow(coords: vec3<f32>) -> f32 {
    if coords.x < 0.0 || coords.x > 1.0 || coords.y < 0.0 || coords.y > 1.0 || coords.z > 1.0 {
        return 1.0;
    }
    let shadow_size = max(shadow_uniforms.shadow_map_size.x, 512.0);
    let texel_size = 1.0 / shadow_size;
    var shadow = 0.0;
    for (var x: i32 = -1; x <= 1; x++) {
        for (var y: i32 = -1; y <= 1; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            shadow += textureSampleCompare(t_shadow_map, s_shadow_map, coords.xy + offset, coords.z);
        }
    }
    return shadow / 9.0;
}

// ── Lighting ────────────────────────────────────────────────────────────────

fn compute_light_contribution(
    light: GpuLight,
    world_pos: vec3<f32>,
    n: vec3<f32>,
    v: vec3<f32>,
    f0: vec3<f32>,
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
    n_dot_v: f32,
) -> vec3<f32> {
    let light_type = light.position_type.w;
    let color = light.color_intensity.rgb;
    let intensity = light.color_intensity.a;

    // Compute light direction and attenuation
    var l: vec3<f32>;
    var attenuation = 1.0;

    if light_type == LIGHT_DIRECTIONAL {
        l = -normalize(light.position_type.xyz);
    } else {
        // Point or Spot
        let to_light = light.position_type.xyz - world_pos;
        let distance = length(to_light);
        l = to_light / max(distance, 1e-7);

        // Range-based attenuation
        let range = light.direction_range.w;
        if range > 0.0 {
            let ratio = clamp(1.0 - pow(distance / range, 4.0), 0.0, 1.0);
            attenuation = ratio * ratio / (distance * distance + 1.0);
        } else {
            attenuation = 1.0 / (distance * distance + 1.0);
        }

        // Spot cone falloff
        if light_type == LIGHT_SPOT {
            let spot_dir = normalize(light.direction_range.xyz);
            let cos_angle = dot(-l, spot_dir);
            let inner_cos = light.spot_params.x;
            let outer_cos = light.spot_params.y;
            let spot_factor = clamp((cos_angle - outer_cos) / (inner_cos - outer_cos + 1e-7), 0.0, 1.0);
            attenuation *= spot_factor * spot_factor;
        }
    }

    let h = normalize(v + l);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_h = max(dot(n, h), 0.0);
    let h_dot_v = max(dot(h, v), 0.0);

    if n_dot_l <= 0.0 {
        return vec3<f32>(0.0);
    }

    // Cook-Torrance specular
    let d = distribution_ggx(n_dot_h, roughness);
    let f = fresnel_schlick(f0, h_dot_v);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    let specular = (d * g) * f / (4.0 * n_dot_v * max(n_dot_l, 0.001));

    // Energy conservation
    let k_s = f;
    let k_d = (vec3<f32>(1.0) - k_s) * (1.0 - metallic);
    let diffuse = k_d * albedo / PI;

    let radiance = color * intensity * attenuation;
    return (diffuse + specular) * radiance * n_dot_l;
}

// ── Fragment Shader ─────────────────────────────────────────────────────────

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_base_color, s_base_color, in.tex_coords);
    let base_color = tex_color * in.color * material.base_color_factor;
    let albedo = base_color.rgb;

    let metallic = material.metallic;
    let roughness = max(material.roughness, 0.04);

    let n = normalize(in.world_normal);
    let v = normalize(camera.camera_pos.xyz - in.world_position);
    let n_dot_v = max(dot(n, v), 0.001);

    let f0 = mix(vec3<f32>(0.04), albedo, metallic);

    // ── IBL Ambient (split-sum approximation) ──────────────────────────────
    // Diffuse IBL: irradiance cubemap sampled by normal
    let irradiance = textureSample(t_irradiance, s_irradiance, n).rgb;
    let k_s_ambient = fresnel_schlick_roughness(f0, n_dot_v, roughness);
    let k_d_ambient = (vec3<f32>(1.0) - k_s_ambient) * (1.0 - metallic);
    let diffuse_ibl = k_d_ambient * albedo * irradiance;

    // Specular IBL: pre-filtered environment map sampled at roughness-based mip level + BRDF LUT
    let r = reflect(-v, n);
    let max_mip = 4.0; // pre-filtered cubemap mip levels (0=mirror, 4=rough)
    let prefiltered = textureSampleLevel(t_prefiltered, s_prefiltered, r, roughness * max_mip).rgb;
    let brdf = textureSample(t_brdf_lut, s_brdf_lut, vec2<f32>(n_dot_v, roughness)).rg;
    let specular_ibl = prefiltered * (k_s_ambient * brdf.x + brdf.y);

    let ambient_ibl = (diffuse_ibl + specular_ibl) * light_array.ambient.a;

    // Fallback: if ambient intensity is 0, IBL contributes nothing
    // If no IBL maps are bound, the cubemap samples return black → graceful fallback

    // ── Direct lighting ──────────────────────────────────────────────────
    var direct = vec3<f32>(0.0);
    let light_count = u32(light_array.light_count.x);

    for (var i = 0u; i < light_count && i < MAX_LIGHTS; i++) {
        var contribution = compute_light_contribution(
            light_array.lights[i],
            in.world_position,
            n, v, f0, albedo,
            metallic, roughness, n_dot_v,
        );

        // Apply shadow to primary directional light (index 0)
        if i == 0u && light_array.lights[0].position_type.w == LIGHT_DIRECTIONAL {
            contribution *= sample_shadow(in.shadow_coords);
        }

        direct += contribution;
    }

    let color = ambient_ibl + direct;

    // Output linear HDR — tone mapping handled by PostProcessPipeline
    return vec4<f32>(color, base_color.a);
}
