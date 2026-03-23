use criterion::{Criterion, black_box, criterion_group, criterion_main};
use soorat::color::Color;
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

criterion_group!(benches, bench_color, bench_sprite, bench_vertex);
criterion_main!(benches);
