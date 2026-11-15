//! P3 — f32 transcendentals on soorat's own inputs (soorat-port-plan §4 step 1).
//!
//! Generates the arguments soorat 1.2.0 actually passes to f32 `sin`, `cos`,
//! `acos`, `powf`, `exp`, `ln` and `log10`, computed the way soorat computes
//! them (in f32). Writes:
//! - `../math/out/p3_inputs.txt`: `<set> <fn> <a bits> <b bits>` per case, read by
//!   `scripts/probes/math/tests/p3_trans.tcyr` (ganita's f32 tier);
//! - `../math/out/p3_rust.txt`: `<rust bits> <reference bits>` per case, where
//!   Rust is std's f32 function (host glibc) and the reference is the same
//!   function in f64 rounded once to f32 (correctly rounded but for rare double
//!   roundings, which the comparison reports).
//!
//! `p3_compare.py` joins the three columns.

use std::f32::consts::{PI, TAU};
use std::fmt::Write as _;

// Function ids (shared with the Cyrius side).
const SIN: u32 = 0;
const COS: u32 = 1;
const ACOS: u32 = 2;
const POW: u32 = 3;
const EXP: u32 = 4;
const LN: u32 = 5;
const LOG10: u32 = 6;

// Set ids.
const SET_SPHERE: u32 = 0; // primitives::sphere, acoustic/em balloons: (i/n)·π, (j/m)·τ
const SET_STEP: u32 = 1; // cylinder, wire_circle, thermo: (τ/n)·i
const SET_ROT: u32 = 2; // sprite rotations: arbitrary angles
const SET_SLERP: u32 = 3; // slerp: acos(cos), sin(θ), sin(tθ)
const SET_SRGB_DEC: u32 = 4; // ((c + 0.055)/1.055)^2.4
const SET_SRGB_ENC: u32 = 5; // l^(1/2.4)
const SET_SPLITS: u32 = 6; // (far/near)^ratio
const SET_DECAY: u32 = 7; // exp(-t/τ), 10·log10(e), ln 10

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
    /// Uniform f32 in [0, 1) with 24 bits.
    fn unit(&mut self) -> f32 {
        ((self.next() >> 40) as f32) / 16_777_216.0
    }
}

fn f(func: u32, a: f32, b: f32) -> f32 {
    match func {
        SIN => a.sin(),
        COS => a.cos(),
        ACOS => a.acos(),
        POW => a.powf(b),
        EXP => a.exp(),
        LN => a.ln(),
        _ => a.log10(),
    }
}

fn reference(func: u32, a: f32, b: f32) -> f32 {
    let (a, b) = (f64::from(a), f64::from(b));
    (match func {
        SIN => a.sin(),
        COS => a.cos(),
        ACOS => a.acos(),
        POW => a.powf(b),
        EXP => a.exp(),
        LN => a.ln(),
        _ => a.log10(),
    }) as f32
}

fn main() {
    let mut cases: Vec<(u32, u32, f32, f32)> = Vec::new();
    let segs: Vec<u32> = (3..=64).chain([96, 128, 256, 512, 1024]).collect();

    // Sphere/balloon lattice: phi = (y/rings)·π, theta = (x/segs)·τ, as primitives::sphere.
    for &n in &segs {
        for i in 0..=n {
            let phi = (i as f32 / n as f32) * PI;
            let theta = (i as f32 / n as f32) * TAU;
            for a in [phi, theta] {
                cases.push((SET_SPHERE, SIN, a, 0.0));
                cases.push((SET_SPHERE, COS, a, 0.0));
            }
        }
    }
    // Fixed-step rings: angle = (τ/n)·i, as cylinder / wire_circle / thermo.
    for &n in &segs {
        let step = TAU / n as f32;
        for i in 0..=n {
            let a = step * i as f32;
            cases.push((SET_STEP, SIN, a, 0.0));
            cases.push((SET_STEP, COS, a, 0.0));
        }
    }
    let mut r = Rng(0xDEAD_BEEF_CAFE_F00D);
    // Sprite rotations: arbitrary angles in about ±4π.
    for _ in 0..20_000 {
        let a = (r.unit() - 0.5) * 8.0 * PI;
        cases.push((SET_ROT, SIN, a, 0.0));
        cases.push((SET_ROT, COS, a, 0.0));
    }
    // slerp: cos ∈ [-1, 0.9995] (the nlerp switch), then sin θ and sin(tθ).
    for k in 0..=20_000u32 {
        let c = -1.0 + (k as f32 / 20_000.0) * 1.9995;
        let theta = c.clamp(-1.0, 1.0).acos();
        cases.push((SET_SLERP, ACOS, c, 0.0));
        cases.push((SET_SLERP, SIN, theta, 0.0));
        let t = r.unit();
        cases.push((SET_SLERP, SIN, (1.0 - t) * theta, 0.0));
        cases.push((SET_SLERP, SIN, t * theta, 0.0));
    }
    // sRGB decode: every byte above the knee, then a dense grid.
    for i in 0..=255u32 {
        let c = i as f32 / 255.0;
        if c > 0.040_45 {
            cases.push((SET_SRGB_DEC, POW, (c + 0.055) / 1.055, 2.4));
        }
    }
    for k in 0..20_000u32 {
        let c = 0.040_45 + (k as f32 / 20_000.0) * (1.0 - 0.040_45);
        cases.push((SET_SRGB_DEC, POW, (c + 0.055) / 1.055, 2.4));
    }
    // sRGB encode: l^(1/2.4) above the knee (box-filtered mip values are arbitrary).
    for _ in 0..20_000u32 {
        let l = 0.003_130_8 + r.unit() * (1.0 - 0.003_130_8);
        cases.push((SET_SRGB_ENC, POW, l, 1.0 / 2.4));
    }
    // Cascade splits.
    for near in [0.01f32, 0.05, 0.1, 0.5, 1.0] {
        for far in [10.0f32, 50.0, 100.0, 250.0, 1000.0, 5000.0] {
            for count in 1..=8u32 {
                for i in 1..count {
                    cases.push((SET_SPLITS, POW, far / near, i as f32 / count as f32));
                }
            }
        }
    }
    // Coupled decay: exp(-t/τ) and 10·log10(e), plus ln 10.
    for _ in 0..20_000 {
        let t = r.unit() * 5.0;
        let tau = 0.05 + r.unit() * 2.0;
        cases.push((SET_DECAY, EXP, -t / tau, 0.0));
        let e = (r.unit() * 30.0 - 27.0).exp2().max(1e-9); // 2^-27 .. 2^3
        cases.push((SET_DECAY, LOG10, e, 0.0));
    }
    cases.push((SET_DECAY, LN, 10.0, 0.0));

    let dir = format!("{}/../math/out", env!("CARGO_MANIFEST_DIR"));
    std::fs::create_dir_all(&dir).expect("out dir");
    let mut inputs = String::new();
    let mut rust = String::new();
    for &(set, func, a, b) in &cases {
        let _ = writeln!(inputs, "{set} {func} {} {}", a.to_bits(), b.to_bits());
        let _ = writeln!(
            rust,
            "{} {}",
            f(func, a, b).to_bits(),
            reference(func, a, b).to_bits()
        );
    }
    std::fs::write(format!("{dir}/p3_inputs.txt"), inputs).expect("inputs");
    std::fs::write(format!("{dir}/p3_rust.txt"), rust).expect("rust");
    println!("{} cases", cases.len());
}
