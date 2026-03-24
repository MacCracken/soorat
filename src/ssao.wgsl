// Screen-Space Ambient Occlusion (SSAO).
// Samples depth buffer to estimate occlusion at each pixel.

struct SsaoUniforms {
    // x = radius, y = bias, z = intensity, w = sample_count
    params: vec4<f32>,
    // projection matrix for reconstructing view-space position from depth
    projection: mat4x4<f32>,
    // inverse projection for depth → view-space position
    inv_projection: mat4x4<f32>,
}

@group(0) @binding(0) var t_depth: texture_depth_2d;
@group(0) @binding(1) var s_depth: sampler;
@group(0) @binding(2) var t_normal: texture_2d<f32>;
@group(0) @binding(3) var s_normal: sampler;
@group(0) @binding(4) var<uniform> uniforms: SsaoUniforms;

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

// Reconstruct view-space position from depth
fn view_pos_from_depth(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = vec4<f32>(uv * 2.0 - 1.0, depth, 1.0);
    let view = uniforms.inv_projection * ndc;
    return view.xyz / view.w;
}

// Simple hash for pseudo-random sampling direction
fn hash(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let radius = uniforms.params.x;
    let bias = uniforms.params.y;
    let intensity = uniforms.params.z;
    let sample_count = i32(uniforms.params.w);

    let depth = textureSample(t_depth, s_depth, in.tex_coords);
    let pos = view_pos_from_depth(in.tex_coords, depth);
    let normal = textureSample(t_normal, s_normal, in.tex_coords).rgb * 2.0 - 1.0;

    var occlusion = 0.0;

    for (var i = 0; i < sample_count; i++) {
        // Generate sample direction using hash
        let fi = f32(i);
        let angle = hash(in.tex_coords + vec2<f32>(fi * 0.1, fi * 0.3)) * 6.2831;
        let r = hash(in.tex_coords + vec2<f32>(fi * 0.7, fi * 0.11)) * radius;
        let h = hash(in.tex_coords + vec2<f32>(fi * 0.13, fi * 0.37));

        let sample_offset = vec3<f32>(cos(angle) * r, sin(angle) * r, h * radius);
        // Orient sample to hemisphere around normal
        let sample_pos = pos + sign(dot(sample_offset, normal)) * sample_offset;

        // Project sample back to screen space
        let projected = uniforms.projection * vec4<f32>(sample_pos, 1.0);
        let sample_uv = (projected.xy / projected.w) * 0.5 + 0.5;

        let sample_depth = textureSample(t_depth, s_depth, vec2<f32>(sample_uv.x, 1.0 - sample_uv.y));
        let sample_z = view_pos_from_depth(sample_uv, sample_depth).z;

        let range_check = smoothstep(0.0, 1.0, radius / abs(pos.z - sample_z));
        if sample_z >= sample_pos.z + bias {
            occlusion += range_check;
        }
    }

    let sc = max(f32(sample_count), 1.0);
    occlusion = 1.0 - (occlusion / sc) * intensity;
    return vec4<f32>(occlusion, occlusion, occlusion, 1.0);
}
