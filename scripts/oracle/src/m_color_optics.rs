//! `color_optics`: soorat 1.2.0's `optics` half of `color` (M1 part b2): `Color` ⇄ prakash 1.2.0.
//!
//! Every output is class **A**. `from_prakash` clamps each channel in f64 and narrows once (`as f32`); `to_prakash`
//! widens exactly; `from_temperature` and `from_wavelength` are prakash's f64 formulas (glibc `powf`/`ln` inside the
//! temperature fit) followed by `from_prakash`. The Cyrius side calls Cyrius prakash 2.15.2, so these rows also pin
//! that the two prakash builds agree once narrowed to f32. Replayed by tests/parity_optics.tcyr, which needs the
//! `optics` feature (prakash).

use crate::{Module, Rng, Toks};
use prakash::spectral::Rgb;
use soorat::Color;

/// A double in about [-0.5, 1.5) with 52 random fraction bits: most are not f32 values, so the narrowing rounds.
fn chan(r: &mut Rng) -> f64 {
    let m = (r.next() >> 12) as f64 / 4_503_599_627_370_496.0; // [0, 1), 52 bits
    m * 2.0 - 0.5
}

fn c4(c: Color) -> Toks {
    Toks::new().gs(&c.to_array())
}

pub fn module() -> Module {
    let mut m = Module::new("color_optics");
    let mut r = Rng(0x0971_C5C0_2026_1115);

    let mut rgbs: Vec<[f64; 3]> = (0..200)
        .map(|_| [chan(&mut r), chan(&mut r), chan(&mut r)])
        .collect();
    // f32 midpoints (ties to even), ±0, ±∞, NaN, and the clamp's bounds ± a hair.
    let mid = (f64::from(0.5f32) + f64::from(f32::from_bits(0.5f32.to_bits() + 1))) / 2.0;
    let mid2 =
        (f64::from(f32::from_bits(0x3f00_0001)) + f64::from(f32::from_bits(0x3f00_0002))) / 2.0;
    rgbs.extend([
        [mid, mid2, -mid],
        [0.0, -0.0, 1.0],
        [f64::NAN, f64::INFINITY, f64::NEG_INFINITY],
        [1.0 + f64::EPSILON, -f64::MIN_POSITIVE, 0.999_999_999_999],
        [1e-50, 5e-324, 0.25],
    ]);
    for (i, c) in rgbs.iter().enumerate() {
        let alpha = if i % 3 == 0 { 1.0 } else { (i as f32) / 256.0 };
        let col = Color::from_prakash(Rgb::new(c[0], c[1], c[2]), alpha);
        m.row(
            "Color::from_prakash",
            'A',
            &Toks::new().f(c[0]).f(c[1]).f(c[2]).g(alpha),
            &c4(col),
            &format!("case {i}"),
        );
    }

    for i in 0..40 {
        let ch = [r.f(), r.f() / 8.0, r.f() / 8.0, 1.0];
        let p = Color::new(ch[0], ch[1], ch[2], ch[3]).to_prakash();
        m.row(
            "Color::to_prakash",
            'A',
            &Toks::new().gs(&ch),
            &Toks::new().f(p.r).f(p.g).f(p.b),
            &format!("case {i}"),
        );
    }

    let mut ks: Vec<f64> = (0..=400).map(|k| f64::from(k) * 100.0).collect();
    ks.extend([
        6500.5,
        1850.0,
        1900.0 + 1e-9,
        6600.0,
        6599.999_999,
        -1.0,
        1e9,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ]);
    for _ in 0..100 {
        ks.push(1000.0 + chan(&mut r).abs() * 20_000.0);
    }
    for (i, &k) in ks.iter().enumerate() {
        m.row(
            "Color::from_temperature",
            'A',
            &Toks::new().f(k),
            &c4(Color::from_temperature(k)),
            &format!("{k:?} K (case {i})"),
        );
    }

    let mut nms: Vec<f64> = (740..=1580).map(|k| f64::from(k) / 2.0).collect();
    nms.extend([
        380.0 - 1e-9,
        780.0 + 1e-9,
        419.999_999_9,
        700.000_000_1,
        f64::NAN,
        f64::INFINITY,
        -0.0,
    ]);
    for _ in 0..100 {
        nms.push(380.0 + (chan(&mut r) + 0.5) * 200.0);
    }
    for (i, &nm) in nms.iter().enumerate() {
        let out = match Color::from_wavelength(nm) {
            Some(c) => c4(c),
            None => Toks::new().none(),
        };
        m.row(
            "Color::from_wavelength",
            'A',
            &Toks::new().f(nm),
            &out,
            &format!("{nm:?} nm (case {i})"),
        );
    }
    m
}
