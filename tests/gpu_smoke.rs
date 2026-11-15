//! GPU smoke tests: build every pipeline and render small offscreen frames.
//!
//! These need a real adapter. When none is available (e.g. headless CI) each
//! test prints a notice and returns early, so the suite still passes. wgpu
//! validation errors panic through the default uncaptured-error handler, so
//! any layout/shader/binding mismatch fails the test.

use soorat::environment::{EnvironmentMap, IblBindGroup};
use soorat::gpu_particles::{GpuParticle, GpuParticleSystem, SimParams};
use soorat::mesh_pipeline::{CameraUniforms, MeshDrawParams, ShadowPassUniforms};
use soorat::shadow::{ShadowUniforms, directional_light_matrix_at};
use soorat::*;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

fn gpu() -> Option<GpuContext> {
    match pollster::block_on(GpuContext::new()) {
        Ok(ctx) => Some(ctx),
        Err(e) => {
            eprintln!("skipping GPU smoke test: no adapter ({e})");
            None
        }
    }
}

fn pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * width + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

#[test]
fn sprite_pipeline_renders_tinted_quad() {
    let Some(ctx) = gpu() else { return };
    let (device, queue) = (&ctx.device, &ctx.queue);

    let target = RenderTarget::new(device, 32, 32, FORMAT);
    let pipeline = SpritePipeline::new(device, FORMAT).unwrap();
    pipeline.update_projection(queue, 32.0, 32.0);
    let white = Texture::white_pixel(device, queue).unwrap();
    let bind_group = white.bind_group(device, pipeline.texture_bind_group_layout().unwrap());

    let mut batch = SpriteBatch::new();
    batch.push(Sprite::new(8.0, 8.0, 16.0, 16.0).with_color(Color::RED));
    let stats = pipeline.draw(
        device,
        queue,
        &target.view,
        &batch,
        &bind_group,
        Some(Color::BLACK),
    );
    assert_eq!((stats.sprites, stats.draw_calls), (1, 1));

    let px = target.read_pixels(device, queue).unwrap();
    assert_eq!(pixel(&px, 32, 16, 16), [255, 0, 0, 255], "sprite center");
    assert_eq!(pixel(&px, 32, 2, 2), [0, 0, 0, 255], "cleared background");
}

#[test]
fn sprite_draw_batched_over_limit_does_not_overrun() {
    let Some(ctx) = gpu() else { return };
    let (device, queue) = (&ctx.device, &ctx.queue);

    let target = RenderTarget::new(device, 8, 8, FORMAT);
    let pipeline = SpritePipeline::new(device, FORMAT).unwrap();
    let white = Texture::white_pixel(device, queue).unwrap();
    let fallback = white.bind_group(device, pipeline.texture_bind_group_layout().unwrap());
    let cache = TextureCache::new();

    let mut batch = SpriteBatch::new();
    for i in 0..(batch::MAX_SPRITES_PER_BATCH + 500) {
        batch.push(Sprite::new(0.0, 0.0, 1.0, 1.0).with_texture((i % 3) as u64));
    }
    let stats = pipeline.draw_batched(
        device,
        queue,
        &pipeline::SpriteBatchDrawParams {
            view: &target.view,
            batch: &batch,
            texture_cache: &cache,
            fallback_bind_group: &fallback,
            clear_color: None,
        },
    );
    assert_eq!(stats.sprites as usize, batch::MAX_SPRITES_PER_BATCH);
    // Flush and surface any validation error.
    let _ = target.read_pixels(device, queue).unwrap();
}

