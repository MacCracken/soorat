//! `lod`: soorat 1.2.0's `lod` module (M1 part c). Its outputs are indices and resolutions chosen by f32
//! comparisons, and `LodChain::new`'s squared distances: Rust's f32 sequence, class **A** (D4 rule a).

use crate::{Module, Rng, Toks};
use soorat::lod::{LodChain, TerrainLod};

fn next_up(x: f32) -> f32 {
    if x == 0.0 {
        f32::from_bits(1)
    } else if x > 0.0 {
        f32::from_bits(x.to_bits() + 1)
    } else {
        f32::from_bits(x.to_bits() - 1)
    }
}

fn next_down(x: f32) -> f32 {
    -next_up(-x)
}

pub fn module() -> Module {
    let mut m = Module::new("lod");
    let mut r = Rng(0x10D0_C4A1_2026_1115);

    let mut dists: Vec<Vec<f32>> = vec![
        vec![],
        vec![100.0],
        vec![10.0, 30.0, 100.0],
        vec![f32::NAN, 5.0],
        vec![1e20, -3.0, f32::INFINITY],
    ];
    for _ in 0..20 {
        let n = (r.next() % 6) as usize + 1;
        dists.push((0..n).map(|_| r.pos() * 8.0).collect());
    }
    for (i, d) in dists.iter().enumerate() {
        let c = LodChain::new(d);
        let sq: Vec<f32> = c.levels.iter().map(|l| l.max_distance_sq).collect();
        let idx: Vec<i128> = c.levels.iter().map(|l| l.mesh_index as i128).collect();
        m.row(
            "LodChain::new",
            'A',
            &Toks::new().gs(d),
            &Toks::new().is(&idx).gs(&sq),
            &format!("chain {i}"),
        );
        m.row(
            "LodChain::level_count",
            'A',
            &Toks::new().gs(d),
            &Toks::new().i(c.level_count() as i128),
            &format!("chain {i}"),
        );
    }

    // select: each chain's thresholds ± 1 ulp, plus the edges.
    for (ci, d) in dists.iter().enumerate() {
        let c = LodChain::new(d);
        let mut qs = vec![0.0f32, -0.0, -1.0, f32::NAN, f32::INFINITY, f32::MAX];
        for l in &c.levels {
            let s = l.max_distance_sq;
            if s.is_finite() {
                qs.extend([next_down(s), s, next_up(s)]);
            }
        }
        for q in qs {
            m.row(
                "LodChain::select",
                'A',
                &Toks::new().gs(d).g(q),
                &Toks::new().i(c.select(q) as i128),
                &format!("chain {ci} at {q:?}"),
            );
        }
    }
    let c = LodChain::new(&[10.0, 50.0, 200.0]);
    for i in 0..300 {
        let cam = [r.f() * 16.0, r.f() * 16.0, r.f() * 16.0];
        let obj = [r.f() * 16.0, r.f() * 16.0, r.f() * 16.0];
        m.row(
            "LodChain::select_from_positions",
            'A',
            &Toks::new().gs(&[10.0, 50.0, 200.0]).gs(&cam).gs(&obj),
            &Toks::new().i(c.select_from_positions(cam, obj) as i128),
            &format!("case {i}"),
        );
    }

    let terrain: Vec<(Vec<u32>, Vec<f32>)> = vec![
        (vec![64, 32, 16, 8], vec![50.0, 100.0, 200.0, 500.0]),
        (vec![64, 8], vec![50.0, 200.0]),
        (vec![], vec![]),
        (vec![64, 32, 16], vec![50.0, 100.0]),
        (vec![64], vec![50.0, 100.0, 200.0]),
        (vec![7], vec![f32::NAN]),
    ];
    for (ti, (res, d)) in terrain.iter().enumerate() {
        let ri: Vec<i128> = res.iter().map(|&x| x as i128).collect();
        let ins = Toks::new().is(&ri).gs(d);
        match TerrainLod::new(res.clone(), d.clone()) {
            Ok(t) => {
                m.row(
                    "TerrainLod::new",
                    'A',
                    &ins,
                    &Toks::new().i(1),
                    &format!("terrain {ti}: Ok"),
                );
                let mut qs = vec![
                    0.0f32,
                    -1.0,
                    f32::NAN,
                    f32::INFINITY,
                    999.0,
                    30.0,
                    75.0,
                    150.0,
                    300.0,
                ];
                for &x in d {
                    if x.is_finite() {
                        qs.extend([next_down(x), x, next_up(x)]);
                    }
                }
                for q in qs {
                    m.row(
                        "TerrainLod::select_resolution",
                        'A',
                        &ins.clone().g(q),
                        &Toks::new().i(t.select_resolution(q) as i128),
                        &format!("terrain {ti} at {q:?}"),
                    );
                }
            }
            Err(e) => {
                let v = format!("{e:?}");
                let name = v.split('(').next().unwrap_or("?");
                m.row(
                    "TerrainLod::new",
                    'A',
                    &ins,
                    &Toks::new().err(name),
                    &format!("terrain {ti}: Err"),
                );
            }
        }
    }
    m
}
