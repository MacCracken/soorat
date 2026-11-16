//! `thermo` and `thermo_render` (M5 part c): soorat 1.2.0's `thermo_render`.
//!
//! - `thermo` (class **A**, replayed by tests/parity.tcyr): `thermal_heat_map` and the two parameter structs'
//!   defaults. Rust compiles these without the `thermo` feature.
//! - `thermo_render` (class **E**, replayed by tests/parity_thermo.tcyr): the five renderers on ushma 1.4.0's types,
//!   which are f64 as ushma 2.0.0's are, so the inputs are any f64 (`f:`). A mesh is its vertices (12 f32 each) and
//!   indices; a `LineBatch` its vertices (7 f32 each). `validate.py --write` writes `thermo_render.e.tsv`: the formula
//!   in f64, rounded once (ADR 0002 #30).
//!
//! Inputs: a grid is `i:nx i:ny`, its values (`v:n f:…`), origin x and y, dx, dy, min and max (`f:`), then y_pos
//! (`g:`), the colour mode (`i:`), the base colour (`v:4 g:…`) and alpha (`g:`). A profile is its temperatures, dx,
//! origin (`v:3 f:…`), direction (`v:3 f:…`), min and max. A cycle is its T-s and P-v points (`v:2n f:…`, x then y),
//! then the diagram parameters (origin `v:3 g:…`, x and y scales, the two colours) and pv_offset. A network is its
//! node temperatures (`v:n f:…`), edges (`v:2m i:…`), conductances (`v:k f:…`), centre (`v:3 g:…`), radius and node
//! size. A flux field is `i:nx i:ny`, its vectors (`v:2n f:…`), dx, dy and max_magnitude (`f:`), y_pos and
//! arrow_scale (`g:`).

use crate::{Module, Rng, Toks};
use soorat::color::Color;
use soorat::debug_draw::LineBatch;
use soorat::thermo_render::{
    CycleDiagramParams, ThermalColorMode, ThermalVisParams, cycle_diagram_to_lines,
    heat_flux_to_arrows, temperature_profile_to_lines, thermal_grid_to_mesh, thermal_heat_map,
    thermal_network_to_lines,
};
use soorat::vertex::Vertex3D;
use ushma::integration::soorat::{
    CycleDiagramData, HeatFluxField, TemperatureProfile, ThermalGridVisualization,
    ThermalNetworkVisualization,
};

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

