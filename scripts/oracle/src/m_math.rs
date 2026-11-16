//! `math`: soorat 1.2.0's `math_util` (through the `oracle` re-exports) on probe P2's grid.
//!
//! Every function here builds GPU-bound geometry, so the port computes it in f64 and
//! rounds once (D4 rule b): the rows are class **E**. Rust's f32 outputs are recorded
//! here; `scripts/oracle/validate.py --write` derives `math.e.tsv`, the same formulas in
//! f64 rounded once, for the rows where that differs. `flatten_mat4` only moves values:
//! class A.

use crate::{Module, Rng, Toks};
use soorat::oracle::{compose_trs, cross, flatten_mat4, look_at, mul_mat4, normal_to_basis, normalize3, perspective_90};

const N: usize = 200;

/// The edge vectors P2 used: zero, tiny (1e-12), huge (1e30), ±Y, and a NaN component.
fn edge3() -> [[f32; 3]; 6] {
    [
        [0.0, 0.0, 0.0],
        [f32::from_bits(0x2b8c_bccc), 0.0, 0.0],
        [f32::from_bits(0x7149_f2ca); 3],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [f32::NAN, 1.0, 2.0],
    ]
}

fn t3(v: [f32; 3]) -> Toks {
    Toks::new().gs(&v)
}

pub fn module() -> Module {
    let mut m = Module::new("math");
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);

    for i in 0..N {
        let a: [f32; 16] = std::array::from_fn(|_| r.f());
        let b: [f32; 16] = std::array::from_fn(|_| r.f());
        m.row("mul_mat4", 'E', &Toks::new().gs(&a).gs(&b), &Toks::new().gs(&mul_mat4(a, b)), &format!("case {i}"));
    }
    let edges = edge3();
    for i in 0..N + edges.len() {
        let v = if i < N { [r.f(), r.f(), r.f()] } else { edges[i - N] };
        m.row("normalize3", 'E', &t3(v), &t3(normalize3(v)), &format!("case {i}"));
    }
    for i in 0..N {
        let a = [r.f(), r.f(), r.f()];
        let b = [r.f(), r.f(), r.f()];
        m.row("cross", 'E', &t3(a).gs(&b), &t3(cross(a, b)), &format!("case {i}"));
    }
    for i in 0..N + 1 {
        let (near, far) = if i < N {
            let n = r.pos();
            (n, n + r.pos() * 64.0)
        } else {
            (1.0, 1.0)
        };
        m.row(
            "perspective_90",
            'E',
            &Toks::new().g(near).g(far),
            &Toks::new().gs(&perspective_90(near, far)),
            &format!("case {i}"),
        );
    }
    for i in 0..N + 3 {
        let (pos, dir, up) = match i {
            _ if i < N => ([r.f(), r.f(), r.f()], [r.f(), r.f(), r.f()], [r.f(), r.f(), r.f()]),
            n if n == N => ([1.0, 2.0, 3.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]),
            n if n == N + 1 => ([1.0, 2.0, 3.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            _ => ([0.0; 3], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
        };
        m.row(
            "look_at",
            'E',
            &t3(pos).gs(&dir).gs(&up),
            &Toks::new().gs(&look_at(pos, dir, up)),
            &format!("case {i}"),
        );
    }
    for i in 0..N {
        let t = [r.f(), r.f(), r.f()];
        let q = [r.f(), r.f(), r.f(), r.f()];
        let s = [r.f(), r.f(), r.f()];
        m.row(
            "compose_trs",
            'E',
            &t3(t).gs(&q).gs(&s),
            &Toks::new().gs(&compose_trs(t, q, s)),
            &format!("case {i}"),
        );
    }
    for i in 0..N + edges.len() {
        let n = if i < N { normalize3([r.f(), r.f(), r.f()]) } else { edges[i - N] };
        let (right, up) = normal_to_basis(n);
        m.row("normal_to_basis", 'E', &t3(n), &t3(right).gs(&up), &format!("case {i}"));
    }
    for i in 0..20 {
        let cols: [[f32; 4]; 4] = std::array::from_fn(|_| std::array::from_fn(|_| r.f()));
        let flat: Vec<f32> = cols.iter().flatten().copied().collect();
        m.row("flatten_mat4", 'A', &Toks::new().gs(&flat), &Toks::new().gs(&flatten_mat4(cols)), &format!("case {i}"));
    }
    m
}
