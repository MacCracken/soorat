// GPU particle simulation compute shader.
// Updates particle positions/velocities each frame.

struct Particle {
    position: vec4<f32>,   // xyz = position, w = life
    velocity: vec4<f32>,   // xyz = velocity, w = size
    color: vec4<f32>,      // rgba
}

struct SimParams {
    delta_time: f32,
    gravity_y: f32,
    damping: f32,
    particle_count: u32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: SimParams;

// `damping` is the velocity factor per 1/60 s reference step. Raising it to
// dt·60 makes the decay frame-rate independent; at dt = 1/60 it is `damping`.
fn damping_factor(damping: f32, dt: f32) -> f32 {
    if dt <= 0.0 {
        return 1.0;
    }
    if damping <= 0.0 {
        return 0.0;
    }
    return pow(damping, dt * 60.0);
}

@compute @workgroup_size(64)
fn cs_update(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if idx >= params.particle_count {
        return;
    }

    var p = particles[idx];

    // Skip dead particles
    if p.position.w <= 0.0 {
        return;
    }

    // Apply gravity
    p.velocity.y += params.gravity_y * params.delta_time;

    // Apply damping (frame-rate independent)
    let k = damping_factor(params.damping, params.delta_time);
    p.velocity = p.velocity * vec4<f32>(k, k, k, 1.0);

    // Integrate position
    p.position = vec4<f32>(
        p.position.xyz + p.velocity.xyz * params.delta_time,
        p.position.w - params.delta_time, // decrease life
    );

    particles[idx] = p;
}
