//! `acoustic` and `acoustic_render` (M5 part a): soorat 1.2.0's `acoustic_render`.
//!
//! - `acoustic` (class **A**, replayed by tests/parity.tcyr): `acoustic_heat_map`, and the two parameter structs'
//!   defaults. Rust compiles these without the `acoustics` feature.
//! - `acoustic_render` (class **E**, replayed by tests/parity_acoustics.tcyr): `ray_paths_to_lines`,
//!   `portal_to_lines` and `coupled_decay_to_lines`, as the vertices their `LineBatch` holds (7 f32 each). Rust reads
//!   goonj 1.4.3's f32 types; the port reads goonj 2.3.2's f64 ones (plan §2.5), fed the same f32-representable values.
//!   `validate.py --write` writes `acoustic_render.e.tsv`: the formula in f64, rounded once (ADR 0002 #29).
//!
//! Inputs: a ray visualization is its source (`v:3`), `i:<paths>`, then per path `i:<bounces>` and per bounce its
//! point (`v:3`) and 8 band energies (`v:8`); then the two colours. A portal is position, normal, width, height, colour,
//! arrow length. A decay is rt60_early, rt60_late, early_amplitude, coupling_strength, then the curve's parameters
//! (origin `v:3`, time_range, db_range, x_scale, y_scale, `i:<steps>`, the two colours).

use crate::{Module, Rng, Toks};
use goonj::coupled::CoupledDecay;
use goonj::integration::soorat::RayVisualization;
use goonj::portal::Portal;
use goonj::ray::{RayBounce, RayPath};
use hisab::Vec3;
use soorat::acoustic_render::{
    AcousticColorMode, AcousticVisParams, DecayCurveParams, acoustic_heat_map,
    coupled_decay_to_lines, portal_to_lines, ray_paths_to_lines,
};
use soorat::color::Color;
use soorat::debug_draw::LineBatch;

/// A ray path: each bounce's point and 8 band energies.
type Path = Vec<([f32; 3], [f32; 8])>;
/// A portal: position, normal, width, height, arrow length.
type PortalCase = ([f32; 3], [f32; 3], f32, f32, f32);

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

fn c4(c: Color) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

fn v3(r: &mut Rng) -> [f32; 3] {
    [r.f(), r.f(), r.f()]
}

fn vec3(p: [f32; 3]) -> Vec3 {
    Vec3::new(p[0], p[1], p[2])
}

fn bits_near(x: f32, k: i32) -> f32 {
    f32::from_bits((x.to_bits() as i32 + k) as u32)
}

/// A ray visualization and its input tokens.
fn rays(source: [f32; 3], paths: &[Path]) -> (RayVisualization, Toks) {
    let mut t = Toks::new().gs(&source).i(paths.len() as i128);
    let mut ps = Vec::new();
    for path in paths {
        t = t.i(path.len() as i128);
        let mut bs = Vec::new();
        for (p, e) in path {
            t = t.gs(p).gs(e);
            bs.push(RayBounce {
                point: vec3(*p),
                normal: Vec3::ZERO,
                wall_index: 0,
                distance_from_previous: 0.0,
                energy_after: *e,
            });
        }
        ps.push(RayPath {
            bounces: bs,
            total_distance: 0.0,
            final_energy: [0.0; 8],
        });
    }
    (
        RayVisualization {
            source: vec3(source),
            paths: ps,
        },
        t,
    )
}

