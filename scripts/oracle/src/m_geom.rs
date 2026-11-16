//! `primitives`, `viz`, `terrain` (M3 part a). A vertex is its 12 f32 (position, normal, UV, color); a mesh is its
//! vertices (`v:12n`) then its indices (`v:m`).
//!
//! Geometry is class **E** (D4 rule b): `validate.py --write` writes `<module>.e.tsv`, Rust's formula in f64 rounded
//! once, where that differs from Rust's f32 outputs. Copies, indices, counts and the f64 normalizations are class A;
//! the heat colours B. A heatmap's colour is `[ix / 16, iz / 16, 0.5, 1]` and a height field's `[y, ix, iz, 1]` (f32),
//! which the Cyrius dispatcher reproduces.

use crate::{Module, Rng, Toks};
use soorat::color::Color;
use soorat::oracle::{
    HeightField, XzGrid, finite_min_max, grid_is_backed, grid_len, heat_rgba, heightfield_mesh,
    lattice_indices, magnitude_t, signed_rgba, signed_t, unit_t, with_alpha, xz_heatmap_mesh,
};
use soorat::primitives::{cube, cylinder, plane, sphere};
use soorat::terrain::{TerrainConfig, flat_heightmap, generate_terrain};
use soorat::vertex::Vertex3D;

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

pub fn primitives() -> Module {
    let mut m = Module::new("primitives");
    let (v, i) = cube();
    m.row("cube", 'A', &Toks::new(), &mesh(&v, &i), "cube()");
    for (s, r) in [
        (0u32, 0u32),
        (1, 1),
        (3, 2),
        (4, 3),
        (7, 5),
        (8, 4),
        (16, 8),
        (24, 12),
        (32, 16),
        (64, 32),
    ] {
        let (v, i) = sphere(s, r);
        m.row(
            "sphere",
            'E',
            &Toks::new().i(s as i128).i(r as i128),
            &mesh(&v, &i),
            &format!("sphere({s}, {r})"),
        );
    }
    for d in [0u32, 1, 2, 3, 5, 10, 32] {
        let (v, i) = plane(d);
        m.row(
            "plane",
            'E',
            &Toks::new().i(d as i128),
            &mesh(&v, &i),
            &format!("plane({d})"),
        );
    }
    for s in [0u32, 3, 4, 5, 7, 8, 16, 32, 64, 100] {
        let (v, i) = cylinder(s);
        m.row(
            "cylinder",
            'E',
            &Toks::new().i(s as i128),
            &mesh(&v, &i),
            &format!("cylinder({s})"),
        );
    }
    m
}

