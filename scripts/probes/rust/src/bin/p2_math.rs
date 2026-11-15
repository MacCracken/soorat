//! P2 — f32 traps (soorat-port-plan §4 step 1).
//!
//! Prints soorat 1.2.0's `math_util` results (through the `oracle` re-exports)
//! for a deterministic input set, one line per case, every f32 as its bit
//! pattern in decimal. `scripts/probes/math/tests/p2_math.tcyr` is the Cyrius
//! port and prints the same lines; `p2_compare.py` diffs the two.
//!
//! Inputs come from an integer xorshift64 that both sides implement
//! identically, turned into f32 through exact conversions (|k| < 2²⁴, then a
//! power-of-two scale), so the inputs are bit-identical without a data file.
//!
//! With `--accuracy` it instead measures, in Rust alone, how far each f32
//! result is from the same formula evaluated in f64 and rounded once
//! (D4 rule b): the expected gain of the Cyrius f64 path.
//!
//! Run: `cargo run --release --bin p2_math [-- --accuracy]`.

use soorat::oracle::{compose_trs, cross, look_at, mul_mat4, normal_to_basis, normalize3, perspective_90};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// A float in about ±8 with 20 fractional bits: exact in f32 and f64.
    fn f(&mut self) -> f32 {
        let k = ((self.next() >> 40) as i64) - (1 << 23); // 24-bit signed
        k as f32 / 1_048_576.0
    }
    /// A positive float in (0, 16).
    fn pos(&mut self) -> f32 {
        let k = ((self.next() >> 40) as i64) | 1; // 1..2^24
        k as f32 / 1_048_576.0
    }
}

const N: usize = 200;

fn b(v: &[f32]) -> String {
    v.iter().map(|x| x.to_bits().to_string()).collect::<Vec<_>>().join(" ")
}