#[test]
fn sprite_buffers_grow_across_frames() {
    let Some(ctx) = gpu() else { return };
    let (device, queue) = (&ctx.device, &ctx.queue);

    let target = RenderTarget::new(device, 8, 8, FORMAT);
    let pipeline = SpritePipeline::new(device, FORMAT).unwrap();
    let white = Texture::white_pixel(device, queue).unwrap();
    let bind_group = white.bind_group(device, pipeline.texture_bind_group_layout().unwrap());
    let mut buffers = SpriteBuffers::new(device, 1);
    let mut batch = SpriteBatch::new();
    for n in [1, 3, 7, 40] {
        while batch.len() < n {
            batch.push(Sprite::new(0.0, 0.0, 2.0, 2.0));
        }
        buffers.prepare(device, queue, &batch);
        let stats =
            pipeline.draw_with_buffers(device, queue, &target.view, &buffers, &bind_group, None);
        assert_eq!(stats.sprites as usize, n);
    }
    let _ = target.read_pixels(device, queue).unwrap();
}

#[test]
fn pbr_mesh_with_shadow_and_ibl_renders() {
    let Some(ctx) = gpu() else { return };
    let (device, queue) = (&ctx.device, &ctx.queue);

    let target = RenderTarget::new(device, 64, 64, FORMAT);
    let depth = DepthBuffer::new(device, 64, 64);
    let pipeline = MeshPipeline::new(device, FORMAT).unwrap();

    // Geometry: a sphere at the origin.
    let (verts, idx) = primitives::sphere(16, 8);
    let mesh = Mesh::new(device, &verts, &idx);

    // Shadow pass from a light centred on the scene.
    let light = directional_light_matrix_at([0.3, -1.0, 0.2], [0.0; 3], 2.0, -5.0, 5.0);
    let shadow_map = ShadowMap::new(device, 256);
    let shadow_pipeline = ShadowPipeline::new(device);
    shadow_pipeline.update_uniforms(
        queue,
        &ShadowUniforms {
            light_view_proj: light,
            ..Default::default()
        },
    );
    shadow_pipeline.render(device, queue, &shadow_map, &[&mesh]);
    pipeline.update_shadow_pass(
        queue,
        &ShadowPassUniforms {
            light_view_proj: light,
            shadow_map_size: [256.0, 0.0, 0.0, 0.0],
        },
    );

    // IBL: 4×4 cubemaps exercise the CPU mip-chain upload (3 levels).
    let face = [180u8; 4 * 4 * 4];
    let faces = [&face[..]; 6];
    let irradiance = EnvironmentMap::from_faces(device, queue, faces, 4).unwrap();
    let prefiltered = EnvironmentMap::from_faces(device, queue, faces, 4).unwrap();
    let brdf = Texture::from_rgba_linear(
        device,
        queue,
        &[128u8; 4 * 4 * 4],
        4,
        4,
        "brdf",
        create_default_sampler(device),
    )
    .unwrap();
    let ibl = IblBindGroup::new(
        device,
        pipeline.ibl_bind_group_layout().unwrap(),
        &irradiance,
        &prefiltered,
        &brdf,
    );

    let white = Texture::white_pixel(device, queue).unwrap();
    let material = white.bind_group(device, pipeline.material_bind_group_layout().unwrap());
    let shadow_bg = pipeline
        .create_shadow_bind_group(device, &shadow_map)
        .unwrap();

    // Camera looking at the origin; model rotated so the normal matrix matters.
    let mut camera = CameraUniforms {
        view_proj: [
            0.5, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0, -0.1, 0.0, 0.0, 0.0, 0.5, 1.0,
        ],
        camera_pos: [0.0, 0.0, 5.0, 0.0],
        ..Default::default()
    };
    camera.set_model(math_rotate_y(0.7));
    pipeline.update_camera(queue, &camera);
    pipeline.update_lights(queue, &LightArrayUniforms::default());

    pipeline.draw(
        device,
        queue,
        &MeshDrawParams {
            color_view: &target.view,
            depth: &depth,
            mesh: &mesh,
            material_bind_group: &material,
            shadow_bind_group: &shadow_bg,
            ibl_bind_group: &ibl.bind_group,
            clear_color: Some(Color::BLACK),
        },
    );

    let px = target.read_pixels(device, queue).unwrap();
    let center = pixel(&px, 64, 32, 32);
    assert_ne!(
        center,
        [0, 0, 0, 255],
        "sphere should cover the center pixel"
    );
    assert_eq!(
        pixel(&px, 64, 1, 1),
        [0, 0, 0, 255],
        "background stays clear"
    );
}