pub fn module() -> Module {
    let mut m = Module::new("acoustic");
    let mut r = Rng(0xAC05_0005_0000_000A);
    let mut ts: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        0.25,
        0.5,
        0.75,
        -1.0,
        2.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MIN_POSITIVE,
        bits_near(0.25, -1),
        bits_near(0.25, 1),
        bits_near(0.5, -1),
        bits_near(0.5, 1),
        bits_near(0.75, -1),
        bits_near(0.75, 1),
        bits_near(1.0, -1),
    ];
    for _ in 0..60 {
        ts.push(r.pos() / 16.0);
    }
    for t in ts {
        m.row(
            "acoustic_heat_map",
            'A',
            &Toks::new().g(t),
            &Toks::new().gs(&c4(acoustic_heat_map(t))),
            &format!("t {t}"),
        );
    }
    let p = AcousticVisParams::default();
    let mode = match p.color_mode {
        AcousticColorMode::Pressure => "AcousticColorMode.Pressure",
        AcousticColorMode::SignedPressure => "AcousticColorMode.SignedPressure",
        AcousticColorMode::Energy => "AcousticColorMode.Energy",
        _ => "AcousticColorMode.Unknown",
    };
    m.row(
        "AcousticVisParams::default",
        'A',
        &Toks::new(),
        &Toks::new().e(mode).gs(&c4(p.base_color)).g(p.alpha),
        "AcousticVisParams::default()",
    );
    let d = DecayCurveParams::default();
    m.row(
        "DecayCurveParams::default",
        'A',
        &Toks::new(),
        &Toks::new()
            .gs(&d.origin)
            .g(d.time_range)
            .g(d.db_range)
            .g(d.x_scale)
            .g(d.y_scale)
            .i(d.steps as i128)
            .gs(&c4(d.color_early))
            .gs(&c4(d.color_late)),
        "DecayCurveParams::default()",
    );
    m
}

fn decay_row(m: &mut Module, dc: &CoupledDecay, p: &DecayCurveParams, note: &str) {
    let mut b = LineBatch::new();
    coupled_decay_to_lines(dc, p, &mut b);
    m.row(
        "coupled_decay_to_lines",
        'E',
        &Toks::new()
            .g(dc.rt60_early)
            .g(dc.rt60_late)
            .g(dc.early_amplitude)
            .g(dc.coupling_strength)
            .gs(&p.origin)
            .g(p.time_range)
            .g(p.db_range)
            .g(p.x_scale)
            .g(p.y_scale)
            .i(p.steps as i128)
            .gs(&c4(p.color_early))
            .gs(&c4(p.color_late)),
        &verts(&b),
        note,
    );
}

