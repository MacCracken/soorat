//! `render_graph` (M4 part c): `RenderGraph` driven by scripts of its public calls. Class A throughout.
//!
//! A row starts from a graph (`RenderGraph::new`, `default`, `forward` or `forward_with_effects`, the row's fn) and runs
//! its inputs, a list of calls, each an op code and its arguments:
//! - `i:0 s:<name> i:<type>`: `add_pass(name, TYPES[type])`;
//! - `i:1 s:<pass> s:<dep>`: `add_dependency(pass, dep)`;
//! - `i:2 s:<name> i:<0|1>`: `set_enabled(name, on)`;
//! - `i:3`: `execution_order()`, output as `v:<n>` and each name;
//! - `i:4`: `pass_count()` and `enabled_count()`;
//! - `i:5 s:<name>`: `get_pass(name)`: `n:`, or its name, `e:PassType.<V>`, enabled, and `v:<n>` dependency names.
//!
//! The outputs are the queries' results, in order. The scripts cover Rust's ten tests, cycles (two, three, a
//! self-dependency), dependencies on missing and disabled passes (including a chain through a disabled pass), names
//! added after their dependants, re-adding, duplicates, the empty name and non-ASCII names, a 300-pass chain, a
//! 100-pass fan-in, and 400 random scripts over an eight-name pool.

use crate::{Module, Rng, Toks};
use soorat::render_graph::{PassType, RenderGraph};

const TYPES: [PassType; 9] = [
    PassType::Shadow,
    PassType::Geometry,
    PassType::Ssao,
    PassType::Bloom,
    PassType::PostProcess,
    PassType::Debug,
    PassType::Ui,
    PassType::Compute,
    PassType::Custom,
];

fn type_name(t: PassType) -> &'static str {
    match t {
        PassType::Shadow => "PassType.Shadow",
        PassType::Geometry => "PassType.Geometry",
        PassType::Ssao => "PassType.Ssao",
        PassType::Bloom => "PassType.Bloom",
        PassType::PostProcess => "PassType.PostProcess",
        PassType::Debug => "PassType.Debug",
        PassType::Ui => "PassType.Ui",
        PassType::Compute => "PassType.Compute",
        PassType::Custom => "PassType.Custom",
        _ => "PassType.Unknown",
    }
}

#[derive(Clone)]
enum Op {
    Add(String, usize),
    Dep(String, String),
    Enable(String, bool),
    Order,
    Counts,
    Get(String),
}

fn add(n: &str, t: usize) -> Op {
    Op::Add(n.to_string(), t)
}
fn dep(p: &str, d: &str) -> Op {
    Op::Dep(p.to_string(), d.to_string())
}
fn en(n: &str, on: bool) -> Op {
    Op::Enable(n.to_string(), on)
}
fn get(n: &str) -> Op {
    Op::Get(n.to_string())
}

/// Runs `ops` on `g`; returns the input and output token lists.
fn run(g: &mut RenderGraph, ops: &[Op]) -> (Toks, Toks) {
    let mut ins = Toks::new();
    let mut outs = Toks::new();
    for op in ops {
        match op {
            Op::Add(n, t) => {
                ins = ins.i(0).s(n).i(*t as i128);
                g.add_pass(n.clone(), TYPES[*t]);
            }
            Op::Dep(p, d) => {
                ins = ins.i(1).s(p).s(d);
                g.add_dependency(p, d.clone());
            }
            Op::Enable(n, on) => {
                ins = ins.i(2).s(n).i(*on as i128);
                g.set_enabled(n, *on);
            }
            Op::Order => {
                ins = ins.i(3);
                let order = g.execution_order();
                outs = outs.seq(order.len());
                for name in order {
                    outs = outs.s(name);
                }
            }
            Op::Counts => {
                ins = ins.i(4);
                outs = outs.i(g.pass_count() as i128).i(g.enabled_count() as i128);
            }
            Op::Get(n) => {
                ins = ins.i(5).s(n);
                match g.get_pass(n) {
                    None => outs = outs.none(),
                    Some(p) => {
                        outs = outs
                            .s(&p.name)
                            .e(type_name(p.pass_type))
                            .i(p.enabled as i128)
                            .seq(p.dependencies.len());
                        for d in &p.dependencies {
                            outs = outs.s(d);
                        }
                    }
                }
            }
        }
    }
    (ins, outs)
}

