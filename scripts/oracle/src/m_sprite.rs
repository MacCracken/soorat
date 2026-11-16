//! `sprite`: soorat 1.2.0's `sprite` module and the sprite → quad expansion (`batch`, through `pipeline`'s
//! re-exports), M2 part a.
//!
//! - `UvRect` (A), `Sprite::center` / `bounds` (B: f32 sums, expected 0 ulp), `Sprite::default` (A).
//! - `SpriteBatch::sort_by_z` (A): the stable permutation, as the original indices of the sorted sprites.
//! - `batch_to_vertices` / `_u32` (C): the vertices (8 f32 each) and the indices. A rotated sprite's corners go through
//!   f32 `sin`/`cos` (host glibc), so rows where glibc misrounds are in `sprite.cr.tsv` with the correctly rounded
//!   corners (`validate.py --write`). A sprite is 13 f32: x, y, width, height, rotation, color (4), uv (4).
//! - `batch_to_vertices_digest` / `_u32_digest` (A): batches of n sprites `Sprite::new(i, 0, 1, 1)` at and beyond the
//!   u16 limit, as the vertex and index counts and FNV-1a 64 digests of the vertex and index bytes (little-endian),
//!   each digest as two 32-bit halves: a row of 16,383 sprites' vertices would be 524,256 tokens.

use crate::{Module, Rng, Toks};
use soorat::color::Color;
use soorat::pipeline::{MAX_SPRITES_PER_BATCH, batch_to_vertices, batch_to_vertices_u32};
use soorat::sprite::{Sprite, SpriteBatch, UvRect};

fn sprite13(s: &Sprite) -> Vec<f32> {
    let mut v = vec![s.x, s.y, s.width, s.height, s.rotation];
    v.extend_from_slice(&s.color.to_array());
    v.extend_from_slice(&[s.uv.u_min, s.uv.v_min, s.uv.u_max, s.uv.v_max]);
    v
}

fn unit(r: &mut Rng) -> f32 {
    ((r.next() >> 41) as f32) / 8_388_608.0
}

fn fnv(bytes: impl Iterator<Item = u8>) -> (i128, i128) {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    ((h >> 32) as i128, (h & 0xffff_ffff) as i128)
}

fn random_sprite(r: &mut Rng, rot: f32) -> Sprite {
    let mut s = Sprite::new(r.f() * 64.0, r.f() * 64.0, r.pos() * 4.0, r.f() * 4.0)
        .with_rotation(rot)
        .with_color(Color::new(unit(r), unit(r), unit(r), unit(r)));
    s.uv = UvRect {
        u_min: unit(r),
        v_min: unit(r),
        u_max: unit(r),
        v_max: unit(r),
    };
    s
}