pub fn render_module() -> Module {
    let mut m = Module::new("acoustic_render");
    let mut r = Rng(0xAC05_0005_0000_000B);
    let white = Color::WHITE;
    let red = Color::RED;

    // ── ray paths ──
    let e1 = [1.0f32; 8];
    let half = [0.5f32; 8];
    let mixed = [0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2];
    let mut cases: Vec<(&str, [f32; 3], Vec<Path>)> = vec![
        ("no paths", [0.0, 1.0, 0.0], vec![]),
        ("a path with no bounces", [0.0, 1.0, 0.0], vec![vec![]]),
        (
            "one bounce",
            [0.0, 1.0, 0.0],
            vec![vec![([1.0, 1.0, 0.0], half)]],
        ),
        (
            "three bounces",
            [0.5, 1.5, -2.0],
            vec![vec![
                ([3.0, 1.0, 0.0], mixed),
                ([3.0, 2.0, 4.0], half),
                ([-1.0, 2.0, 4.0], e1),
            ]],
        ),
        (
            "two paths",
            [0.0, 0.0, 0.0],
            vec![
                vec![([1.0, 0.0, 0.0], half), ([1.0, 1.0, 0.0], half)],
                vec![([0.0, 0.0, 1.0], mixed)],
            ],
        ),
    ];
    let specials: [(&str, [f32; 8]); 8] = [
        ("a NaN band", [0.5, f32::NAN, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]),
        (
            "an infinite band",
            [0.5, f32::INFINITY, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
        ),
        ("negative energies", [-0.5; 8]),
        ("energies above 1", [3.0; 8]),
        ("energies that overflow an f32 sum", [3e38; 8]),
        ("-0 energies", [-0.0; 8]),
        ("tiny energies", [1e-30; 8]),
        ("one band", [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.75]),
    ];
    for (note, e) in specials {
        cases.push((
            note,
            [0.0, 0.0, 0.0],
            vec![vec![
                ([1.0, 0.0, 0.0], e),
                ([2.0, 0.0, 0.0], half),
                ([3.0, 0.0, 0.0], e),
            ]],
        ));
    }
    for (note, src, paths) in &cases {
        let (rv, t) = rays(*src, paths);
        let mut b = LineBatch::new();
        ray_paths_to_lines(&rv, white, red, &mut b);
        m.row(
            "ray_paths_to_lines",
            'E',
            &t.gs(&c4(white)).gs(&c4(red)),
            &verts(&b),
            note,
        );
    }
    for k in 0..200 {
        let np = (r.next() % 3) as usize + 1;
        let paths: Vec<Path> = (0..np)
            .map(|_| {
                let nb = (r.next() % 5) as usize;
                (0..nb)
                    .map(|_| {
                        let p = v3(&mut r);
                        let mut e = [0.0f32; 8];
                        for x in &mut e {
                            *x = r.pos() / 16.0;
                        }
                        (p, e)
                    })
                    .collect()
            })
            .collect();
        let src = v3(&mut r);
        let (rv, t) = rays(src, &paths);
        let a = Color::new(r.pos() / 16.0, r.pos() / 16.0, r.pos() / 16.0, 1.0);
        let z = Color::new(
            r.pos() / 16.0,
            r.pos() / 16.0,
            r.pos() / 16.0,
            r.pos() / 16.0,
        );
        let mut b = LineBatch::new();
        ray_paths_to_lines(&rv, a, z, &mut b);
        m.row(
            "ray_paths_to_lines",
            'E',
            &t.gs(&c4(a)).gs(&c4(z)),
            &verts(&b),
            &format!("random {k}"),
        );
    }

    // ── portals ──
    let mut portals: Vec<PortalCase> = vec![
        ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], 2.0, 3.0, 1.0),
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], 1.0, 2.0, 0.5),
        ([5.0, 0.0, 0.0], [0.0, 1.0, 0.0], 1.0, 1.0, 2.0),
        ([0.0, 0.0, 0.0], [0.0, -1.0, 0.0], 1.0, 1.0, 1.0),
        ([0.0, 0.0, 0.0], [0.6, 0.8, 0.0], 1.5, 0.75, 1.0),
        ([1.0, 2.0, 3.0], [3.0, 4.0, 12.0], 2.0, 2.0, 1.0),
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1.0, 1.0, 1.0),
        ([0.0, 0.0, 0.0], [f32::NAN, 0.0, 1.0], 1.0, 1.0, 1.0),
        ([0.0, 0.0, 0.0], [0.0, f32::NAN, 1.0], 1.0, 1.0, 1.0),
        ([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 0.0, 0.0, 0.0),
        ([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], -1.0, 2.0, -1.0),
        ([1e30, -1e30, 0.0], [0.0, 0.0, 1.0], 1e30, 1e30, 1e30),
    ];
    // |n.y| crosses 0.99 for n = (s, 0.99, 0): each side, a few ulps apart.
    for k in -3..=3 {
        portals.push((
            [0.0, 0.0, 0.0],
            [0.1, bits_near(0.99, k), 0.0],
            1.0,
            1.0,
            1.0,
        ));
    }
    for _ in 0..200 {
        portals.push((v3(&mut r), v3(&mut r), r.pos(), r.pos(), r.pos() / 4.0));
    }
    for (pos, n, w, h, len) in &portals {
        let portal = Portal {
            position: vec3(*pos),
            normal: vec3(*n),
            width: *w,
            height: *h,
        };
        let mut b = LineBatch::new();
        portal_to_lines(&portal, white, *len, &mut b);
        m.row(
            "portal_to_lines",
            'E',
            &Toks::new().gs(pos).gs(n).g(*w).g(*h).gs(&c4(white)).g(*len),
            &verts(&b),
            &format!("position {pos:?}, normal {n:?}, {w} × {h}, arrow {len}"),
        );
    }

    // ── coupled decays ──
    let dc = |e: f32, l: f32, a: f32| CoupledDecay {
        rt60_early: e,
        rt60_late: l,
        early_amplitude: a,
        coupling_strength: 0.5,
    };
    let base = DecayCurveParams::default();
    let decays = [
        (dc(0.5, 2.0, 0.7), "rt60 0.5 / 2.0, a 0.7"),
        (dc(1.0, 1.0, 0.5), "equal slopes"),
        (dc(0.0, 2.0, 0.7), "rt60_early 0 (f32::EPSILON)"),
        (dc(-1.0, -1.0, 0.5), "both rt60 negative"),
        (dc(f32::NAN, 2.0, 0.7), "NaN rt60_early"),
        (dc(0.5, 2.0, f32::NAN), "NaN early_amplitude"),
        (dc(0.5, 2.0, -1.0), "early_amplitude below 0"),
        (dc(0.5, 2.0, 2.0), "early_amplitude above 1"),
        (dc(0.5, 2.0, 0.0), "early_amplitude 0"),
        (dc(0.5, 2.0, 1.0), "early_amplitude 1"),
        (dc(1e-6, 1e6, 0.5), "extreme rt60s"),
    ];
    for (d, note) in &decays {
        decay_row(&mut m, d, &base, note);
    }
    let d0 = dc(0.5, 2.0, 0.7);
    let variants: Vec<(DecayCurveParams, &str)> = vec![
        (
            DecayCurveParams {
                steps: 0,
                ..DecayCurveParams::default()
            },
            "0 steps",
        ),
        (
            DecayCurveParams {
                steps: 1,
                ..DecayCurveParams::default()
            },
            "1 step",
        ),
        (
            DecayCurveParams {
                steps: 2,
                ..DecayCurveParams::default()
            },
            "2 steps",
        ),
        (
            DecayCurveParams {
                steps: 3,
                ..DecayCurveParams::default()
            },
            "3 steps",
        ),
        (
            DecayCurveParams {
                steps: 500,
                ..DecayCurveParams::default()
            },
            "500 steps",
        ),
        (
            DecayCurveParams {
                time_range: 0.0,
                ..DecayCurveParams::default()
            },
            "time range 0",
        ),
        (
            DecayCurveParams {
                time_range: -1.0,
                ..DecayCurveParams::default()
            },
            "time range negative",
        ),
        (
            DecayCurveParams {
                time_range: f32::NAN,
                ..DecayCurveParams::default()
            },
            "time range NaN",
        ),
        (
            DecayCurveParams {
                db_range: -0.5,
                ..DecayCurveParams::default()
            },
            "db range -0.5 (min -1)",
        ),
        (
            DecayCurveParams {
                db_range: 0.0,
                ..DecayCurveParams::default()
            },
            "db range 0",
        ),
        (
            DecayCurveParams {
                db_range: f32::NAN,
                ..DecayCurveParams::default()
            },
            "db range NaN",
        ),
        (
            DecayCurveParams {
                db_range: -120.0,
                ..DecayCurveParams::default()
            },
            "db range -120",
        ),
        (
            DecayCurveParams {
                origin: [1.5, -2.0, 3.0],
                x_scale: 2.5,
                y_scale: 0.75,
                ..DecayCurveParams::default()
            },
            "moved and scaled",
        ),
    ];
    for (p, note) in &variants {
        decay_row(&mut m, &d0, p, note);
    }
    for k in 0..100 {
        let d = dc(r.pos() / 4.0, r.pos() / 2.0, r.pos() / 16.0);
        let p = DecayCurveParams {
            origin: v3(&mut r),
            time_range: r.pos() / 2.0,
            db_range: -(r.pos() * 8.0),
            x_scale: r.pos(),
            y_scale: r.pos(),
            steps: (r.next() % 60) as u32 + 2,
            color_early: Color::new(r.pos() / 16.0, 0.5, 0.25, 1.0),
            color_late: Color::new(0.25, r.pos() / 16.0, 0.5, 1.0),
        };
        decay_row(&mut m, &d, &p, &format!("random {k}"));
    }
    m
}