pub fn viz() -> Module {
    let mut m = Module::new("viz");
    let mut r = Rng(0x0715_2026_1115);
    let d =
        |r: &mut Rng| -> f64 { ((r.next() >> 11) as f64 / 9_007_199_254_740_992.0 - 0.5) * 200.0 };
    let mut triples: Vec<[f64; 3]> = (0..150)
        .map(|_| [d(&mut r), d(&mut r), d(&mut r)])
        .collect();
    triples.extend([
        [1.0, 0.0, 2.0],
        [f64::NAN, 0.0, 1.0],
        [f64::INFINITY, 0.0, 1.0],
        [0.5, 1.0, 1.0],
        [0.5, 0.0, f64::EPSILON],
        [0.5, 0.0, 2.0 * f64::EPSILON],
        [0.5, f64::NAN, 1.0],
        [-3.0, -4.0, 2.0],
        [1e300, -1e300, 1e300],
        [0.0, 0.0, 0.0],
    ]);
    for (i, t) in triples.iter().enumerate() {
        let ins = Toks::new().f(t[0]).f(t[1]).f(t[2]);
        m.row(
            "unit_t",
            'A',
            &ins,
            &Toks::new().g(unit_t(t[0], t[1], t[2])),
            &format!("case {i}"),
        );
        m.row(
            "signed_t",
            'A',
            &ins,
            &Toks::new().g(signed_t(t[0], t[1], t[2])),
            &format!("case {i}"),
        );
        m.row(
            "magnitude_t",
            'A',
            &ins,
            &Toks::new().g(magnitude_t(t[0], t[1], t[2])),
            &format!("case {i}"),
        );
    }
    let sets: Vec<Vec<f64>> = vec![
        vec![],
        vec![f64::NAN],
        vec![1.0, f64::NAN, -2.0, f64::INFINITY],
        vec![-0.0, 0.0],
        (0..20).map(|_| d(&mut r)).collect(),
    ];
    for (i, s) in sets.iter().enumerate() {
        let out = match finite_min_max(s.iter().copied()) {
            Some((lo, hi)) => Toks::new().f(lo).f(hi),
            None => Toks::new().none(),
        };
        let mut ins = Toks::new().i(s.len() as i128);
        for &x in s {
            ins = ins.f(x);
        }
        m.row("finite_min_max", 'A', &ins, &out, &format!("set {i}"));
    }
    let dimsets: Vec<Vec<usize>> = vec![
        vec![],
        vec![0],
        vec![3, 4],
        vec![1 << 40, 1 << 30],
        vec![usize::MAX, 2],
        vec![7, 0, 9],
        vec![2, 3, 5],
    ];
    for (i, ds) in dimsets.iter().enumerate() {
        let ins = Toks::new().is(&ds.iter().map(|&x| x as i128).collect::<Vec<_>>());
        let out = match grid_len(ds) {
            Some(n) => Toks::new().i(n as i128),
            None => Toks::new().none(),
        };
        m.row("grid_len", 'A', &ins, &out, &format!("dims {i}"));
        for data_len in [0usize, 5, 12, 30, 1000] {
            m.row(
                "grid_is_backed",
                'A',
                &ins.clone().i(data_len as i128),
                &Toks::new().i(grid_is_backed(ds, data_len, "test") as i128),
                &format!("dims {i}, data {data_len}"),
            );
        }
    }
    for (i, &t) in [0.0f32, 0.1, 0.25, 0.6, 0.99, 1.5, -0.5, f32::NAN, -0.0]
        .iter()
        .enumerate()
    {
        let a = [1.0f32, 0.5, 0.0, f32::NAN][i % 4];
        m.row(
            "heat_rgba",
            'B',
            &Toks::new().g(t).g(a),
            &Toks::new().gs(&heat_rgba(t, a)),
            &format!("case {i}"),
        );
        m.row(
            "signed_rgba",
            'B',
            &Toks::new().g(t).g(a),
            &Toks::new().gs(&signed_rgba(t, a)),
            &format!("case {i}"),
        );
        let base = Color::new(0.1, 0.2, 0.3, 0.4);
        for alpha in [a, 2.0, -1.0, f32::INFINITY] {
            m.row(
                "with_alpha",
                'A',
                &Toks::new().gs(&base.to_array()).g(alpha),
                &Toks::new().gs(&with_alpha(base, alpha)),
                &format!("case {i}"),
            );
        }
    }
    let grids = [(1usize, 1usize), (2, 3), (5, 4), (0, 3), (3, 0), (16, 16)];
    for (i, &(nx, nz)) in grids.iter().enumerate() {
        let g = XzGrid {
            nx,
            nz,
            origin_x: r.f() * 10.0,
            origin_z: r.f() * 10.0,
            y: r.f(),
            dx: r.pos() / 3.0,
            dz: r.pos() / 7.0,
        };
        let (v, ix) = xz_heatmap_mesh(&g, |x, z| [x as f32 / 16.0, z as f32 / 16.0, 0.5, 1.0]);
        let ins = Toks::new()
            .i(nx as i128)
            .i(nz as i128)
            .gs(&[g.origin_x, g.origin_z, g.y, g.dx, g.dz]);
        m.row(
            "xz_heatmap_mesh",
            'E',
            &ins,
            &mesh(&v, &ix),
            &format!("grid {i}: {nx} × {nz}"),
        );
    }
    let fields = [
        (1usize, 1usize),
        (2, 2),
        (3, 2),
        (5, 4),
        (8, 8),
        (1, 6),
        (0, 3),
    ];
    for (i, &(cols, rows)) in fields.iter().enumerate() {
        let hf = HeightField {
            cols,
            rows,
            origin_x: r.f() * 10.0,
            origin_z: r.f() * 10.0,
            dx: r.pos() / 3.0,
            dz: if i == 4 { 0.0 } else { r.pos() / 5.0 },
        };
        let mut hs: Vec<f32> = (0..cols * rows).map(|_| r.f()).collect();
        if hs.len() > 3 {
            hs[1] = f32::NAN;
            hs[2] = f32::INFINITY;
        }
        let (v, ix) = heightfield_mesh(
            &hf,
            |x, z| hs[z * cols + x],
            |x, z, y| [y, x as f32, z as f32, 1.0],
        );
        let ins = Toks::new()
            .i(cols as i128)
            .i(rows as i128)
            .gs(&[hf.origin_x, hf.origin_z, hf.dx, hf.dz])
            .gs(&hs);
        m.row(
            "heightfield_mesh",
            'E',
            &ins,
            &mesh(&v, &ix),
            &format!("field {i}: {cols} × {rows}"),
        );
    }
    for (rows, cols) in [(0u32, 0u32), (1, 1), (2, 3), (4, 4), (3, 7)] {
        let ix: Vec<i128> = lattice_indices(rows, cols)
            .iter()
            .map(|&x| x as i128)
            .collect();
        m.row(
            "lattice_indices",
            'A',
            &Toks::new().i(rows as i128).i(cols as i128),
            &Toks::new().is(&ix),
            &format!("{rows} × {cols}"),
        );
    }
    m
}

