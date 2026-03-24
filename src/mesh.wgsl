// 3D mesh vertex + fragment shader with basic lighting.

struct CameraUniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
}

struct LightUniforms {
    ambient_color: vec4<f32>,
    light_direction: vec4<f32>,
    light_color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniforms;

@group(0) @binding(1)
var<uniform> light: LightUniforms;

@group(1) @binding(0)
var t_base_color: texture_2d<f32>;
@group(1) @binding(1)
var s_base_color: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = camera.model * vec4<f32>(in.position, 1.0);
    out.clip_position = camera.view_proj * world_pos;
    // Transform normal by model matrix (assumes uniform scale)
    out.world_normal = normalize((camera.model * vec4<f32>(in.normal, 0.0)).xyz);
    out.tex_coords = in.tex_coords;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_base_color, s_base_color, in.tex_coords);
    let base_color = tex_color * in.color;

    // Ambient
    let ambient = light.ambient_color.rgb * light.ambient_color.a;

    // Lambertian diffuse
    let n_dot_l = max(dot(in.world_normal, -light.light_direction.xyz), 0.0);
    let diffuse = light.light_color.rgb * light.light_color.a * n_dot_l;

    let lit_color = base_color.rgb * (ambient + diffuse);
    return vec4<f32>(lit_color, base_color.a);
}
