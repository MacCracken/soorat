//! `shadow` (M4 part a): shadow's CPU half: the directional light matrices, practical cascade splits, the
//! point-light cube faces, the atlas tile math, and the uniforms' defaults. The depth maps, the atlas texture and the
//! pass are GPU (M7).
//!
//! The matrices and splits are class **E** (plan §7: Rust's formula in f64 rounded once; `validate.py --write`
//! writes `shadow.e.tsv`). The atlas math and the defaults are class A. Branch points are tested on both sides: the
//! extent and depth-span guards, the `|d.y| > 0.99` up-vector switch, perspective_90's near ≈ far guard, and the
//! splits' input sanitizing.

use crate::{Module, Rng, Toks};
use soorat::oracle::{atlas_columns, tile_uv_in, tile_viewport_in};
use soorat::shadow::{
    CascadeUniforms, PointShadowMap, ShadowAtlasConfig, ShadowUniforms, compute_practical_splits,
    directional_light_matrix, directional_light_matrix_at, tile_uv, tile_viewport,
};

fn bits_near(x: f32, k: i32) -> f32 {
    f32::from_bits((x.to_bits() as i32 + k) as u32)
}

pub fn module() -> Module {
    let mut m = Module::new("shadow");
    let mut r = Rng(0x5AD0_0004_0000_000A);

    let u = ShadowUniforms::default();
    m.row(
        "ShadowUniforms::default",
        'A',
        &Toks::new(),
        &Toks::new().gs(&u.light_view_proj).gs(&u.model),
        "ShadowUniforms::default()",
    );
    let c = CascadeUniforms::default();
    m.row(
        "CascadeUniforms::default",
        'A',
        &Toks::new(),
        &Toks::new()
            .gs(&c.splits)
            .gs(&c.matrices.iter().flatten().copied().collect::<Vec<f32>>()),
        "CascadeUniforms::default()",
    );

    // ── directional light matrices ──
    let mut dirs: Vec<[f32; 3]> = vec![
        [0.0, -1.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, -1.0],
        [0.0, -1.0, -1.0],
        [0.3, -1.0, 0.2],
        [0.4, -1.0, 0.1],
        [0.0, -0.5, -1.0],
        [0.0, -0.999, -0.01],
        [0.0, 0.0, 0.0],
        [1e-20, 0.0, 0.0],
        [f32::NAN, -1.0, 0.0],
        [0.0, -1.0, f32::INFINITY],
        [-3.0, 4.0, 12.0],
        [1e30, -1e30, 0.0],
    ];
    // |d.y| / |d| crosses 0.99 at s ≈ 0.142470 for d = (s, -1, 0): each side, a few ulps apart.
    for k in -20..=20 {
        dirs.push([bits_near(0.142_470_33, k * 3), -1.0, 0.0]);
    }
    let ranges: Vec<(f32, f32, f32)> = vec![
        (10.0, 0.1, 50.0),
        (20.0, 0.1, 100.0),
        (8.0, -10.0, 10.0),
        (5.0, -20.0, 20.0),
        (1.0, 50.0, -50.0),
        (0.0, 0.0, 1.0),
        (-1.0, 0.0, 1.0),
        (f32::NAN, 0.0, 1.0),
        (1e-30, 0.0, 1.0),
        (1e30, 0.0, 1.0),
        (f32::INFINITY, 0.0, 1.0),
        (1.0, 1.0, 1.0),
        (1.0, 1.0, f32::NAN),
        (1.0, 0.0, 1e-10),
        (1.0, 0.0, bits_near(1e-10, -1)),
        (1.0, 0.0, bits_near(1e-10, 1)),
        (1.0, 0.0, f32::INFINITY),
        (1.0, f32::NEG_INFINITY, f32::INFINITY),
    ];
    for d in &dirs {
        for &(e, n, f) in &ranges {
            let mtx = directional_light_matrix(*d, e, n, f);
            m.row(
                "directional_light_matrix",
                'E',
                &Toks::new().gs(d).g(e).g(n).g(f),
                &Toks::new().gs(&mtx),
                &format!("d {d:?}, extent {e}, near {n}, far {f}"),
            );
        }
    }
    let centers: Vec<[f32; 3]> = vec![
        [10.0, 3.0, -4.0],
        [10.0, 0.0, -5.0],
        [1e6, -1e6, 3.0],
        [0.0, 0.0, 0.0],
    ];
    for d in dirs.iter().take(10) {
        for ce in &centers {
            for &(e, n, f) in ranges.iter().take(6) {
                let mtx = directional_light_matrix_at(*d, *ce, e, n, f);
                m.row(
                    "directional_light_matrix_at",
                    'E',
                    &Toks::new().gs(d).gs(ce).g(e).g(n).g(f),
                    &Toks::new().gs(&mtx),
                    &format!("d {d:?}, center {ce:?}, extent {e}, near {n}, far {f}"),
                );
            }
        }
    }
    for k in 0..200 {
        let d = [r.f(), r.f(), r.f()];
        let ce = [r.f() * 4.0, r.f() * 4.0, r.f() * 4.0];
        let e = r.pos() * 4.0;
        let n = r.f() * 8.0;
        let f = n + r.pos() * 8.0;
        let mtx = directional_light_matrix_at(d, ce, e, n, f);
        m.row(
            "directional_light_matrix_at",
            'E',
            &Toks::new().gs(&d).gs(&ce).g(e).g(n).g(f),
            &Toks::new().gs(&mtx),
            &format!("random {k}"),
        );
    }

    // ── practical cascade splits ──
    let nears = [
        0.0f32,
        -1.0,
        f32::NAN,
        f32::INFINITY,
        1e-6,
        0.001,
        0.1,
        1.0,
        10.0,
    ];
    let fars = [
        f32::NAN,
        f32::INFINITY,
        -5.0,
        0.1,
        1.0,
        100.0,
        500.0,
        1e6,
        3e38,
    ];
    let counts = [0usize, 1, 2, 3, 4, 8];
    let lambdas = [f32::NAN, -3.0, 0.0, 0.25, 0.5, 1.0, 7.0];
    for &n in &nears {
        for &f in &fars {
            for &cnt in &counts {
                for &l in &lambdas {
                    let s = compute_practical_splits(n, f, cnt, l);
                    m.row(
                        "compute_practical_splits",
                        'E',
                        &Toks::new().g(n).g(f).i(cnt as i128).g(l),
                        &Toks::new().gs(&s),
                        &format!("near {n}, far {f}, count {cnt}, lambda {l}"),
                    );
                }
            }
        }
    }
    for k in 0..100 {
        let n = r.pos() / 16.0;
        let f = n + r.pos() * 64.0;
        let l = r.pos() / 16.0;
        let cnt = (r.next() % 6) as usize + 1;
        let s = compute_practical_splits(n, f, cnt, l);
        m.row(
            "compute_practical_splits",
            'E',
            &Toks::new().g(n).g(f).i(cnt as i128).g(l),
            &Toks::new().gs(&s),
            &format!("random {k}"),
        );
    }

    // ── point-light cube faces ──
    let mut points: Vec<([f32; 3], f32, f32)> = vec![
        ([0.0, 5.0, 0.0], 0.1, 25.0),
        ([10.0, 3.0, -5.0], 0.1, 50.0),
        ([0.0, 0.0, 0.0], 1.0, 1.0),
        ([0.0, 0.0, 0.0], 1.0, 0.5),
        ([1.0, 2.0, 3.0], 0.0, 100.0),
        ([1.0, 2.0, 3.0], f32::NAN, 10.0),
        ([1e6, -1e6, 1e6], 0.5, 1000.0),
    ];
    for _ in 0..30 {
        let n = r.pos() / 8.0;
        points.push((
            [r.f() * 4.0, r.f() * 4.0, r.f() * 4.0],
            n,
            n + r.pos() * 8.0,
        ));
    }
    for (p, n, f) in &points {
        let psm = PointShadowMap::new(*p, *n, *f);
        m.row(
            "PointShadowMap::new",
            'E',
            &Toks::new().gs(p).g(*n).g(*f),
            &Toks::new().gs(&psm
                .face_matrices
                .iter()
                .flatten()
                .copied()
                .collect::<Vec<f32>>()),
            &format!("position {p:?}, near {n}, far {f}"),
        );
    }

    // ── the atlas ──
    let sizes = [0u32, 1, 2, 512, 1000, 1024, 4096, 16_777_217, u32::MAX];
    let tiles = [0u32, 1, 3, 512, 1024, 2048, u32::MAX];
    for &s in &sizes {
        for &t in &tiles {
            m.row(
                "atlas_columns",
                'A',
                &Toks::new().i(s as i128).i(t as i128),
                &Toks::new().i(atlas_columns(s, t) as i128),
                "",
            );
        }
    }
    let idx = [0u32, 1, 3, 4, 15, 16, 17, 63, 1000, u32::MAX];
    for &cols in &[0u32, 1, 4, 7, 4096, u32::MAX] {
        for &t in &[0u32, 1, 1024, 16_777_217, u32::MAX] {
            for &i in &idx {
                let (x, y, w, h) = tile_viewport_in(cols, t, i);
                m.row(
                    "tile_viewport_in",
                    'A',
                    &Toks::new().i(cols as i128).i(t as i128).i(i as i128),
                    &Toks::new()
                        .i(x as i128)
                        .i(y as i128)
                        .i(w as i128)
                        .i(h as i128),
                    "",
                );
                for &s in &[0u32, 1, 4096, 16_777_217, u32::MAX] {
                    m.row(
                        "tile_uv_in",
                        'A',
                        &Toks::new()
                            .i(cols as i128)
                            .i(t as i128)
                            .i(s as i128)
                            .i(i as i128),
                        &Toks::new().gs(&tile_uv_in(cols, t, s, i)),
                        "",
                    );
                }
            }
        }
    }
    for &(s, t) in &[
        (4096u32, 1024u32),
        (512, 1024),
        (0, 0),
        (4096, 1),
        (1000, 300),
        (u32::MAX, 3),
    ] {
        let cfg = ShadowAtlasConfig {
            size: s,
            tile_size: t,
        };
        for &i in &idx {
            let (x, y, w, h) = tile_viewport(&cfg, i);
            m.row(
                "tile_viewport",
                'A',
                &Toks::new().i(s as i128).i(t as i128).i(i as i128),
                &Toks::new()
                    .i(x as i128)
                    .i(y as i128)
                    .i(w as i128)
                    .i(h as i128),
                "",
            );
            m.row(
                "tile_uv",
                'A',
                &Toks::new().i(s as i128).i(t as i128).i(i as i128),
                &Toks::new().gs(&tile_uv(&cfg, i)),
                "",
            );
        }
    }
    m
}
