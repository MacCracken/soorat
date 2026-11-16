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
use goonj::directivity::DirectivityBalloon;
use goonj::integration::soorat::{ModeVisualization, PressureMap, RayVisualization};
use goonj::portal::Portal;
use goonj::ray::{RayBounce, RayPath};
use hisab::Vec3;
use soorat::acoustic_render::{
    AcousticColorMode, AcousticVisParams, DecayCurveParams, acoustic_heat_map,
    coupled_decay_to_lines, directivity_balloon_to_mesh, mode_pattern_to_mesh, portal_to_lines,
    pressure_map_slice, ray_paths_to_lines,
};
use soorat::color::Color;
use soorat::debug_draw::LineBatch;
use soorat::vertex::Vertex3D;

/// A ray path: each bounce's point and 8 band energies.
type Path = Vec<([f32; 3], [f32; 8])>;
/// A portal: position, normal, width, height, arrow length.
type PortalCase = ([f32; 3], [f32; 3], f32, f32, f32);
/// A pressure slice: dimensions, values, origin, spacing, y index, colour mode, alpha, note.
type PmCase = (
    [usize; 3],
    Vec<f32>,
    [f32; 3],
    f32,
    usize,
    usize,
    f32,
    String,
);
/// A room mode: dimensions, pattern, width, depth, height scale, colour mode, alpha, note.
type ModeCase = ([usize; 2], Vec<f32>, f32, f32, f32, usize, f32, String);
/// A balloon: azimuths, elevations, band gains, centre, front, base radius, gain scale, lat, lon, note.
type BalloonCase = (
    Vec<f32>,
    Vec<f32>,
    [Vec<f32>; 8],
    [f32; 3],
    [f32; 3],
    f32,
    f32,
    u32,
    u32,
    String,
);

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

fn mesh(v: &[Vertex3D], ix: &[u32]) -> Toks {
    let f: Vec<f32> = v
        .iter()
        .flat_map(|v| {
            [
                v.position[0],
                v.position[1],
                v.position[2],
                v.normal[0],
                v.normal[1],
                v.normal[2],
                v.tex_coords[0],
                v.tex_coords[1],
                v.color[0],
                v.color[1],
                v.color[2],
                v.color[3],
            ]
        })
        .collect();
    let i: Vec<i128> = ix.iter().map(|&x| x as i128).collect();
    Toks::new().gs(&f).is(&i)
}

const MODES: [AcousticColorMode; 3] = [
    AcousticColorMode::Pressure,
    AcousticColorMode::SignedPressure,
    AcousticColorMode::Energy,
];