pub fn module() -> Module {
    let mut m = Module::new("sprite");
    let mut r = Rng(0x5B_817E_2026_1115);

    // UvRect
    let mut rects: Vec<[u32; 6]> = vec![
        [64, 0, 32, 32, 256, 256],
        [0, 0, 10, 10, 0, 0],
        [0, 0, 10, 10, 0, 256],
        [0, 0, 10, 10, 256, 0],
        [u32::MAX, u32::MAX, u32::MAX, u32::MAX, 3, 7],
        [1, 1, 1, 1, u32::MAX, u32::MAX],
        [300, 0, 50, 0, 256, 1],
    ];
    for _ in 0..120 {
        let aw = (r.next() % 4096) as u32 + 1;
        let ah = (r.next() % 4096) as u32 + 1;
        rects.push([
            (r.next() % 5000) as u32,
            (r.next() % 5000) as u32,
            (r.next() % 300) as u32,
            (r.next() % 300) as u32,
            aw,
            ah,
        ]);
    }
    for (i, p) in rects.iter().enumerate() {
        let uv = UvRect::from_pixel_rect(p[0], p[1], p[2], p[3], p[4], p[5]);
        let ins: Vec<i128> = p.iter().map(|&x| x as i128).collect();
        let outs = Toks::new().gs(&[uv.u_min, uv.v_min, uv.u_max, uv.v_max]);
        m.row(
            "UvRect::from_pixel_rect",
            'A',
            &Toks::new().is(&ins),
            &outs,
            &format!("case {i}"),
        );
    }
    for i in 0..12 {
        let uv = UvRect {
            u_min: unit(&mut r),
            v_min: unit(&mut r),
            u_max: unit(&mut r),
            v_max: unit(&mut r),
        };
        let c: Vec<f32> = uv.corners().iter().flatten().copied().collect();
        m.row(
            "UvRect::corners",
            'A',
            &Toks::new().gs(&[uv.u_min, uv.v_min, uv.u_max, uv.v_max]),
            &Toks::new().gs(&c),
            &format!("case {i}"),
        );
    }

    // center, bounds
    let mut boxes: Vec<[f32; 4]> = (0..200)
        .map(|_| [r.f() * 1e3, r.f() * 1e3, r.f() * 100.0, r.f() * 100.0])
        .collect();
    boxes.extend([
        [10.0, 20.0, -32.0, -16.0],
        [f32::NAN, 1.0, f32::INFINITY, -0.0],
        [f32::MAX, -f32::MAX, f32::MAX, f32::MAX],
        [1e-45, -0.0, -1e-45, 0.0],
    ]);
    for (i, b) in boxes.iter().enumerate() {
        let s = Sprite::new(b[0], b[1], b[2], b[3]);
        let (cx, cy) = s.center();
        let (x0, y0, x1, y1) = s.bounds();
        m.row(
            "Sprite::center",
            'B',
            &Toks::new().gs(b),
            &Toks::new().g(cx).g(cy),
            &format!("case {i}"),
        );
        m.row(
            "Sprite::bounds",
            'B',
            &Toks::new().gs(b),
            &Toks::new().g(x0).g(y0).g(x1).g(y1),
            &format!("case {i}"),
        );
    }
    let d = Sprite::default();
    m.row(
        "Sprite::default",
        'A',
        &Toks::new(),
        &Toks::new()
            .gs(&sprite13(&d))
            .i(d.texture_id as i128)
            .i(d.z_order as i128),
        "Sprite::default()",
    );

    // sort_by_z: the permutation, as original indices (texture_id = index)
    let mut zsets: Vec<Vec<i32>> = vec![
        vec![],
        vec![7],
        vec![2, 1],
        vec![2, 0, 1],
        vec![0, 0, 0],
        vec![i32::MAX, i32::MIN, 0, -1, i32::MAX, i32::MIN],
    ];
    zsets.push((0..1000).map(|i| 1000 - i).collect());
    zsets.push((0..100).map(|i| i / 7).collect());
    for _ in 0..12 {
        let n = (r.next() % 200) as usize + 2;
        zsets.push((0..n).map(|_| (r.next() % 9) as i32 - 4).collect());
    }
    for _ in 0..4 {
        zsets.push((0..300).map(|_| r.next() as i32).collect());
    }
    for (i, zs) in zsets.iter().enumerate() {
        let mut b = SpriteBatch::new();
        for (k, &z) in zs.iter().enumerate() {
            b.push(Sprite::default().with_z_order(z).with_texture(k as u64));
        }
        b.sort_by_z();
        let perm: Vec<i128> = b.sprites.iter().map(|s| s.texture_id as i128).collect();
        let ins: Vec<i128> = zs.iter().map(|&z| z as i128).collect();
        m.row(
            "SpriteBatch::sort_by_z",
            'A',
            &Toks::new().is(&ins),
            &Toks::new().is(&perm),
            &format!("set {i} ({} sprites)", zs.len()),
        );
    }

    // batch_to_vertices: small batches, every sprite in full
    let rots = [
        0.0f32,
        -0.0,
        1.57,
        -3.0,
        0.5,
        f32::NAN,
        f32::INFINITY,
        1e6,
        std::f32::consts::PI,
        1e-30,
    ];
    for i in 0..60 {
        let n = [0usize, 1, 2, 3, 7, 16][i % 6];
        let mut b = SpriteBatch::new();
        for k in 0..n {
            let rot = if (i + k) % 3 == 0 {
                rots[(i + k) % rots.len()]
            } else {
                r.f()
            };
            b.push(random_sprite(&mut r, rot));
        }
        let mut ins = Toks::new().i(n as i128);
        for s in &b.sprites {
            ins = ins.gs(&sprite13(s));
        }
        let u32 = i % 4 == 3;
        let (verts, ib): (Vec<f32>, Vec<i128>) = if u32 {
            let (v, ix) = batch_to_vertices_u32(&b);
            (
                v.iter()
                    .flat_map(|v| {
                        [
                            v.position[0],
                            v.position[1],
                            v.tex_coords[0],
                            v.tex_coords[1],
                            v.color[0],
                            v.color[1],
                            v.color[2],
                            v.color[3],
                        ]
                    })
                    .collect(),
                ix.iter().map(|&x| x as i128).collect(),
            )
        } else {
            let (v, ix) = batch_to_vertices(&b);
            (
                v.iter()
                    .flat_map(|v| {
                        [
                            v.position[0],
                            v.position[1],
                            v.tex_coords[0],
                            v.tex_coords[1],
                            v.color[0],
                            v.color[1],
                            v.color[2],
                            v.color[3],
                        ]
                    })
                    .collect(),
                ix.iter().map(|&x| x as i128).collect(),
            )
        };
        let name = if u32 {
            "batch_to_vertices_u32"
        } else {
            "batch_to_vertices"
        };
        m.row(
            name,
            'C',
            &ins,
            &Toks::new().gs(&verts).is(&ib),
            &format!("batch {i} ({n} sprites)"),
        );
    }

    // At and beyond the u16 limit: digests
    for (u32, n) in [
        (false, MAX_SPRITES_PER_BATCH - 1),
        (false, MAX_SPRITES_PER_BATCH),
        (false, MAX_SPRITES_PER_BATCH + 1),
        (false, MAX_SPRITES_PER_BATCH + 100),
        (true, MAX_SPRITES_PER_BATCH + 1),
        (true, 70_000),
    ] {
        let mut b = SpriteBatch::new();
        for k in 0..n {
            b.push(Sprite::new(k as f32, 0.0, 1.0, 1.0));
        }
        let (vc, ic, vh, ih) = if u32 {
            let (v, ix) = batch_to_vertices_u32(&b);
            let vb: &[u8] = bytemuck::cast_slice(&v);
            (
                v.len(),
                ix.len(),
                fnv(vb.iter().copied()),
                fnv(ix.iter().flat_map(|x| x.to_le_bytes())),
            )
        } else {
            let (v, ix) = batch_to_vertices(&b);
            let vb: &[u8] = bytemuck::cast_slice(&v);
            (
                v.len(),
                ix.len(),
                fnv(vb.iter().copied()),
                fnv(ix.iter().flat_map(|x| x.to_le_bytes())),
            )
        };
        let name = if u32 {
            "batch_to_vertices_u32_digest"
        } else {
            "batch_to_vertices_digest"
        };
        m.row(
            name,
            'A',
            &Toks::new().i(n as i128),
            &Toks::new()
                .i(vc as i128)
                .i(ic as i128)
                .i(vh.0)
                .i(vh.1)
                .i(ih.0)
                .i(ih.1),
            &format!("{n} sprites Sprite::new(i, 0, 1, 1)"),
        );
    }
    m
}
