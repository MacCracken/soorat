//! `fluid` and `fluid_render` (M5 part d): soorat 1.2.0's `fluid_render`.
//!
//! - `fluid` (class **A**, replayed by tests/parity.tcyr): `fluid_heat_map` and `ParticleColorParams::default`.
//! - `fluid_render` (class **E**, replayed by tests/parity_fluids.tcyr): `particles_to_quads`, `particles_to_billboards`
//!   and `shallow_water_to_mesh` on pravash 1.3.0's types, which are f64 as pravash 2.0.1's are, so the inputs are any
//!   f64 (`f:`). A mesh is its vertices (12 f32 each) then its indices. `validate.py --write` writes
//!   `fluid_render.e.tsv`: the formula in f64, rounded once (ADR 0002 #31).
//!
//! Inputs: particles are `i:<n>` then, per particle, position and velocity (`v:3 f:…`), density and pressure (`f:`).
//! `particles_to_quads` then takes size (`g:`), the colour mode (`i:`), the base colour (`v:4 g:…`), max_velocity
//! (`g:`), max_density and max_pressure (`f:`); `particles_to_billboards` size, camera right and up (`v:3 g:…`), then
//! the same colour parameters. A water surface is `i:nx i:ny f:dx`, its heights (`v:n f:…`), y_scale (`g:`) and the
//! colour.

use crate::{Module, Rng, Toks};
use pravash::common::FluidParticle;
use pravash::shallow::ShallowWater;
use soorat::color::Color;
use soorat::fluid_render::{
    FluidColorMode, ParticleColorParams, fluid_heat_map, particles_to_billboards,
    particles_to_quads, shallow_water_to_mesh,
};
use soorat::vertex::Vertex3D;

const MODES: [FluidColorMode; 4] = [
    FluidColorMode::Solid,
    FluidColorMode::Velocity,
    FluidColorMode::Density,
    FluidColorMode::Pressure,
];

fn c4(c: Color) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

