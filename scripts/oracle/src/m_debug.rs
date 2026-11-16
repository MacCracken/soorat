//! `debug` and `debug_collider`: soorat 1.2.0's `debug_draw` geometry (M2 part b), as the vertices a `LineBatch`
//! holds (7 f32 each: position, color).
//!
//! - `line`, `wire_box`, `grid`: class **A** (copies, f32 sums and products, `ceil`, `as i32`).
//! - `wire_circle`, `wire_sphere`, `wire_capsule`: class **C**. Their angles' f32 `sin`/`cos` are host glibc's;
//!   `validate.py --write` puts the correctly rounded vertices in `debug.cr.tsv` where glibc misrounds.
//! - `debug_collider` (the `physics-debug` feature, impetus 1.4): `LineBatch::collider` for each shape kind, class C
//!   (a ball and a capsule are circles). A shape is `i:<kind>` (impetus's IM_SHAPE_* numbering: box 0, ball 1,
//!   capsule 2, convex hull 3, tri mesh 4, height field 5, segment 6) then its f64 dimensions. Replayed by
//!   tests/parity_features.tcyr.

use crate::{Module, Rng, Toks};
use impetus::collider::ColliderShape;
use soorat::color::Color;
use soorat::debug_draw::LineBatch;

fn verts(b: &LineBatch) -> Toks {
    let v: Vec<f32> = b
        .vertices
        .iter()
        .flat_map(|v| {
            [
                v.position[0],
                v.position[1],
                v.position[2],
                v.color[0],
                v.color[1],
                v.color[2],
                v.color[3],
            ]
        })
        .collect();
    Toks::new().gs(&v)
}

fn v3(r: &mut Rng) -> [f32; 3] {
    [r.f() * 10.0, r.f() * 10.0, r.f() * 10.0]
}

fn col(r: &mut Rng) -> Color {
    Color::new(((r.next() >> 41) as f32) / 8_388_608.0, 0.5, 0.25, 1.0)
}

pub fn module() -> Module {
    let mut m = Module::new("debug");
    let mut r = Rng(0xDEB6_2026_1115);

    for i in 0..10 {
        let (a, b, c) = (v3(&mut r), v3(&mut r), col(&mut r));
        let mut lb = LineBatch::new();
        lb.line(a, b, c);
        m.row(
            "LineBatch::line",
            'A',
            &Toks::new().gs(&a).gs(&b).gs(&c.to_array()),
            &verts(&lb),
            &format!("case {i}"),
        );
    }
    for i in 0..10 {
        let (a, b, c) = (v3(&mut r), v3(&mut r), col(&mut r));
        let mut lb = LineBatch::new();
        lb.wire_box(a, b, c);
        m.row(
            "LineBatch::wire_box",
            'A',
            &Toks::new().gs(&a).gs(&b).gs(&c.to_array()),
            &verts(&lb),
            &format!("case {i}"),
        );
    }
    let radii = [1.0f32, 0.5, 3.75, -1.0, 0.0, f32::NAN, 1e30];
    for (i, &seg) in [0u32, 1, 2, 3, 4, 7, 16, 32, 64, 100].iter().enumerate() {
        for (k, &rad) in radii.iter().enumerate() {
            if k > 2 && i % 3 != 0 {
                continue;
            }
            let (c, cl) = (v3(&mut r), col(&mut r));
            let ins = Toks::new().gs(&c).g(rad).i(seg as i128).gs(&cl.to_array());
            let mut lb = LineBatch::new();
            lb.wire_circle(c, rad, seg, cl);
            m.row(
                "LineBatch::wire_circle",
                'C',
                &ins,
                &verts(&lb),
                &format!("{seg} segments, r {rad:?}"),
            );
            if seg <= 32 {
                let mut ls = LineBatch::new();
                ls.wire_sphere(c, rad, seg, cl);
                m.row(
                    "LineBatch::wire_sphere",
                    'C',
                    &ins,
                    &verts(&ls),
                    &format!("{seg} segments, r {rad:?}"),
                );
                let hh = r.f();
                let mut lc = LineBatch::new();
                lc.wire_capsule(c, hh, rad, seg, cl);
                m.row(
                    "LineBatch::wire_capsule",
                    'C',
                    &Toks::new()
                        .gs(&c)
                        .g(hh)
                        .g(rad)
                        .i(seg as i128)
                        .gs(&cl.to_array()),
                    &verts(&lc),
                    &format!("{seg} segments, r {rad:?}"),
                );
            }
        }
    }
    let grids = [
        (5.0f32, 1.0f32),
        (10.0, 0.3),
        (1.0, 3.0),
        (0.5, 0.5),
        (100.0, 7.7),
        (5.0, 0.0),
        (5.0, -1.0),
        (0.0, 1.0),
        (-0.0, 1.0),
        (f32::NAN, 1.0),
        (3.0, f32::NAN),
        (2.5, 1.0),
        (1e-3, 1e-4),
    ];
    for (i, &(h, s)) in grids.iter().enumerate() {
        let c = col(&mut r);
        let mut lb = LineBatch::new();
        lb.grid(h, s, c);
        m.row(
            "LineBatch::grid",
            'A',
            &Toks::new().g(h).g(s).gs(&c.to_array()),
            &verts(&lb),
            &format!("grid {i}: {h:?} / {s:?}"),
        );
    }
    m
}

/// The `physics-debug` rows.
pub fn collider_module() -> Module {
    let mut m = Module::new("debug_collider");
    let mut r = Rng(0xC011_1DE7_2026_1115);
    let d = |r: &mut Rng| -> f64 { (r.next() >> 11) as f64 / 9_007_199_254_740_992.0 * 8.0 - 2.0 };
    for i in 0..60 {
        let (pos, cl) = (v3(&mut r), col(&mut r));
        let (shape, ins) = match i % 6 {
            0 => {
                let he = [d(&mut r).abs(), d(&mut r).abs(), d(&mut r).abs()];
                (
                    ColliderShape::Box { half_extents: he },
                    Toks::new().i(0).f(he[0]).f(he[1]).f(he[2]),
                )
            }
            1 => {
                let rad = d(&mut r).abs();
                (ColliderShape::Ball { radius: rad }, Toks::new().i(1).f(rad))
            }
            2 => {
                let (hh, rad) = (d(&mut r).abs(), d(&mut r).abs());
                (
                    ColliderShape::Capsule {
                        half_height: hh,
                        radius: rad,
                    },
                    Toks::new().i(2).f(hh).f(rad),
                )
            }
            3 => {
                let (a, b) = (
                    [d(&mut r), d(&mut r), d(&mut r)],
                    [d(&mut r), d(&mut r), d(&mut r)],
                );
                (
                    ColliderShape::Segment { a, b },
                    Toks::new()
                        .i(6)
                        .f(a[0])
                        .f(a[1])
                        .f(a[2])
                        .f(b[0])
                        .f(b[1])
                        .f(b[2]),
                )
            }
            4 => (
                ColliderShape::ConvexHull {
                    points: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                },
                Toks::new().i(3),
            ),
            _ => (
                ColliderShape::Heightfield {
                    heights: vec![0.0, 1.0],
                    scale: [1.0, 1.0, 1.0],
                },
                Toks::new().i(5),
            ),
        };
        let mut lb = LineBatch::new();
        lb.collider(&shape, pos, cl);
        m.row(
            "LineBatch::collider",
            'C',
            &ins.gs(&pos).gs(&cl.to_array()),
            &verts(&lb),
            &format!("shape {i}"),
        );
    }
    m
}