#[test]
fn post_processing_composites_bloom_when_bound() {
    // Black input, white bloom, intensity 1, exposure 1, Reinhard:
    // (0 + 1·1) / (1 + 1) = 0.5 with bloom, 0 without. Before the fix the
    // bloom entry point was unreachable, so the output was always 0.
    let Some(ctx) = gpu() else { return };
    let (device, queue) = (&ctx.device, &ctx.queue);
    let format = wgpu::TextureFormat::Rgba8Unorm;

    let black = Texture::from_color(device, queue, Color::BLACK).unwrap();
    let white = Texture::white_pixel(device, queue).unwrap();
    let output = RenderTarget::new(device, 4, 4, format);

    let mut post = PostProcessPipeline::new(device, format).unwrap();
    post.set_input(device, &black.view, &black.sampler).unwrap();
    post.update_uniforms(
        queue,
        &PostProcessUniforms::default().with_bloom_intensity(1.0),
    );

    post.render(device, queue, &output.view);
    let px = output.read_pixels(device, queue).unwrap();
    assert_eq!(pixel(&px, 4, 1, 1)[..3], [0, 0, 0], "no bloom bound");

    post.set_bloom(device, &white.view, &white.sampler).unwrap();
    post.render(device, queue, &output.view);
    let px = output.read_pixels(device, queue).unwrap();
    for c in &pixel(&px, 4, 1, 1)[..3] {
        assert!((127..=128).contains(c), "bloom composited: {c}");
    }

    post.clear_bloom();
    post.render(device, queue, &output.view);
    let px = output.read_pixels(device, queue).unwrap();
    assert_eq!(pixel(&px, 4, 1, 1)[..3], [0, 0, 0], "bloom cleared");
}

#[test]
fn post_processing_and_debug_pipelines_build_and_run() {
    let Some(ctx) = gpu() else { return };
    let (device, queue) = (&ctx.device, &ctx.queue);

    let hdr = HdrFramebuffer::new(device, 0, 0); // clamped to 1×1
    assert_eq!((hdr.width, hdr.height), (1, 1));
    let hdr = HdrFramebuffer::new(device, 16, 16);
    let output = RenderTarget::new(device, 16, 16, FORMAT);

    let mut post = PostProcessPipeline::new(device, FORMAT).unwrap();
    post.set_input(device, &hdr.view, &hdr.sampler).unwrap();
    post.update_uniforms(queue, &PostProcessUniforms::default());
    post.render(device, queue, &output.view);

    let bloom = BloomPipeline::new(device, HdrFramebuffer::FORMAT).expect("bloom pipeline");
    let bright = HdrFramebuffer::new(device, 16, 16);
    let temp = HdrFramebuffer::new(device, 16, 16);
    let out = HdrFramebuffer::new(device, 16, 16);
    let hdr_bg = bloom
        .create_bind_group(device, &hdr.view, &hdr.sampler)
        .unwrap();
    let bright_bg = bloom
        .create_bind_group(device, &bright.view, &bright.sampler)
        .unwrap();
    let temp_bg = bloom
        .create_bind_group(device, &temp.view, &temp.sampler)
        .unwrap();
    bloom.update_uniforms(queue, &BloomUniforms::new(1.0, 0.5, 0.3, 16, 16));
    bloom.render(
        device,
        queue,
        &hdr::BloomTargets {
            hdr_bind_group: &hdr_bg,
            bright_view: &bright.view,
            blur_temp_view: &temp.view,
            bloom_output_view: &out.view,
            bright_bind_group: &bright_bg,
            blur_temp_bind_group: &temp_bg,
        },
    );

    // SSAO reads a (now sampleable) depth buffer.
    let ssao = SsaoPipeline::new(device, wgpu::TextureFormat::R8Unorm).unwrap();
    let depth = DepthBuffer::new(device, 16, 16);
    let normals = HdrFramebuffer::new(device, 16, 16);
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    let ssao_bg = ssao
        .create_bind_group(
            device,
            &depth.view,
            &sampler,
            &normals.view,
            &normals.sampler,
        )
        .unwrap();
    let ao = RenderTarget::new(device, 16, 16, wgpu::TextureFormat::R8Unorm);
    ssao.update_uniforms(queue, &SsaoUniforms::default());
    ssao.render(device, queue, &ao.view, &ssao_bg);

    let lines = LinePipeline::new(device, FORMAT).unwrap();
    let mut batch = LineBatch::new();
    batch.wire_box([0.0; 3], [1.0; 3], Color::GREEN);
    lines.update_view_proj(
        queue,
        &[
            0.2, 0.0, 0.0, 0.0, 0.0, 0.2, 0.0, 0.0, 0.0, 0.0, 0.1, 0.0, 0.0, 0.0, 0.5, 1.0,
        ],
    );
    let line_depth = DepthBuffer::new(device, 16, 16);
    lines.draw(device, queue, &output.view, &line_depth, &batch);

    let _ = output.read_pixels(device, queue).unwrap();
}

