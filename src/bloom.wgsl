// Bloom post-processing — two passes:
// Pass 1 (threshold): extract bright pixels
// Pass 2 (blur): Gaussian blur on extracted brights
// Composite is done in postprocess.wgsl by sampling bloom texture

struct BloomUniforms {
    // x = threshold, y = soft_threshold, z = intensity, w = pass (0=threshold, 1=blur_h, 2=blur_v)
    params: vec4<f32>,
    // x = texel_width, y = texel_height
    texel_size: vec4<f32>,
}

@group(0) @binding(0) var t_input: texture_2d<f32>;
@group(0) @binding(1) var s_input: sampler;
@group(0) @binding(2) var<uniform> uniforms: BloomUniforms;

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

// Luminance for threshold
fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

@fragment
fn fs_threshold(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_input, s_input, in.tex_coords).rgb;
    let lum = luminance(color);
    let threshold = uniforms.params.x;
    let soft = uniforms.params.y;

    // Soft knee threshold
    let contribution = clamp((lum - threshold + soft) / (2.0 * soft + 1e-7), 0.0, 1.0);
    let bright = color * contribution;
    return vec4<f32>(bright, 1.0);
}

// 9-tap Gaussian blur weights
const WEIGHTS: array<f32, 5> = array<f32, 5>(
    0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216
);

@fragment
fn fs_blur_h(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = vec2<f32>(uniforms.texel_size.x, 0.0);
    var result = textureSample(t_input, s_input, in.tex_coords).rgb * WEIGHTS[0];
    for (var i = 1; i < 5; i++) {
        let offset = texel * f32(i);
        result += textureSample(t_input, s_input, in.tex_coords + offset).rgb * WEIGHTS[i];
        result += textureSample(t_input, s_input, in.tex_coords - offset).rgb * WEIGHTS[i];
    }
    return vec4<f32>(result, 1.0);
}

@fragment
fn fs_blur_v(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = vec2<f32>(0.0, uniforms.texel_size.y);
    var result = textureSample(t_input, s_input, in.tex_coords).rgb * WEIGHTS[0];
    for (var i = 1; i < 5; i++) {
        let offset = texel * f32(i);
        result += textureSample(t_input, s_input, in.tex_coords + offset).rgb * WEIGHTS[i];
        result += textureSample(t_input, s_input, in.tex_coords - offset).rgb * WEIGHTS[i];
    }
    return vec4<f32>(result, 1.0);
}