pub fn terrain() -> Module {
    let mut m = Module::new("terrain");
    let mut r = Rng(0x7E77_2026_1115);
    let d = TerrainConfig::default();
    m.row(
        "TerrainConfig::default",
        'A',
        &Toks::new(),
        &Toks::new().i(d.width as i128).i(d.depth as i128).gs(&[
            d.world_width,
            d.world_depth,
            d.height_scale,
        ]),
        "default",
    );
    let configs = [
        (8u32, 8u32, 16.0f32, 16.0f32, 2.0f32),
        (16, 4, 30.0, 7.5, 0.5),
        (1, 1, 1.0, 1.0, 1.0),
        (0, 5, 10.0, 10.0, 1.0),
        (32, 32, 64.0, 64.0, 10.0),
        (5, 3, 1.0 / 3.0, 7.0, -1.0),
    ];
    for (i, &(w, dp, ww, wd, hs)) in configs.iter().enumerate() {
        let c = TerrainConfig {
            width: w,
            depth: dp,
            world_width: ww,
            world_depth: wd,
            height_scale: hs,
        };
        let n = (w as usize + 1) * (dp as usize + 1);
        let mut heights: Vec<f32> = (0..n).map(|_| r.f()).collect();
        if i == 1 {
            heights.truncate(n / 2);
        }
        if i == 4 && heights.len() > 10 {
            heights[3] = f32::NAN;
        }
        let t = generate_terrain(&c, &heights);
        let ins = Toks::new()
            .i(w as i128)
            .i(dp as i128)
            .gs(&[ww, wd, hs])
            .gs(&heights);
        m.row(
            "generate_terrain",
            'E',
            &ins,
            &mesh(&t.vertices, &t.indices),
            &format!("config {i}"),
        );
    }
    for (w, dp) in [(0u32, 0u32), (3, 4), (64, 64)] {
        m.row(
            "flat_heightmap",
            'A',
            &Toks::new().i(w as i128).i(dp as i128),
            &Toks::new().i(flat_heightmap(w, dp).len() as i128),
            &format!("{w} × {dp}"),
        );
    }
    m
}