#[test]
fn particle_system_simulates_and_reads_back() {
    let Some(ctx) = gpu() else { return };
    let (device, queue) = (&ctx.device, &ctx.queue);

    let mut system = GpuParticleSystem::new(device, 0); // clamped to capacity 1
    assert_eq!(system.capacity(), 1);
    system = GpuParticleSystem::new(device, 128);
    system.upload(queue, &vec![GpuParticle::default(); 100]);
    system.particle_count = 10_000; // out of range: must be clamped, not overrun
    system.simulate(device, queue, 1.0 / 60.0);
    let instances = system.to_instance_data(device, queue);
    assert!(instances.len() <= 128);
}

#[test]
fn particle_damping_is_frame_rate_independent() {
    // One step: x = v · damping^(dt·60) · dt. At dt = 1/30 that is damping²,
    // the decay of two 1/60 s steps. Before the fix the factor was `damping`
    // whatever dt was, so x was twice as large here.
    let Some(ctx) = gpu() else { return };
    let (device, queue) = (&ctx.device, &ctx.queue);

    for (dt, expected_factor) in [(1.0_f32 / 60.0, 0.5_f32), (1.0 / 30.0, 0.25), (0.0, 1.0)] {
        let mut system = GpuParticleSystem::new(device, 1);
        system.upload(
            queue,
            &[GpuParticle {
                position: [0.0, 0.0, 0.0, 10.0],
                velocity: [1.0, 0.0, 0.0, 0.1],
                color: [1.0; 4],
            }],
        );
        system.simulate_with_params(
            device,
            queue,
            SimParams {
                delta_time: dt,
                gravity_y: 0.0,
                damping: 0.5,
                particle_count: 1,
            },
        );
        let instances = system.to_instance_data(device, queue);
        assert_eq!(instances.len(), 1);
        let x = instances[0].model[12];
        let want = expected_factor * dt;
        assert!(
            (x - want).abs() <= 1e-6,
            "dt {dt}: x = {x}, expected {want}"
        );
    }
}

#[test]
fn skinned_pbr_shader_compiles_for_backend() {
    // No Rust pipeline builds pbr_skinned.wgsl yet; compile it with an
    // auto-derived layout so backend translation is exercised.
    let Some(ctx) = gpu() else { return };
    let device = &ctx.device;
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("pbr_skinned_smoke"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../src/pbr_skinned.wgsl").into()),
    });
    let _pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("pbr_skinned_smoke"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            buffers: &[SkinnedVertex3D::layout()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            targets: &[Some(FORMAT.into())],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DepthBuffer::FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let _ = device.poll(wgpu::PollType::Wait {
        timeout: None,
        submission_index: None,
    });
}

/// Column-major rotation about +Y.
fn math_rotate_y(angle: f32) -> [f32; 16] {
    let (s, c) = angle.sin_cos();
    [
        c, 0.0, -s, 0.0, 0.0, 1.0, 0.0, 0.0, s, 0.0, c, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}