fn params(mode: usize, alpha: f32) -> AcousticVisParams {
    AcousticVisParams {
        color_mode: MODES[mode],
        base_color: Color::CORNFLOWER_BLUE,
        alpha,
    }
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

    // ── pressure-map slices (part b) ──
    let o = [0.0f32, 0.0, 0.0];
    let cube: Vec<f32> = (0..8).map(|k| k as f32 - 3.5).collect();
    let mut odd: Vec<f32> = (0..60).map(|k| (k as f32 * 0.37).sin()).collect();
    odd[3] = f32::NAN;
    odd[17] = f32::INFINITY;
    odd[22] = f32::NEG_INFINITY;
    let mut slices: Vec<PmCase> = vec![
        (
            [0, 1, 1],
            vec![],
            o,
            1.0,
            0,
            0,
            1.0,
            "an empty grid".to_string(),
        ),
        (
            [2, 2, 2],
            vec![0.5; 7],
            o,
            1.0,
            0,
            0,
            1.0,
            "values short of the grid".to_string(),
        ),
        (
            [2, 2, 2],
            vec![0.5; 8],
            o,
            1.0,
            2,
            0,
            1.0,
            "y out of range".to_string(),
        ),
        (
            [1, 1, 1],
            vec![0.25],
            o,
            1.0,
            0,
            0,
            1.0,
            "one cell".to_string(),
        ),
        (
            [3, 1, 2],
            vec![2.0; 6],
            o,
            1.0,
            0,
            0,
            1.0,
            "a flat slice (range 0)".to_string(),
        ),
        (
            [3, 1, 2],
            vec![f32::NAN; 6],
            o,
            1.0,
            0,
            1,
            1.0,
            "no finite value".to_string(),
        ),
        (
            [2, 7, 2],
            (0..28).map(|k| k as f32).collect(),
            [0.1, 0.3, 0.7],
            0.1,
            6,
            0,
            1.0,
            "y·spacing rounds".to_string(),
        ),
    ];
    for y in 0..2 {
        for mode in 0..3 {
            slices.push((
                [2, 2, 2],
                cube.clone(),
                [1.0, 2.0, -1.5],
                0.5,
                y,
                mode,
                0.75,
                format!("2³, y {y}, mode {mode}"),
            ));
        }
    }
    for mode in 0..3 {
        slices.push((
            [4, 3, 5],
            odd.clone(),
            [-2.0, 0.5, 3.0],
            0.25,
            1,
            mode,
            1.0,
            format!("non-finite values, mode {mode}"),
        ));
    }
    for k in 0..60 {
        let dims = [
            (r.next() % 5) as usize + 1,
            (r.next() % 5) as usize + 1,
            (r.next() % 5) as usize + 1,
        ];
        let n = dims[0] * dims[1] * dims[2];
        let vals: Vec<f32> = (0..n).map(|_| r.f()).collect();
        let y = (r.next() % dims[1] as u64) as usize;
        let org = v3(&mut r);
        let sp = r.pos() / 4.0;
        let mode = (r.next() % 3) as usize;
        let alpha = r.pos() / 16.0;
        slices.push((dims, vals, org, sp, y, mode, alpha, format!("random {k}")));
    }
    for (dims, values, origin, sp, y, mode, alpha, note) in &slices {
        let pm = PressureMap {
            values: values.clone(),
            dimensions: *dims,
            origin: vec3(*origin),
            spacing: *sp,
            frequency_hz: 500.0,
        };
        let (v, i) = pressure_map_slice(&pm, *y, &params(*mode, *alpha));
        m.row(
            "pressure_map_slice",
            'E',
            &Toks::new()
                .i(dims[0] as i128)
                .i(dims[1] as i128)
                .i(dims[2] as i128)
                .gs(values)
                .gs(origin)
                .g(*sp)
                .i(*y as i128)
                .i(*mode as i128)
                .g(*alpha),
            &mesh(&v, &i),
            note,
        );
    }

    // ── room-mode patterns ──
    let pat: Vec<f32> = (0..9).map(|k| ((k as f32) * 0.7).cos()).collect();
    let mut pn = pat.clone();
    pn[4] = f32::NAN;
    pn[7] = f32::INFINITY;
    let mut modes: Vec<ModeCase> = vec![
        (
            [0, 0],
            vec![],
            1.0,
            1.0,
            1.0,
            0,
            1.0,
            "an empty pattern".to_string(),
        ),
        (
            [3, 3],
            vec![0.5; 8],
            1.0,
            1.0,
            1.0,
            0,
            1.0,
            "a pattern short of its dimensions".to_string(),
        ),
        (
            [1, 1],
            vec![0.5],
            2.0,
            2.0,
            1.0,
            0,
            1.0,
            "one sample".to_string(),
        ),
        (
            [1, 5],
            vec![0.0, 0.5, 1.0, 0.5, 0.0],
            2.0,
            3.0,
            0.5,
            0,
            1.0,
            "one column (dx 0)".to_string(),
        ),
        (
            [3, 3],
            pn,
            4.0,
            3.0,
            0.5,
            0,
            1.0,
            "non-finite samples".to_string(),
        ),
        (
            [4, 3],
            (0..12).map(|k| k as f32 / 6.0 - 1.0).collect(),
            0.3,
            0.7,
            0.1,
            1,
            1.0,
            "inexact steps".to_string(),
        ),
    ];
    for mode in 0..3 {
        modes.push((
            [3, 3],
            pat.clone(),
            4.0,
            3.0,
            0.5,
            mode,
            0.8,
            format!("3 × 3, mode {mode}"),
        ));
    }
    for k in 0..60 {
        let dims = [(r.next() % 6) as usize + 1, (r.next() % 6) as usize + 1];
        let p: Vec<f32> = (0..dims[0] * dims[1]).map(|_| r.f() / 8.0).collect();
        let (w, d, hs) = (r.pos(), r.pos(), r.pos() / 4.0);
        let mode = (r.next() % 3) as usize;
        let alpha = r.pos() / 16.0;
        modes.push((dims, p, w, d, hs, mode, alpha, format!("random {k}")));
    }
    for (dims, pattern, w, d, hs, mode, alpha, note) in &modes {
        let mv = ModeVisualization {
            frequency_hz: 100.0,
            mode_indices: [1, 0, 0],
            pattern: pattern.clone(),
            pattern_dimensions: *dims,
        };
        let (v, i) = mode_pattern_to_mesh(&mv, *w, *d, *hs, &params(*mode, *alpha));
        m.row(
            "mode_pattern_to_mesh",
            'E',
            &Toks::new()
                .i(dims[0] as i128)
                .i(dims[1] as i128)
                .gs(pattern)
                .g(*w)
                .g(*d)
                .g(*hs)
                .i(*mode as i128)
                .g(*alpha),
            &mesh(&v, &i),
            note,
        );
    }

    // ── directivity balloons ──
    let pi = std::f32::consts::PI;
    let az: Vec<f32> = (0..7).map(|k| k as f32 * pi / 6.0).collect();
    let el = vec![-0.5f32, 0.0, 0.5];
    let cardioid: [Vec<f32>; 8] = std::array::from_fn(|_| {
        (0..21)
            .map(|k| 0.5 + 0.5 * ((k % 7) as f32 * pi / 6.0).cos())
            .collect()
    });
    let z = [0.0f32, 0.0, 1.0];
    let mut holes = cardioid.clone();
    holes[2].truncate(5);
    holes[5][8] = f32::NAN;
    holes[6].clear();
    let mut aznan = az.clone();
    aznan[1] = f32::NAN;
    let mut balloons: Vec<BalloonCase> = vec![
        (
            az.clone(),
            el.clone(),
            cardioid.clone(),
            o,
            z,
            1.0,
            0.5,
            1,
            8,
            "lat below 2".to_string(),
        ),
        (
            az.clone(),
            el.clone(),
            cardioid.clone(),
            o,
            z,
            1.0,
            0.5,
            8,
            2,
            "lon below 3".to_string(),
        ),
        (
            az.clone(),
            el.clone(),
            cardioid.clone(),
            o,
            z,
            1.0,
            0.5,
            2,
            3,
            "the minimum".to_string(),
        ),
        (
            az.clone(),
            el.clone(),
            cardioid.clone(),
            [1.0, -2.0, 0.5],
            z,
            1.0,
            0.5,
            8,
            12,
            "a cardioid".to_string(),
        ),
        (
            az.clone(),
            el.clone(),
            cardioid.clone(),
            o,
            [1.0, 0.0, 0.0],
            0.5,
            1.0,
            6,
            8,
            "front +X".to_string(),
        ),
        (
            az.clone(),
            el.clone(),
            cardioid.clone(),
            o,
            [0.0, 0.0, 0.0],
            0.5,
            1.0,
            6,
            8,
            "a zero front (+Z)".to_string(),
        ),
        (
            az.clone(),
            el.clone(),
            cardioid.clone(),
            o,
            [f32::NAN, 0.0, 1.0],
            0.5,
            1.0,
            4,
            6,
            "a NaN front".to_string(),
        ),
        (
            vec![],
            el.clone(),
            cardioid.clone(),
            o,
            z,
            1.0,
            0.5,
            4,
            6,
            "no azimuths (gain 1)".to_string(),
        ),
        (
            az.clone(),
            vec![],
            cardioid.clone(),
            o,
            z,
            1.0,
            0.5,
            4,
            6,
            "no elevations (gain 1)".to_string(),
        ),
        (
            az.clone(),
            el.clone(),
            holes,
            o,
            z,
            1.0,
            0.5,
            6,
            8,
            "missing and NaN gains".to_string(),
        ),
        (
            aznan,
            vec![f32::NAN, 0.25, -0.25],
            cardioid.clone(),
            o,
            z,
            1.0,
            0.5,
            6,
            8,
            "NaN table entries".to_string(),
        ),
    ];
    for k in 0..40 {
        let naz = (r.next() % 10) as usize + 2;
        let nel = (r.next() % 4) as usize + 1;
        let a: Vec<f32> = (0..naz).map(|i| i as f32 * pi / (naz - 1) as f32).collect();
        let e: Vec<f32> = (0..nel).map(|_| r.f() / 8.0).collect();
        let g: [Vec<f32>; 8] =
            std::array::from_fn(|_| (0..naz * nel).map(|_| r.pos() / 16.0).collect());
        let lat = (r.next() % 12) as u32 + 2;
        let lon = (r.next() % 16) as u32 + 3;
        let (c, f) = (v3(&mut r), v3(&mut r));
        balloons.push((
            a,
            e,
            g,
            c,
            f,
            r.pos() / 4.0,
            r.pos() / 8.0,
            lat,
            lon,
            format!("random {k}"),
        ));
    }
    for (az, el, gains, center, front, base, scale, lat, lon, note) in &balloons {
        let b = DirectivityBalloon {
            azimuths: az.clone(),
            elevations: el.clone(),
            gains: gains.clone(),
        };
        let (v, i) = directivity_balloon_to_mesh(&b, *center, *front, *base, *scale, *lat, *lon);
        let mut t = Toks::new().gs(az).gs(el);
        for g in gains {
            t = t.gs(g);
        }
        m.row(
            "directivity_balloon_to_mesh",
            'E',
            &t.gs(center)
                .gs(front)
                .g(*base)
                .g(*scale)
                .i(*lat as i128)
                .i(*lon as i128),
            &mesh(&v, &i),
            note,
        );
    }
    m
}
