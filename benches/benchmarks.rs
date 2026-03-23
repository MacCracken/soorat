use criterion::{Criterion, black_box, criterion_group, criterion_main};
use soorat::color::Color;
use soorat::sprite::{Sprite, SpriteBatch};

fn bench_color(c: &mut Criterion) {
    let mut group = c.benchmark_group("color");

    group.bench_function("from_hex", |b| {
        b.iter(|| Color::from_hex(black_box(0xFF6633FF)))
    });

    group.bench_function("lerp", |b| {
        let a = Color::BLACK;
        let z = Color::WHITE;
        b.iter(|| black_box(&a).lerp(black_box(z), black_box(0.5)))
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

    group.bench_function("batch_push_100", |b| {
        let mut batch = SpriteBatch::new();
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

criterion_group!(benches, bench_color, bench_sprite);
criterion_main!(benches);