/// Ends a script with the order, the counts and every pool name's pass.
fn finish(mut ops: Vec<Op>, pool: &[&str]) -> Vec<Op> {
    ops.push(Op::Order);
    ops.push(Op::Counts);
    for n in pool {
        ops.push(get(n));
    }
    ops
}

pub fn module() -> Module {
    let mut m = Module::new("render_graph");
    let mut r = Rng(0x5AD0_0004_0000_000C);

    // ── the starting graphs ──
    let std_pool = [
        "shadow",
        "geometry",
        "ssao",
        "bloom",
        "debug",
        "ui",
        "post_process",
        "missing",
    ];
    let q = finish(Vec::new(), &std_pool);
    for (fname, mut g) in [
        ("RenderGraph::new", RenderGraph::new()),
        ("RenderGraph::default", RenderGraph::default()),
        ("RenderGraph::forward", RenderGraph::forward()),
        (
            "RenderGraph::forward_with_effects",
            RenderGraph::forward_with_effects(),
        ),
    ] {
        let (i, o) = run(&mut g, &q);
        m.row(fname, 'A', &i, &o, "the graph as built, then queried");
    }
    // Changes to the built graphs: disable a pass in the middle, re-add one, depend on a disabled one.
    let tweaks: Vec<(&str, Vec<Op>)> = vec![
        ("disable ssao", vec![en("ssao", false)]),
        ("disable geometry", vec![en("geometry", false)]),
        (
            "disable shadow, re-add it",
            vec![en("shadow", false), add("shadow", 8)],
        ),
        (
            "ui depends on post_process (a cycle)",
            vec![dep("ui", "post_process")],
        ),
        (
            "a pass before shadow",
            vec![add("prepass", 7), dep("shadow", "prepass")],
        ),
    ];
    for (note, ops) in &tweaks {
        for (fname, ctor) in [
            (
                "RenderGraph::forward",
                RenderGraph::forward as fn() -> RenderGraph,
            ),
            (
                "RenderGraph::forward_with_effects",
                RenderGraph::forward_with_effects,
            ),
        ] {
            let mut g = ctor();
            let (i, o) = run(&mut g, &finish(ops.clone(), &std_pool));
            m.row(fname, 'A', &i, &o, note);
        }
    }

    // ── scripts ──
    let pool = ["a", "b", "c", "d", "e", "f", "g", "h"];
    let mut scripts: Vec<(&str, Vec<Op>)> = vec![
        (
            "re_adding_a_pass_updates_in_place",
            vec![
                add("a", 1),
                add("b", 6),
                dep("b", "a"),
                en("a", false),
                add("a", 8),
            ],
        ),
        ("empty_graph", vec![]),
        ("single_pass", vec![add("geometry", 1)]),
        (
            "dependency_ordering",
            vec![
                add("post", 4),
                add("geometry", 1),
                add("shadow", 0),
                dep("geometry", "shadow"),
                dep("post", "geometry"),
            ],
        ),
        (
            "disable_pass",
            vec![add("shadow", 0), add("geometry", 1), en("shadow", false)],
        ),
        (
            "cycle_detection",
            vec![add("a", 1), add("b", 4), dep("a", "b"), dep("b", "a")],
        ),
        (
            "a three-cycle and a pass after it",
            vec![
                add("a", 0),
                add("b", 1),
                add("c", 2),
                add("d", 3),
                dep("a", "b"),
                dep("b", "c"),
                dep("c", "a"),
                dep("d", "c"),
            ],
        ),
        (
            "a self-dependency",
            vec![add("a", 0), dep("a", "a"), add("b", 1), dep("b", "a")],
        ),
        (
            "a dependency on a missing pass",
            vec![add("a", 0), dep("a", "zz"), add("b", 1)],
        ),
        (
            "a chain through a disabled pass",
            vec![
                add("a", 0),
                add("b", 1),
                add("c", 2),
                dep("a", "b"),
                dep("b", "c"),
                en("b", false),
            ],
        ),
        (
            "a dependency of a pass not yet added is ignored",
            vec![dep("a", "b"), add("a", 0), add("b", 1)],
        ),
        (
            "a dependency on a pass added later",
            vec![add("a", 0), dep("a", "b"), add("b", 1)],
        ),
        (
            "a duplicate dependency",
            vec![add("a", 0), add("b", 1), dep("a", "b"), dep("a", "b")],
        ),
        (
            "enabling a missing pass",
            vec![en("a", true), add("b", 1), en("c", false)],
        ),
        (
            "re-adding keeps dependencies and re-enables",
            vec![
                add("a", 0),
                add("b", 1),
                dep("b", "a"),
                en("b", false),
                add("b", 5),
            ],
        ),
        (
            "every pass type",
            (0..9)
                .map(|t| add(pool.get(t).copied().unwrap_or("x"), t))
                .collect(),
        ),
        (
            "a diamond",
            vec![
                add("d", 4),
                add("b", 2),
                add("c", 3),
                add("a", 1),
                dep("d", "b"),
                dep("d", "c"),
                dep("b", "a"),
                dep("c", "a"),
            ],
        ),
        (
            "the order queried, the graph changed, the order queried again",
            vec![
                add("a", 0),
                add("b", 1),
                dep("a", "b"),
                Op::Order,
                en("b", false),
                Op::Order,
                en("b", true),
                dep("b", "a"),
                Op::Order,
                Op::Counts,
            ],
        ),
    ];
    let odd = ["", "x y", "ü", "100%", "tab\there", "a"];
    scripts.push((
        "the empty name and names that need escaping",
        vec![
            add("", 0),
            add("x y", 1),
            add("ü", 2),
            add("100%", 3),
            add("tab\there", 4),
            dep("x y", ""),
            dep("ü", "x y"),
            dep("", "100%"),
            dep("tab\there", "ü"),
        ],
    ));
    for (note, ops) in &scripts {
        let mut g = RenderGraph::new();
        let p: &[&str] = if note.starts_with("the empty name") {
            &odd
        } else {
            &["a", "b", "c", "d", "post", "geometry", "shadow", "zz"]
        };
        let (i, o) = run(&mut g, &finish(ops.clone(), p));
        m.row("RenderGraph::script", 'A', &i, &o, note);
    }

    // A 300-pass chain, each depending on the next (the search is 300 deep), and a 100-pass fan-in on one root.
    let names: Vec<String> = (0..300).map(|k| format!("p{k}")).collect();
    let mut ops: Vec<Op> = names.iter().map(|n| add(n, 1)).collect();
    for k in 0..299 {
        ops.push(dep(&names[k], &names[k + 1]));
    }
    let mut g = RenderGraph::new();
    let (i, o) = run(&mut g, &finish(ops, &["p0", "p299"]));
    m.row("RenderGraph::script", 'A', &i, &o, "a 300-pass chain");
    let mut ops: Vec<Op> = names.iter().take(100).map(|n| add(n, 2)).collect();
    for n in names.iter().take(100) {
        ops.push(dep(n, "root"));
    }
    ops.push(add("root", 0));
    let mut g = RenderGraph::new();
    let (i, o) = run(&mut g, &finish(ops, &["root", "p0"]));
    m.row("RenderGraph::script", 'A', &i, &o, "a 100-pass fan-in");

    // Random scripts over the pool.
    for k in 0..400 {
        let len = (r.next() % 30) as usize + 1;
        let mut ops = Vec::new();
        for _ in 0..len {
            let pick = |r: &mut Rng| pool[(r.next() % 8) as usize];
            match r.next() % 20 {
                0..=6 => ops.push(add(pick(&mut r), (r.next() % 9) as usize)),
                7..=13 => {
                    let p = pick(&mut r);
                    ops.push(dep(p, pick(&mut r)));
                }
                14..=16 => {
                    let n = pick(&mut r);
                    ops.push(en(n, !r.next().is_multiple_of(3)));
                }
                17..=18 => ops.push(Op::Order),
                _ => ops.push(Op::Counts),
            }
        }
        let mut g = RenderGraph::new();
        let (i, o) = run(&mut g, &finish(ops, &pool));
        m.row("RenderGraph::script", 'A', &i, &o, &format!("random {k}"));
    }
    m
}
