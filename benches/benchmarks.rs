use criterion::{Criterion, black_box, criterion_group, criterion_main};
use soorat::color::Color;
use soorat::pipeline::{batch_to_vertices, batch_to_vertices_into};
use soorat::sprite::{Sprite, SpriteBatch};
use soorat::vertex::Vertex2D;

fn bench_color(c: &mut Criterion) {
    let mut group = c.benchmark_group("color");

    group.bench_function("from_hex", |b| {
        b.iter(|| Color::from_hex(black_box(0xFF6633FF)))
    });

    group.bench_function("from_rgba8", |b| {
        b.iter(|| {
            Color::from_rgba8(
                black_box(255),
                black_box(102),
                black_box(51),
                black_box(255),
            )
        })
    });

    group.bench_function("lerp", |b| {
        let a = Color::BLACK;
        let z = Color::WHITE;
        b.iter(|| black_box(&a).lerp(black_box(z), black_box(0.5)))
    });

    group.bench_function("luminance", |b| {
        let c = Color::CORNFLOWER_BLUE;
        b.iter(|| black_box(c).luminance())
    });

    group.bench_function("to_array", |b| {
        let c = Color::CORNFLOWER_BLUE;
        b.iter(|| black_box(c).to_array())
    });

    group.finish();
}

fn bench_sprite(c: &mut Criterion) {
    let mut group = c.benchmark_group("sprite");

    group.bench_function("create", |b| {
        b.iter(|| {
            Sprite::new(black_box(100.0), black_box(200.0), 64.0, 64.0)
                .with_color(Color::RED)
                .with_z_order(5)
        })
    });

    group.bench_function("create_full_builder", |b| {
        b.iter(|| {
            Sprite::new(black_box(100.0), black_box(200.0), 64.0, 64.0)
                .with_color(Color::RED)
                .with_rotation(black_box(1.57))
                .with_texture(black_box(42))
                .with_z_order(5)
        })
    });

    group.bench_function("center_bounds", |b| {
        let s = Sprite::new(100.0, 200.0, 64.0, 64.0);
        b.iter(|| {
            let _ = black_box(&s).center();
            let _ = black_box(&s).bounds();
        })
    });

    group.bench_function("batch_push_100", |b| {
        let mut batch = SpriteBatch::new();
        b.iter(|| {
            for i in 0..100 {
                batch.push(Sprite::new(black_box(i as f32), 0.0, 32.0, 32.0));
            }
            batch.clear();
        })
    });

    group.bench_function("batch_push_100_prealloc", |b| {
        let mut batch = SpriteBatch::with_capacity(100);
        b.iter(|| {
            for i in 0..100 {
                batch.push(Sprite::new(black_box(i as f32), 0.0, 32.0, 32.0));
            }
            batch.clear();
        })
    });

    group.bench_function("batch_sort_100", |b| {
        let mut batch = SpriteBatch::new();
        for i in 0..100 {
            batch.push(Sprite::new(0.0, 0.0, 32.0, 32.0).with_z_order(100 - i));
        }
        b.iter(|| {
            batch.sort_by_z();
        })
    });

    group.bench_function("batch_push_1000", |b| {
        let mut batch = SpriteBatch::new();
        b.iter(|| {
            for i in 0..1000 {
                batch.push(Sprite::new(black_box(i as f32), 0.0, 32.0, 32.0));
            }
            batch.clear();
        })
    });

    group.bench_function("batch_sort_1000", |b| {
        let mut batch = SpriteBatch::new();
        for i in 0..1000 {
            batch.push(Sprite::new(0.0, 0.0, 32.0, 32.0).with_z_order(1000 - i));
        }
        b.iter(|| {
            batch.sort_by_z();
        })
    });

    group.finish();
}

fn bench_vertex(c: &mut Criterion) {
    let mut group = c.benchmark_group("vertex");

    group.bench_function("bytemuck_cast_100", |b| {
        let verts: Vec<Vertex2D> = (0..100)
            .map(|i| Vertex2D {
                position: [i as f32, 0.0],
                tex_coords: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            })
            .collect();
        b.iter(|| {
            let bytes: &[u8] = bytemuck::cast_slice(black_box(&verts));
            black_box(bytes.len());
        })
    });

    group.bench_function("quad_generation_100", |b| {
        let sprites: Vec<Sprite> = (0..100)
            .map(|i| Sprite::new(i as f32 * 10.0, 0.0, 32.0, 32.0).with_color(Color::WHITE))
            .collect();
        b.iter(|| {
            let mut verts = Vec::with_capacity(400);
            for s in black_box(&sprites) {
                let c = s.color.to_array();
                verts.push(Vertex2D {
                    position: [s.x, s.y],
                    tex_coords: [0.0, 0.0],
                    color: c,
                });
                verts.push(Vertex2D {
                    position: [s.x + s.width, s.y],
                    tex_coords: [1.0, 0.0],
                    color: c,
                });
                verts.push(Vertex2D {
                    position: [s.x + s.width, s.y + s.height],
                    tex_coords: [1.0, 1.0],
                    color: c,
                });
                verts.push(Vertex2D {
                    position: [s.x, s.y + s.height],
                    tex_coords: [0.0, 1.0],
                    color: c,
                });
            }
            black_box(&verts);
        })
    });

    group.finish();
}

