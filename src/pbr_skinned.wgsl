// PBR skinned mesh shader — vertex skinning + normal mapping.
// Extends pbr.wgsl with joint palette and TBN normal mapping.

const PI: f32 = 3.14159265358979323846;
const MAX_LIGHTS: u32 = 8u;
const MAX_JOINTS: u32 = 128u;

const LIGHT_DIRECTIONAL: f32 = 0.0;
const LIGHT_POINT: f32 = 1.0;
const LIGHT_SPOT: f32 = 2.0;

// ── Uniforms ────────────────────────────────────────────────────────────────

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    camera_pos: vec4<f32>,
    normal_matrix_0: vec4<f32>,
    normal_matrix_1: vec4<f32>,
    normal_matrix_2: vec4<f32>,
}

struct GpuLight {
    position_type: vec4<f32>,
    direction_range: vec4<f32>,
    color_intensity: vec4<f32>,
    spot_params: vec4<f32>,
}

struct LightArrayUniforms {
    ambient: vec4<f32>,
    light_count: vec4<f32>,
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
    shadow_map_size: vec4<f32>,
}

struct JointUniforms {
    joint_count: vec4<f32>,
    joints: array<mat4x4<f32>, 128>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniforms;
@group(0) @binding(1) var<uniform> light_array: LightArrayUniforms;
@group(0) @binding(2) var<uniform> material: MaterialUniforms;
@group(0) @binding(3) var<uniform> shadow_uniforms: ShadowUniforms;

@group(1) @binding(0) var t_base_color: texture_2d<f32>;
@group(1) @binding(1) var s_base_color: sampler;
@group(1) @binding(2) var t_normal_map: texture_2d<f32>;
@group(1) @binding(3) var s_normal_map: sampler;

@group(2) @binding(0) var t_shadow_map: texture_depth_2d;
@group(2) @binding(1) var s_shadow_map: sampler_comparison;

@group(3) @binding(0) var<storage, read> joint_palette: JointUniforms;

// ── Vertex I/O ──────────────────────────────────────────────────────────────

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) joints: vec4<u32>,
    @location(6) weights: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) shadow_coords: vec3<f32>,
    @location(5) world_tangent: vec3<f32>,
    @location(6) world_bitangent: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Vertex skinning: blend position and normal by joint weights
    var skinned_pos = vec4<f32>(0.0);
    var skinned_normal = vec3<f32>(0.0);
    var skinned_tangent = vec3<f32>(0.0);

    let joint_count = u32(joint_palette.joint_count.x);
    let total_weight = in.weights.x + in.weights.y + in.weights.z + in.weights.w;

    if total_weight > 0.0 && joint_count > 0u {
        for (var i = 0u; i < 4u; i++) {
            let w = in.weights[i];
            if w <= 0.0 { continue; }
            let joint_idx = min(in.joints[i], joint_count - 1u);
            let joint_mat = joint_palette.joints[joint_idx];
            skinned_pos += w * (joint_mat * vec4<f32>(in.position, 1.0));
            skinned_normal += w * (joint_mat * vec4<f32>(in.normal, 0.0)).xyz;
            skinned_tangent += w * (joint_mat * vec4<f32>(in.tangent.xyz, 0.0)).xyz;
        }
    } else {
        skinned_pos = vec4<f32>(in.position, 1.0);
        skinned_normal = in.normal;
        skinned_tangent = in.tangent.xyz;
    }

    let world_pos = camera.model * skinned_pos;
    out.clip_position = camera.view_proj * world_pos;
    out.world_position = world_pos.xyz;

    // Normal matrix transform
    let nm0 = camera.normal_matrix_0.xyz;
    let nm1 = camera.normal_matrix_1.xyz;
    let nm2 = camera.normal_matrix_2.xyz;
    let n = normalize(skinned_normal);
    out.world_normal = normalize(vec3<f32>(dot(nm0, n), dot(nm1, n), dot(nm2, n)));

    // TBN basis for normal mapping
    let t = normalize(skinned_tangent);
    out.world_tangent = normalize(vec3<f32>(dot(nm0, t), dot(nm1, t), dot(nm2, t)));
    out.world_bitangent = cross(out.world_normal, out.world_tangent) * in.tangent.w;

    out.tex_coords = in.tex_coords;
    out.color = in.color;

    let light_pos = shadow_uniforms.light_view_proj * world_pos;
    out.shadow_coords = vec3<f32>(
        light_pos.x * 0.5 + 0.5,
        -light_pos.y * 0.5 + 0.5,
        light_pos.z,
    );

    return out;
}

// ── PBR Functions ───────────────────────────────────────────────────────────

fn fresnel_schlick(f0: vec3<f32>, cos_theta: f32) -> vec3<f32> {
    let ct = clamp(cos_theta, 0.0, 1.0);
    return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - ct, 5.0);
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

fn sample_shadow(coords: vec3<f32>) -> f32 {
    if coords.x < 0.0 || coords.x > 1.0 || coords.y < 0.0 || coords.y > 1.0 || coords.z > 1.0 || coords.z < 0.0 {
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

    var l: vec3<f32>;
    var attenuation = 1.0;

    if light_type == LIGHT_DIRECTIONAL {
        l = -normalize(light.position_type.xyz);
    } else {
        let to_light = light.position_type.xyz - world_pos;
        let distance = length(to_light);
        l = to_light / max(distance, 1e-7);
        let range = light.direction_range.w;
        if range > 0.0 {
            let ratio = clamp(1.0 - pow(distance / range, 4.0), 0.0, 1.0);
            attenuation = ratio * ratio / (distance * distance + 1.0);
        } else {
            attenuation = 1.0 / (distance * distance + 1.0);
        }
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

    if n_dot_l <= 0.0 { return vec3<f32>(0.0); }

    let d = distribution_ggx(n_dot_h, roughness);
    let f = fresnel_schlick(f0, h_dot_v);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    let specular = (d * g) * f / max(4.0 * n_dot_v * n_dot_l, 0.001);
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

    // Normal mapping: sample normal map and transform from tangent space to world space
    let normal_sample = textureSample(t_normal_map, s_normal_map, in.tex_coords).rgb;
    let tangent_normal = normal_sample * 2.0 - vec3<f32>(1.0);

    let t = normalize(in.world_tangent);
    let b = normalize(in.world_bitangent);
    let ng = normalize(in.world_normal);
    let n = normalize(t * tangent_normal.x + b * tangent_normal.y + ng * tangent_normal.z);

    let v = normalize(camera.camera_pos.xyz - in.world_position);
    let n_dot_v = max(dot(n, v), 0.001);
    let f0 = mix(vec3<f32>(0.04), albedo, metallic);

    let ambient = light_array.ambient.rgb * light_array.ambient.a * albedo;

    var direct = vec3<f32>(0.0);
    let light_count = u32(light_array.light_count.x);
    for (var i = 0u; i < light_count && i < MAX_LIGHTS; i++) {
        var contribution = compute_light_contribution(
            light_array.lights[i], in.world_position,
            n, v, f0, albedo, metallic, roughness, n_dot_v,
        );
        if i == 0u && light_array.lights[0].position_type.w == LIGHT_DIRECTIONAL {
            contribution *= sample_shadow(in.shadow_coords);
        }
        direct += contribution;
    }

    let color = ambient + direct;
    return vec4<f32>(color, base_color.a);
}