fn fs(t: Toks, xs: &[f64]) -> Toks {
    let mut t = t.seq(xs.len());
    for &x in xs {
        t = t.f(x);
    }
    t
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

/// A float in about ±64 with every bit of an f64 in play.
fn wide(r: &mut Rng) -> f64 {
    ((r.next() >> 11) as f64 / (1u64 << 53) as f64 - 0.5) * 128.0
}

/// A particle: position, velocity, density, pressure.
type Part = ([f64; 3], [f64; 3], f64, f64);
/// A quads row: particles, size, colour mode, max_velocity, max_density, max_pressure, note.
type QuadCase = (Vec<Part>, f32, usize, f32, f64, f64, String);
/// A billboards row: particles, size, camera right, camera up, colour mode, note.
type BoardCase = (Vec<Part>, f32, [f32; 3], [f32; 3], usize, String);

fn particles(ps: &[Part]) -> (Vec<FluidParticle>, Toks) {
    let mut t = Toks::new().i(ps.len() as i128);
    let mut out = Vec::new();
    for (pos, vel, d, pr) in ps {
        let mut p = FluidParticle::new((*pos).into(), 1.0);
        p.velocity = (*vel).into();
        p.density = *d;
        p.pressure = *pr;
        t = fs(fs(t, pos), vel).f(*d).f(*pr);
        out.push(p);
    }
    (out, t)
}

fn random_parts(r: &mut Rng, n: usize) -> Vec<Part> {
    (0..n)
        .map(|_| {
            (
                [wide(r), wide(r), wide(r)],
                [wide(r) / 8.0, wide(r) / 8.0, wide(r) / 8.0],
                1000.0 + wide(r) * 4.0,
                wide(r) * 20.0,
            )
        })
        .collect()
}

pub fn module() -> Module {
    let mut m = Module::new("fluid");
    for t in [0.0f32, 0.5, 1.0, -1.0, 2.0, f32::NAN, 0.3] {
        m.row(
            "fluid_heat_map",
            'A',
            &Toks::new().g(t),
            &Toks::new().gs(&c4(fluid_heat_map(t))),
            &format!("t {t}"),
        );
    }
    let p = ParticleColorParams::default();
    let mode = match p.color_mode {
        FluidColorMode::Solid => "FluidColorMode.Solid",
        FluidColorMode::Velocity => "FluidColorMode.Velocity",
        FluidColorMode::Density => "FluidColorMode.Density",
        FluidColorMode::Pressure => "FluidColorMode.Pressure",
        _ => "FluidColorMode.Unknown",
    };
    m.row(
        "ParticleColorParams::default",
        'A',
        &Toks::new(),
        &Toks::new()
            .e(mode)
            .gs(&c4(p.base_color))
            .g(p.max_velocity)
            .f(p.max_density)
            .f(p.max_pressure),
        "ParticleColorParams::default()",
    );
    m
}

pub fn render_module() -> Module {
    let mut m = Module::new("fluid_render");
    let mut r = Rng(0xF1D0_0005_0000_000D);
    let nan = f64::NAN;
    let inf = f64::INFINITY;
    let base = Color::BLUE;

    // ── particle quads ──
    let one: Vec<Part> = vec![([1.0, 2.0, 3.0], [10.0, 0.0, 0.0], 1000.0, 500.0)];
    let odd: Vec<Part> = vec![
        ([0.1, 0.2, 0.3], [3.0, 4.0, 0.0], 998.2, 101.325),
        ([nan, 0.0, 0.0], [nan, 0.0, 0.0], nan, nan),
        ([inf, -inf, 0.0], [inf, 0.0, 0.0], inf, -inf),
        ([1e30, -1e30, 1e-30], [1e30, 1e30, 0.0], 1e300, -1e300),
    ];
    let mut quads: Vec<QuadCase> = vec![
        (vec![], 0.1, 0, 1.0, 1.0, 1.0, "no particles".into()),
        (one.clone(), 0.5, 0, 1.0, 1000.0, 1.0, "one, solid".into()),
        (
            one.clone(),
            0.1,
            1,
            10.0,
            1.0,
            1.0,
            "one, at max velocity".into(),
        ),
        (one.clone(), 0.1, 2, 1.0, 2000.0, 1.0, "one, density".into()),
        (
            one.clone(),
            0.1,
            3,
            1.0,
            1.0,
            1000.0,
            "one, pressure".into(),
        ),
        (
            one.clone(),
            0.1,
            1,
            0.0,
            0.0,
            0.0,
            "zero maxima (floored at 0.001)".into(),
        ),
        (one.clone(), 0.1, 1, f32::NAN, nan, nan, "NaN maxima".into()),
        (
            one.clone(),
            0.3,
            1,
            7.0,
            3.0,
            3.0,
            "an inexact speed ratio".into(),
        ),
    ];
    for mode in 0..4 {
        quads.push((
            odd.clone(),
            0.25,
            mode,
            5.0,
            1000.0,
            200.0,
            format!("odd values, mode {mode}"),
        ));
    }
    for k in 0..40 {
        let n = (r.next() % 6) as usize + 1;
        let ps = random_parts(&mut r, n);
        let mode = (r.next() % 4) as usize;
        quads.push((
            ps,
            r.pos() / 4.0,
            mode,
            r.pos(),
            1000.0 + wide(&mut r),
            wide(&mut r).abs(),
            format!("random {k}"),
        ));
    }
    for (ps, size, mode, mv, md, mp, note) in &quads {
        let (parts, t) = particles(ps);
        let (v, i) = particles_to_quads(&parts, *size, MODES[*mode], base, *mv, *md, *mp);
        let t = t
            .g(*size)
            .i(*mode as i128)
            .gs(&c4(base))
            .g(*mv)
            .f(*md)
            .f(*mp);
        m.row("particles_to_quads", 'E', &t, &mesh(&v, &i), note);
    }

    // ── particle billboards ──
    let x = [1.0f32, 0.0, 0.0];
    let y = [0.0f32, 1.0, 0.0];
    let mut boards: Vec<BoardCase> = vec![
        (vec![], 1.0, x, y, 0, "no particles".into()),
        (
            vec![([0.0, 1.0, 0.0], [0.0; 3], 0.0, 0.0)],
            1.0,
            x,
            y,
            0,
            "one, facing +Z".into(),
        ),
        (
            one.clone(),
            0.5,
            [0.6, 0.0, 0.8],
            [0.0, 1.0, 0.0],
            1,
            "a turned camera".into(),
        ),
        (
            one.clone(),
            0.5,
            [0.3, 0.1, 0.7],
            [0.2, 0.9, 0.1],
            2,
            "a skew basis".into(),
        ),
        (
            one.clone(),
            0.5,
            [0.0; 3],
            [0.0; 3],
            3,
            "a zero basis".into(),
        ),
    ];
    for mode in 0..4 {
        boards.push((
            odd.clone(),
            0.25,
            [0.6, 0.0, 0.8],
            y,
            mode,
            format!("odd values, mode {mode}"),
        ));
    }
    for k in 0..40 {
        let n = (r.next() % 6) as usize + 1;
        let ps = random_parts(&mut r, n);
        let rv = [r.f(), r.f(), r.f()];
        let uv = [r.f(), r.f(), r.f()];
        boards.push((
            ps,
            r.pos() / 4.0,
            rv,
            uv,
            (r.next() % 4) as usize,
            format!("random {k}"),
        ));
    }
    for (ps, size, rv, uv, mode, note) in &boards {
        let (parts, t) = particles(ps);
        let params = ParticleColorParams {
            color_mode: MODES[*mode],
            base_color: base,
            max_velocity: 8.0,
            max_density: 1100.0,
            max_pressure: 900.0,
        };
        let (v, i) = particles_to_billboards(&parts, *size, *rv, *uv, &params);
        let t = t
            .g(*size)
            .gs(rv)
            .gs(uv)
            .i(*mode as i128)
            .gs(&c4(base))
            .g(params.max_velocity)
            .f(params.max_density)
            .f(params.max_pressure);
        m.row("particles_to_billboards", 'E', &t, &mesh(&v, &i), note);
    }

    // ── shallow water ──
    let mut waters: Vec<(usize, usize, f64, Vec<f64>, f32, String)> = vec![
        (1, 3, 1.0, vec![1.0; 3], 1.0, "one column".into()),
        (3, 1, 1.0, vec![1.0; 3], 1.0, "one row".into()),
        (3, 3, 0.0, vec![1.0; 9], 1.0, "dx 0".into()),
        (3, 3, -1.0, vec![1.0; 9], 1.0, "dx negative".into()),
        (3, 3, nan, vec![1.0; 9], 1.0, "dx NaN".into()),
        (3, 3, 1e300, vec![1.0; 9], 1.0, "dx past f32".into()),
        (3, 3, 1e-50, vec![1.0; 9], 1.0, "dx below f32".into()),
        (
            3,
            3,
            1.0,
            vec![1.0; 4],
            1.0,
            "heights short of the grid".into(),
        ),
        (2, 2, 1.0, vec![0.5, 0.75, 1.0, 1.25], 2.0, "2 × 2".into()),
        (
            4,
            3,
            0.5,
            (0..12).map(|k| 1.0 + 0.1 * (k as f64).sin()).collect(),
            1.0,
            "the centred surface".into(),
        ),
        (
            3,
            3,
            0.1,
            vec![1.0, nan, inf, 1.0, 1.0, -inf, 1.0, 1.0, 1.0],
            0.5,
            "non-finite heights".into(),
        ),
        (4, 4, 1.0, vec![1.0; 16], 1.0, "a flat surface".into()),
    ];
    for k in 0..30 {
        let (nx, ny) = ((r.next() % 6) as usize + 2, (r.next() % 6) as usize + 2);
        let hs: Vec<f64> = (0..nx * ny).map(|_| wide(&mut r)).collect();
        waters.push((
            nx,
            ny,
            wide(&mut r).abs() / 16.0,
            hs,
            r.f(),
            format!("random {k}"),
        ));
    }
    for (nx, ny, dx, hs, ys, note) in &waters {
        let mut w = ShallowWater::new(3, 3, 1.0, 1.0).expect("a 3 × 3 water");
        w.nx = *nx;
        w.ny = *ny;
        w.dx = *dx;
        w.height = hs.clone();
        let (v, i) = shallow_water_to_mesh(&w, *ys, base);
        let t = fs(Toks::new().i(*nx as i128).i(*ny as i128).f(*dx), hs)
            .g(*ys)
            .gs(&c4(base));
        m.row("shallow_water_to_mesh", 'E', &t, &mesh(&v, &i), note);
    }
    m
}
