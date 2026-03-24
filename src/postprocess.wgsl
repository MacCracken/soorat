// Full-screen post-processing shader.
// Tone mapping (Reinhard/ACES) + bloom composite.

struct PostProcessUniforms {
    // x = exposure, y = bloom_threshold, z = bloom_intensity, w = tone_map_mode (0=Reinhard, 1=ACES)
    params: vec4<f32>,
}

@group(0) @binding(0) var t_input: texture_2d<f32>;
@group(0) @binding(1) var s_input: sampler;
@group(0) @binding(2) var<uniform> uniforms: PostProcessUniforms;

// Bloom texture (optional — if not bound, bloom_intensity should be 0)
@group(1) @binding(0) var t_bloom: texture_2d<f32>;
@group(1) @binding(1) var s_bloom: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vertex_index) / 2) * 4.0 - 1.0;
    let y = f32(i32(vertex_index) % 2) * 4.0 - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.tex_coords = vec2<f32>((x + 1.0) / 2.0, (1.0 - y) / 2.0);
    return out;
}

fn aces_filmic(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_input, s_input, in.tex_coords).rgb;

    // Add bloom
    let bloom_intensity = uniforms.params.z;
    if bloom_intensity > 0.0 {
        let bloom = textureSample(t_bloom, s_bloom, in.tex_coords).rgb;
        color += bloom * bloom_intensity;
    }

    let exposure = uniforms.params.x;
    let tone_map_mode = u32(uniforms.params.w);

    // Apply exposure
    color = color * exposure;

    // Tone mapping
    if tone_map_mode == 1u {
        color = aces_filmic(color);
    } else {
        color = color / (color + vec3<f32>(1.0));
    }

    return vec4<f32>(color, 1.0);
}

// Simple pass-through for when bloom group isn't bound
@fragment
fn fs_no_bloom(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_input, s_input, in.tex_coords).rgb;

    let exposure = uniforms.params.x;
    let tone_map_mode = u32(uniforms.params.w);

    color = color * exposure;

    if tone_map_mode == 1u {
        color = aces_filmic(color);
    } else {
        color = color / (color + vec3<f32>(1.0));
    }

    return vec4<f32>(color, 1.0);
}