fn cases() -> Vec<(String, Vec<f32>)> {
    let mut out = Vec::new();
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let edge3: [[f32; 3]; 6] = [
        [0.0, 0.0, 0.0],
        [f32::from_bits(0x2b8c_bccc), 0.0, 0.0], // 1e-12
        [f32::from_bits(0x7149_f2ca), f32::from_bits(0x7149_f2ca), f32::from_bits(0x7149_f2ca)], // 1e30
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [f32::NAN, 1.0, 2.0],
    ];

    for i in 0..N {
        let a: [f32; 16] = std::array::from_fn(|_| r.f());
        let c: [f32; 16] = std::array::from_fn(|_| r.f());
        out.push((format!("mul_mat4 {i}"), mul_mat4(a, c).to_vec()));
    }
    for i in 0..N + edge3.len() {
        let v = if i < N { [r.f(), r.f(), r.f()] } else { edge3[i - N] };
        out.push((format!("normalize3 {i}"), normalize3(v).to_vec()));
    }
    for i in 0..N {
        let a = [r.f(), r.f(), r.f()];
        let c = [r.f(), r.f(), r.f()];
        out.push((format!("cross {i}"), cross(a, c).to_vec()));
    }
    for i in 0..N + 1 {
        let (near, far) = if i < N {
            let n = r.pos();
            (n, n + r.pos() * 64.0)
        } else {
            (1.0, 1.0)
        };
        out.push((format!("perspective_90 {i}"), perspective_90(near, far).to_vec()));
    }
    for i in 0..N + 3 {
        let (pos, dir, up) = match i {
            _ if i < N => ([r.f(), r.f(), r.f()], [r.f(), r.f(), r.f()], [r.f(), r.f(), r.f()]),
            n if n == N => ([1.0, 2.0, 3.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]), // dir ∥ up
            n if n == N + 1 => ([1.0, 2.0, 3.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]), // zero dir
            _ => ([0.0; 3], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
        };
        out.push((format!("look_at {i}"), look_at(pos, dir, up).to_vec()));
    }
    for i in 0..N {
        let t = [r.f(), r.f(), r.f()];
        let q = [r.f(), r.f(), r.f(), r.f()];
        let s = [r.f(), r.f(), r.f()];
        out.push((format!("compose_trs {i}"), compose_trs(t, q, s).to_vec()));
    }
    for i in 0..N + edge3.len() {
        let n = if i < N { normalize3([r.f(), r.f(), r.f()]) } else { edge3[i - N] };
        let (right, up) = normal_to_basis(n);
        let mut v = right.to_vec();
        v.extend(up);
        out.push((format!("normal_to_basis {i}"), v));
    }
    out
}

// ── f64 twins of the same formulas, for the accuracy measurement ─────────────

fn mul_mat4_64(a: [f64; 16], b: [f64; 16]) -> [f64; 16] {
    let mut r = [0.0; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[k * 4 + row] * b[col * 4 + k];
            }
            r[col * 4 + row] = sum;
        }
    }
    r
}
fn norm64(v: [f64; 3]) -> [f64; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-10 {
        return [0.0, 0.0, 1.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}
fn cross64(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn look_at64(pos: [f64; 3], dir: [f64; 3], up: [f64; 3]) -> [f64; 16] {
    let f = norm64(dir);
    let s = norm64(cross64(f, up));
    let u = cross64(s, f);
    [
        s[0], u[0], -f[0], 0.0, s[1], u[1], -f[1], 0.0, s[2], u[2], -f[2], 0.0,
        -(s[0] * pos[0] + s[1] * pos[1] + s[2] * pos[2]),
        -(u[0] * pos[0] + u[1] * pos[1] + u[2] * pos[2]),
        f[0] * pos[0] + f[1] * pos[1] + f[2] * pos[2],
        1.0,
    ]
}
fn compose64(t: [f64; 3], r: [f64; 4], s: [f64; 3]) -> [f64; 16] {
    let (x, y, z, w) = (r[0], r[1], r[2], r[3]);
    [
        s[0] * (1.0 - 2.0 * (y * y + z * z)),
        s[0] * (2.0 * (x * y + w * z)),
        s[0] * (2.0 * (x * z - w * y)),
        0.0,
        s[1] * (2.0 * (x * y - w * z)),
        s[1] * (1.0 - 2.0 * (x * x + z * z)),
        s[1] * (2.0 * (y * z + w * x)),
        0.0,
        s[2] * (2.0 * (x * z + w * y)),
        s[2] * (2.0 * (y * z - w * x)),
        s[2] * (1.0 - 2.0 * (x * x + y * y)),
        0.0,
        t[0], t[1], t[2], 1.0,
    ]
}

/// Distance in f32 ulps on the ordered-integer line (±0 one point).
fn ulps(a: f32, b: f32) -> u64 {
    let key = |x: f32| {
        let i = x.to_bits() as i32;
        if i < 0 { i32::MIN - i } else { i }
    };
    (i64::from(key(a)) - i64::from(key(b))).unsigned_abs()
}

fn histogram(name: &str, pairs: &[(f32, f32)]) {
    let mut h = [0usize; 6]; // 0, 1, 2, 3-4, 5-16, >16
    let mut worst = 0;
    for &(got, want) in pairs {
        let u = ulps(got, want);
        worst = worst.max(u);
        let k = match u {
            0 => 0,
            1 => 1,
            2 => 2,
            3..=4 => 3,
            5..=16 => 4,
            _ => 5,
        };
        h[k] += 1;
    }
    let n = pairs.len() as f64;
    println!(
        "{name:<34} n={:<6} exact {:5.1}%  1ulp {:5.1}%  2ulp {:4.1}%  3-4 {:4.1}%  5-16 {:4.1}%  >16 {:4.1}%  worst {worst}",
        pairs.len(),
        100.0 * h[0] as f64 / n,
        100.0 * h[1] as f64 / n,
        100.0 * h[2] as f64 / n,
        100.0 * h[3] as f64 / n,
        100.0 * h[4] as f64 / n,
        100.0 * h[5] as f64 / n,
    );
}

fn accuracy() {
    // Larger sample for the statistics; same generator.
    let mut r = Rng(0x0123_4567_89AB_CDEF);
    let m = 20_000;
    let up3 = |r: &mut Rng| [r.f(), r.f(), r.f()];
    let wid = |v: [f32; 3]| v.map(f64::from);

    let mut mm = Vec::new();
    let mut mm3 = Vec::new();
    for _ in 0..m {
        let a: [f32; 16] = std::array::from_fn(|_| r.f());
        let c: [f32; 16] = std::array::from_fn(|_| r.f());
        let d: [f32; 16] = std::array::from_fn(|_| r.f());
        let got = mul_mat4(a, c);
        let want = mul_mat4_64(a.map(f64::from), c.map(f64::from));
        mm.extend(got.iter().zip(want).map(|(&g, w)| (g, w as f32)));
        // A chain of three, as a joint hierarchy or MVP product does.
        let got3 = mul_mat4(mul_mat4(a, c), d);
        let want3 = mul_mat4_64(mul_mat4_64(a.map(f64::from), c.map(f64::from)), d.map(f64::from));
        mm3.extend(got3.iter().zip(want3).map(|(&g, w)| (g, w as f32)));
    }
    histogram("mul_mat4 (one product)", &mm);
    histogram("mul_mat4 (chain of three)", &mm3);

    let mut la = Vec::new();
    let mut la_t = Vec::new();
    for _ in 0..m {
        let (p, d, u) = (up3(&mut r), up3(&mut r), up3(&mut r));
        let got = look_at(p, d, u);
        let want = look_at64(wid(p), wid(d), wid(u));
        for (k, (&g, w)) in got.iter().zip(want).enumerate() {
            if k >= 12 && k < 15 {
                la_t.push((g, w as f32)); // translation column: dot products, cancellation
            } else {
                la.push((g, w as f32));
            }
        }
    }
    histogram("look_at (rotation part)", &la);
    histogram("look_at (translation column)", &la_t);

    let mut ct = Vec::new();
    for _ in 0..m {
        let t = up3(&mut r);
        let q = [r.f(), r.f(), r.f(), r.f()];
        let s = up3(&mut r);
        let got = compose_trs(t, q, s);
        let want = compose64(wid(t), q.map(f64::from), wid(s));
        ct.extend(got.iter().zip(want).map(|(&g, w)| (g, w as f32)));
    }
    histogram("compose_trs", &ct);

    let mut nb = Vec::new();
    for _ in 0..m {
        let v = up3(&mut r);
        let got = normalize3(v);
        let want = norm64(wid(v));
        nb.extend(got.iter().zip(want).map(|(&g, w)| (g, w as f32)));
    }
    histogram("normalize3", &nb);
}

fn main() {
    if std::env::args().any(|a| a == "--accuracy") {
        accuracy();
        return;
    }
    for (name, v) in cases() {
        println!("{name} {}", b(&v));
    }
}