fn lines(b: &LineBatch) -> Toks {
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

/// A float in about ±64 with every bit of an f64 in play (not f32-representable).
fn wide(r: &mut Rng) -> f64 {
    ((r.next() >> 11) as f64 / (1u64 << 53) as f64 - 0.5) * 128.0
}

pub fn module() -> Module {
    let mut m = Module::new("thermo");
    for t in [0.0f32, 0.25, 0.5, 0.75, 1.0, -1.0, 2.0, f32::NAN, 0.3, 0.6] {
        m.row(
            "thermal_heat_map",
            'A',
            &Toks::new().g(t),
            &Toks::new().gs(&c4(thermal_heat_map(t))),
            &format!("t {t}"),
        );
    }
    let p = ThermalVisParams::default();
    let mode = match p.color_mode {
        ThermalColorMode::Temperature => "ThermalColorMode.Temperature",
        ThermalColorMode::AlphaBlend => "ThermalColorMode.AlphaBlend",
        _ => "ThermalColorMode.Unknown",
    };
    m.row(
        "ThermalVisParams::default",
        'A',
        &Toks::new(),
        &Toks::new().e(mode).gs(&c4(p.base_color)).g(p.alpha),
        "ThermalVisParams::default()",
    );
    let d = CycleDiagramParams::default();
    m.row(
        "CycleDiagramParams::default",
        'A',
        &Toks::new(),
        &Toks::new()
            .gs(&d.origin)
            .g(d.x_scale)
            .g(d.y_scale)
            .gs(&c4(d.ts_color))
            .gs(&c4(d.pv_color)),
        "CycleDiagramParams::default()",
    );
    m
}

/// A thermal grid: dimensions, values, origin, spacing, min, max, y_pos, colour mode, base colour, alpha, note.
type GridCase = (
    [usize; 2],
    Vec<f64>,
    [f64; 2],
    [f64; 2],
    f64,
    f64,
    f32,
    usize,
    Color,
    f32,
    String,
);
/// A temperature profile: temperatures, dx, origin, direction, min, max, note.
type ProfileCase = (Vec<f64>, f64, [f64; 3], [f64; 3], f64, f64, String);
/// A cycle: T-s points, P-v points, origin, x scale, y scale, pv_offset, note.
type CycleCase = (
    Vec<[f64; 2]>,
    Vec<[f64; 2]>,
    [f32; 3],
    f32,
    f32,
    f32,
    String,
);
/// A network: node temperatures, edges, conductances, centre, radius, node size, note.
type NetCase = (
    Vec<f64>,
    Vec<[usize; 2]>,
    Vec<f64>,
    [f32; 3],
    f32,
    f32,
    String,
);
/// A flux field: dimensions, vectors, spacing, max_magnitude, y_pos, arrow_scale, note.
type FluxCase = ([usize; 2], Vec<[f64; 2]>, [f64; 2], f64, f32, f32, String);

pub fn render_module() -> Module {
    let mut m = Module::new("thermo_render");
    let mut r = Rng(0x7E30_0005_0000_000C);
    let nan = f64::NAN;
    let inf = f64::INFINITY;

    // ── thermal grids ──
    let red = Color::RED;
    let mut grids: Vec<GridCase> = vec![
        (
            [0, 0],
            vec![],
            [0.0, 0.0],
            [1.0, 1.0],
            0.0,
            0.0,
            0.0,
            0,
            red,
            1.0,
            "an empty grid".into(),
        ),
        (
            [2, 2],
            vec![1.0; 3],
            [0.0, 0.0],
            [1.0, 1.0],
            0.0,
            1.0,
            0.0,
            0,
            red,
            1.0,
            "values short of the grid".into(),
        ),
        (
            [2, 2],
            vec![250.0, 500.0, nan, 350.0],
            [0.0, 0.0],
            [1.0, 1.0],
            300.0,
            400.0,
            0.0,
            1,
            red,
            1.0,
            "alpha blend out of range and NaN".into(),
        ),
        (
            [2, 2],
            vec![350.0; 4],
            [0.0, 0.0],
            [1.0, 1.0],
            350.0,
            350.0,
            0.0,
            0,
            red,
            1.0,
            "equal bounds".into(),
        ),
        (
            [2, 2],
            vec![300.0, 400.0, 500.0, 600.0],
            [0.0, 0.0],
            [0.5, 0.5],
            nan,
            600.0,
            1.0,
            0,
            red,
            1.0,
            "a NaN min".into(),
        ),
        (
            [3, 2],
            vec![300.0, inf, -inf, 310.0, 320.0, 330.0],
            [0.1, 0.2],
            [0.3, 0.7],
            300.0,
            330.0,
            0.25,
            0,
            red,
            0.5,
            "infinite values, inexact origin and spacing".into(),
        ),
    ];
    for k in 0..40 {
        let dims = [(r.next() % 5) as usize + 1, (r.next() % 5) as usize + 1];
        let vals: Vec<f64> = (0..dims[0] * dims[1])
            .map(|_| 300.0 + wide(&mut r))
            .collect();
        let (lo, hi) = (300.0 + wide(&mut r) / 4.0, 330.0 + wide(&mut r) / 4.0);
        let base = Color::new(r.pos() / 16.0, r.pos() / 16.0, r.pos() / 16.0, 1.0);
        let mode = (r.next() % 2) as usize;
        let y = r.f();
        let alpha = r.pos() / 16.0;
        grids.push((
            dims,
            vals,
            [wide(&mut r), wide(&mut r)],
            [wide(&mut r).abs() / 8.0, wide(&mut r).abs() / 8.0],
            lo,
            hi,
            y,
            mode,
            base,
            alpha,
            format!("random {k}"),
        ));
    }
    for (dims, vals, origin, sp, lo, hi, y, mode, base, alpha, note) in &grids {
        let g = ThermalGridVisualization {
            values: vals.clone(),
            dimensions: *dims,
            origin: *origin,
            spacing: *sp,
            min_temp: *lo,
            max_temp: *hi,
        };
        let params = ThermalVisParams {
            color_mode: if *mode == 0 {
                ThermalColorMode::Temperature
            } else {
                ThermalColorMode::AlphaBlend
            },
            base_color: *base,
            alpha: *alpha,
        };
        let (v, i) = thermal_grid_to_mesh(&g, *y, &params);
        let t = fs(Toks::new().i(dims[0] as i128).i(dims[1] as i128), vals)
            .f(origin[0])
            .f(origin[1])
            .f(sp[0])
            .f(sp[1])
            .f(*lo)
            .f(*hi)
            .g(*y)
            .i(*mode as i128)
            .gs(&c4(*base))
            .g(*alpha);
        m.row("thermal_grid_to_mesh", 'E', &t, &mesh(&v, &i), note);
    }

    // ── temperature profiles ──
    let x = [1.0, 0.0, 0.0];
    let mut profiles: Vec<ProfileCase> = vec![
        (vec![], 1.0, [0.0; 3], x, 0.0, 0.0, "no samples".into()),
        (
            vec![300.0],
            1.0,
            [0.0; 3],
            x,
            300.0,
            300.0,
            "one sample".into(),
        ),
        (
            vec![300.0, 350.0, 400.0, 450.0],
            1.0,
            [0.0; 3],
            x,
            300.0,
            450.0,
            "four samples".into(),
        ),
        (
            vec![300.0, 350.0],
            0.1,
            [0.3, -0.7, 1.1],
            [0.6, 0.8, 0.0],
            300.0,
            350.0,
            "inexact spacing and origin".into(),
        ),
        (
            vec![300.0, 300.0, 300.0],
            1.0,
            [0.0; 3],
            x,
            300.0,
            300.0,
            "a flat profile".into(),
        ),
        (
            vec![300.0, nan, 320.0],
            1.0,
            [0.0; 3],
            x,
            300.0,
            320.0,
            "a NaN sample".into(),
        ),
        (
            vec![300.0, 310.0],
            1.0,
            [0.0; 3],
            x,
            nan,
            nan,
            "NaN bounds".into(),
        ),
    ];
    for k in 0..40 {
        let n = (r.next() % 8) as usize + 2;
        let temps: Vec<f64> = (0..n).map(|_| 300.0 + wide(&mut r)).collect();
        let dir = [
            wide(&mut r) / 64.0,
            wide(&mut r) / 64.0,
            wide(&mut r) / 64.0,
        ];
        profiles.push((
            temps,
            wide(&mut r).abs() / 16.0,
            [wide(&mut r), wide(&mut r), wide(&mut r)],
            dir,
            300.0 + wide(&mut r) / 4.0,
            330.0 + wide(&mut r) / 4.0,
            format!("random {k}"),
        ));
    }
    for (temps, dx, origin, dir, lo, hi, note) in &profiles {
        let p = TemperatureProfile {
            temperatures: temps.clone(),
            dx: *dx,
            origin: *origin,
            direction: *dir,
            min_temp: *lo,
            max_temp: *hi,
        };
        let mut b = LineBatch::new();
        temperature_profile_to_lines(&p, &mut b);
        let t = fs(fs(fs(Toks::new(), temps).f(*dx), origin), dir)
            .f(*lo)
            .f(*hi);
        m.row("temperature_profile_to_lines", 'E', &t, &lines(&b), note);
    }

    // ── cycle diagrams ──
    let otto_ts = vec![[0.0, 300.0], [100.0, 600.0], [200.0, 300.0]];
    let otto_pv = vec![[0.001, 100000.0], [0.01, 50000.0], [0.001, 100000.0]];
    let mut cycles: Vec<CycleCase> = vec![
        (vec![], vec![], [0.0; 3], 1.0, 1.0, 5.0, "no points".into()),
        (
            vec![[0.0, 300.0]],
            vec![[0.001, 1e5]],
            [0.0; 3],
            1.0,
            1.0,
            5.0,
            "one point each".into(),
        ),
        (
            otto_ts.clone(),
            otto_pv.clone(),
            [0.0; 3],
            1.0,
            1.0,
            5.0,
            "an Otto cycle".into(),
        ),
        (
            otto_ts.clone(),
            otto_pv.clone(),
            [1.5, -2.0, 0.5],
            2.5,
            0.75,
            3.3,
            "moved and scaled".into(),
        ),
        (
            vec![[0.0, 300.0], [nan, 400.0], [200.0, 300.0], [300.0, inf]],
            otto_pv.clone(),
            [0.0; 3],
            1.0,
            1.0,
            5.0,
            "non-finite T-s points".into(),
        ),
        (
            vec![[nan, nan], [nan, nan]],
            otto_pv.clone(),
            [0.0; 3],
            1.0,
            1.0,
            5.0,
            "no finite T-s point".into(),
        ),
        (
            vec![[5.0, 300.0], [5.0, 300.0]],
            vec![],
            [0.0; 3],
            1.0,
            1.0,
            5.0,
            "a degenerate range".into(),
        ),
    ];
    for k in 0..40 {
        let n = (r.next() % 8) as usize + 2;
        let ts: Vec<[f64; 2]> = (0..n)
            .map(|_| [wide(&mut r), 300.0 + wide(&mut r)])
            .collect();
        let pv: Vec<[f64; 2]> = (0..n)
            .map(|_| [wide(&mut r).abs() / 1000.0, 1e5 + wide(&mut r) * 100.0])
            .collect();
        cycles.push((
            ts,
            pv,
            [r.f(), r.f(), r.f()],
            r.pos() / 4.0,
            r.pos() / 4.0,
            r.f(),
            format!("random {k}"),
        ));
    }
    for (ts, pv, origin, xs, ys, off, note) in &cycles {
        let c = CycleDiagramData {
            ts_points: ts.clone(),
            pv_points: pv.clone(),
            state_points: vec![],
            kind: "Otto".to_string(),
            efficiency: 0.5,
        };
        let params = CycleDiagramParams {
            origin: *origin,
            x_scale: *xs,
            y_scale: *ys,
            ..CycleDiagramParams::default()
        };
        let mut b = LineBatch::new();
        cycle_diagram_to_lines(&c, &params, *off, &mut b);
        let tsf: Vec<f64> = ts.iter().flatten().copied().collect();
        let pvf: Vec<f64> = pv.iter().flatten().copied().collect();
        let t = fs(fs(Toks::new(), &tsf), &pvf)
            .gs(origin)
            .g(*xs)
            .g(*ys)
            .gs(&c4(params.ts_color))
            .gs(&c4(params.pv_color))
            .g(*off);
        m.row("cycle_diagram_to_lines", 'E', &t, &lines(&b), note);
    }

    // ── thermal networks ──
    let mut nets: Vec<NetCase> = vec![
        (
            vec![],
            vec![],
            vec![],
            [0.0; 3],
            5.0,
            0.2,
            "no nodes".into(),
        ),
        (
            vec![300.0],
            vec![],
            vec![],
            [0.0; 3],
            5.0,
            0.2,
            "one node".into(),
        ),
        (
            vec![300.0, 350.0, 400.0],
            vec![[0, 1], [1, 2]],
            vec![0.1, 0.05],
            [0.0; 3],
            5.0,
            0.2,
            "three nodes".into(),
        ),
        (
            vec![300.0, 400.0],
            vec![[0, 1], [0, 5]],
            vec![0.1, 0.1],
            [1.0, 2.0, 3.0],
            2.5,
            0.3,
            "an edge to a missing node".into(),
        ),
        (
            vec![300.0, 310.0, 320.0, 330.0],
            vec![[0, 1], [1, 2], [2, 3], [3, 0]],
            vec![1.0],
            [0.0; 3],
            1.0,
            0.1,
            "fewer conductances than edges".into(),
        ),
        (
            vec![nan, 300.0, inf],
            vec![[0, 1], [1, 2]],
            vec![nan, 0.0],
            [0.0; 3],
            1.0,
            0.1,
            "non-finite values".into(),
        ),
        (
            vec![300.0; 4],
            vec![[0, 2]],
            vec![0.0],
            [0.0; 3],
            1.0,
            0.1,
            "equal temperatures, zero conductance".into(),
        ),
    ];
    for k in 0..40 {
        let n = (r.next() % 9) as usize + 1;
        let temps: Vec<f64> = (0..n).map(|_| 300.0 + wide(&mut r)).collect();
        let ne = (r.next() % 8) as usize;
        let edges: Vec<[usize; 2]> = (0..ne)
            .map(|_| {
                [
                    (r.next() % (n as u64 + 1)) as usize,
                    (r.next() % n as u64) as usize,
                ]
            })
            .collect();
        let conds: Vec<f64> = (0..ne).map(|_| wide(&mut r).abs()).collect();
        nets.push((
            temps,
            edges,
            conds,
            [r.f(), r.f(), r.f()],
            r.pos(),
            r.pos() / 16.0,
            format!("random {k}"),
        ));
    }
    for (temps, edges, conds, center, radius, ns, note) in &nets {
        let nw = ThermalNetworkVisualization {
            node_temperatures: temps.clone(),
            edges: edges.clone(),
            conductances: conds.clone(),
        };
        let mut b = LineBatch::new();
        thermal_network_to_lines(&nw, *center, *radius, *ns, &mut b);
        let ef: Vec<i128> = edges.iter().flatten().map(|&e| e as i128).collect();
        let t = fs(fs(Toks::new(), temps).is(&ef), conds)
            .gs(center)
            .g(*radius)
            .g(*ns);
        m.row("thermal_network_to_lines", 'E', &t, &lines(&b), note);
    }

    // ── heat-flux arrows ──
    let mut fluxes: Vec<FluxCase> = vec![
        (
            [0, 0],
            vec![],
            [1.0, 1.0],
            0.0,
            0.0,
            1.0,
            "an empty field".into(),
        ),
        (
            [2, 2],
            vec![[1.0, 0.0]; 3],
            [1.0, 1.0],
            1.0,
            0.0,
            1.0,
            "vectors short of the grid".into(),
        ),
        (
            [2, 2],
            vec![[100.0, 0.0], [0.0, -50.0], [50.0, 50.0], [0.0, 0.0]],
            [1.0, 1.0],
            100.0,
            0.0,
            1.0,
            "the basic field".into(),
        ),
        (
            [2, 1],
            vec![[nan, 0.0], [50.0, 0.0]],
            [1.0, 1.0],
            0.0,
            0.0,
            1.0,
            "NaN, and a stale maximum".into(),
        ),
        (
            [2, 1],
            vec![[0.0, 0.0], [0.0, 0.0]],
            [1.0, 1.0],
            0.0,
            0.0,
            1.0,
            "zero vectors".into(),
        ),
        (
            [2, 1],
            vec![[10.0, 0.0], [20.0, 0.0]],
            [1.0, 1.0],
            5.0,
            0.5,
            2.0,
            "a maximum below the vectors".into(),
        ),
        (
            [1, 1],
            vec![[1e-8, 0.0]],
            [1.0, 1.0],
            nan,
            0.0,
            1.0,
            "below f32::EPSILON, a NaN maximum".into(),
        ),
        (
            [1, 1],
            vec![[1e30, 1e30]],
            [1.0, 1.0],
            inf,
            0.0,
            1.0,
            "f32 overflows, an infinite maximum".into(),
        ),
        (
            [3, 1],
            vec![[0.3, 0.4], [0.1, -0.2], [inf, 1.0]],
            [0.1, 0.7],
            -1.0,
            1.5,
            0.25,
            "inexact spacing".into(),
        ),
    ];
    for k in 0..40 {
        let dims = [(r.next() % 5) as usize + 1, (r.next() % 5) as usize + 1];
        let vecs: Vec<[f64; 2]> = (0..dims[0] * dims[1])
            .map(|_| [wide(&mut r), wide(&mut r)])
            .collect();
        let mm = if r.next().is_multiple_of(2) {
            0.0
        } else {
            wide(&mut r).abs()
        };
        fluxes.push((
            dims,
            vecs,
            [wide(&mut r).abs() / 8.0, wide(&mut r).abs() / 8.0],
            mm,
            r.f(),
            r.pos() / 4.0,
            format!("random {k}"),
        ));
    }
    for (dims, vecs, sp, mm, y, scale, note) in &fluxes {
        let f = HeatFluxField {
            fluxes: vecs.clone(),
            dimensions: *dims,
            spacing: *sp,
            max_magnitude: *mm,
        };
        let mut b = LineBatch::new();
        heat_flux_to_arrows(&f, *y, *scale, &mut b);
        let vf: Vec<f64> = vecs.iter().flatten().copied().collect();
        let t = fs(Toks::new().i(dims[0] as i128).i(dims[1] as i128), &vf)
            .f(sp[0])
            .f(sp[1])
            .f(*mm)
            .g(*y)
            .g(*scale);
        m.row("heat_flux_to_arrows", 'E', &t, &lines(&b), note);
    }
    m
}