fn bench_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline");

    group.bench_function("batch_to_vertices_100", |b| {
        let mut batch = SpriteBatch::with_capacity(100);
        for i in 0..100 {
            batch.push(
                Sprite::new(i as f32 * 10.0, 0.0, 32.0, 32.0)
                    .with_color(Color::WHITE)
                    .with_z_order(100 - i),
            );
        }
        batch.sort_by_z();
        b.iter(|| batch_to_vertices(black_box(&batch)))
    });

    group.bench_function("batch_to_vertices_1000", |b| {
        let mut batch = SpriteBatch::with_capacity(1000);
        for i in 0..1000 {
            batch.push(
                Sprite::new(i as f32 * 10.0, 0.0, 32.0, 32.0)
                    .with_color(Color::WHITE)
                    .with_z_order(1000 - i),
            );
        }
        batch.sort_by_z();
        b.iter(|| batch_to_vertices(black_box(&batch)))
    });

    group.bench_function("batch_to_vertices_rotated_100", |b| {
        let mut batch = SpriteBatch::with_capacity(100);
        for i in 0..100 {
            batch.push(
                Sprite::new(i as f32 * 10.0, 0.0, 32.0, 32.0)
                    .with_color(Color::WHITE)
                    .with_rotation(i as f32 * 0.1),
            );
        }
        b.iter(|| batch_to_vertices(black_box(&batch)))
    });

    group.bench_function("batch_to_vertices_into_1000", |b| {
        let mut batch = SpriteBatch::with_capacity(1000);
        for i in 0..1000 {
            batch.push(
                Sprite::new(i as f32 * 10.0, 0.0, 32.0, 32.0)
                    .with_color(Color::WHITE)
                    .with_z_order(1000 - i),
            );
        }
        batch.sort_by_z();
        let mut verts = Vec::with_capacity(4000);
        let mut indices = Vec::with_capacity(6000);
        b.iter(|| batch_to_vertices_into(black_box(&batch), &mut verts, &mut indices))
    });

    group.finish();
}

fn bench_debug_draw(c: &mut Criterion) {
    use soorat::debug_draw::LineBatch;

    let mut group = c.benchmark_group("debug_draw");

    group.bench_function("wire_box_100", |b| {
        let mut batch = LineBatch::new();
        b.iter(|| {
            batch.clear();
            for i in 0..100 {
                let p = i as f32;
                batch.wire_box(
                    black_box([p, 0.0, 0.0]),
                    black_box([p + 1.0, 1.0, 1.0]),
                    Color::GREEN,
                );
            }
            black_box(batch.line_count());
        })
    });

    group.bench_function("wire_sphere_100", |b| {
        let mut batch = LineBatch::new();
        b.iter(|| {
            batch.clear();
            for i in 0..100 {
                batch.wire_sphere(black_box([i as f32, 0.0, 0.0]), 1.0, 16, Color::BLUE);
            }
            black_box(batch.line_count());
        })
    });

    group.bench_function("grid_10x10", |b| {
        let mut batch = LineBatch::new();
        b.iter(|| {
            batch.clear();
            batch.grid(black_box(10.0), black_box(1.0), Color::WHITE);
            black_box(batch.line_count());
        })
    });

    group.finish();
}

fn bench_terrain(c: &mut Criterion) {
    use soorat::terrain::{TerrainConfig, flat_heightmap, generate_terrain};

    let mut group = c.benchmark_group("terrain");

    group.bench_function("generate_32x32", |b| {
        let cfg = TerrainConfig {
            width: 32,
            depth: 32,
            ..Default::default()
        };
        let heights = flat_heightmap(32, 32);
        b.iter(|| generate_terrain(black_box(&cfg), black_box(&heights)))
    });

    group.bench_function("generate_64x64", |b| {
        let cfg = TerrainConfig::default(); // 64x64
        let heights = flat_heightmap(64, 64);
        b.iter(|| generate_terrain(black_box(&cfg), black_box(&heights)))
    });

    group.finish();
}

fn bench_animation(c: &mut Criterion) {
    use soorat::animation::{Joint, Skeleton};

    let mut group = c.benchmark_group("animation");

    group.bench_function("compute_joints_16", |b| {
        let skeleton = Skeleton {
            joints: (0..16)
                .map(|i| Joint {
                    parent: if i > 0 { i as i32 - 1 } else { -1 },
                    ..Default::default()
                })
                .collect(),
        };
        b.iter(|| skeleton.compute_joint_matrices())
    });

    group.bench_function("compute_joints_64", |b| {
        let skeleton = Skeleton {
            joints: (0..64)
                .map(|i| Joint {
                    parent: if i > 0 { i as i32 - 1 } else { -1 },
                    ..Default::default()
                })
                .collect(),
        };
        b.iter(|| skeleton.compute_joint_matrices())
    });

    group.finish();
}

fn bench_shadow(c: &mut Criterion) {
    use soorat::shadow::{PointShadowMap, compute_practical_splits, directional_light_matrix};

    let mut group = c.benchmark_group("shadow");

    group.bench_function("directional_matrix", |b| {
        b.iter(|| directional_light_matrix(black_box([0.0, -1.0, -1.0]), 20.0, 0.1, 100.0))
    });

    group.bench_function("cascade_splits_4", |b| {
        b.iter(|| compute_practical_splits(black_box(0.1), black_box(500.0), 4, 0.5))
    });

    group.bench_function("point_shadow_6_faces", |b| {
        b.iter(|| PointShadowMap::new(black_box([0.0, 5.0, 0.0]), 0.1, 25.0))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_color,
    bench_sprite,
    bench_vertex,
    bench_pipeline,
    bench_debug_draw,
    bench_terrain,
    bench_animation,
    bench_shadow
);
criterion_main!(benches);
