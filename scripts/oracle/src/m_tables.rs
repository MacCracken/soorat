//! `tables`: soorat 1.2.0's f16 and sRGB conversions (crate-private in `environment`, reached through the oracle
//! hooks), M1 part d.
//!
//! - `f16_to_f32` and `f32_to_f16` are bit manipulation: class **A**, 64 values a row. The grid: every subnormal
//!   half (the only arithmetic path, `mant × 2⁻²⁴`), every exponent at its mantissa edges, random halves; for the
//!   narrowing, every f32 on and around each rounding midpoint (± 1 ulp) at every exponent, the overflow and
//!   underflow edges, NaN payloads, and random f32s across the half range. `validate.py` re-derives them with
//!   Python's IEEE half (`struct` format `e`).
//! - `srgb_to_linear` and `linear_to_srgb` call f32 `powf`, host glibc's: class **C** with bound 0 against
//!   `tables.cr.tsv` where glibc misrounds (ADR 0002 #16), one value a row; dense grids across [0, 1] and beyond,
//!   each knee ± 2 ulp, ±0, ±∞, NaN.
//! - `srgb8_decode_table`: class **A**. src/tables.cyr's table is generated from this row (port plan §6.2: "tables
//!   generated from the oracle").

use crate::{Module, Rng, Toks};
use soorat::oracle::{f16_to_f32, f32_to_f16, linear_to_srgb, srgb_to_linear, srgb8_decode_table};

fn step(x: f32, k: i32) -> f32 {
    f32::from_bits((x.to_bits() as i64 + k as i64) as u32)
}

pub fn module() -> Module {
    let mut m = Module::new("tables");
    let mut r = Rng(0x7AB1_E516_2026_1115);

    // f16 → f32
    let mut hs: Vec<u16> = Vec::new();
    for sign in [0u16, 0x8000] {
        for mant in 0..=0x3ffu16 {
            hs.push(sign | mant);
        }
        for e in 1..=31u16 {
            for mant in [0u16, 1, 2, 0x155, 0x1ff, 0x200, 0x201, 0x2aa, 0x3fe, 0x3ff] {
                hs.push(sign | (e << 10) | mant);
            }
        }
    }
    for _ in 0..512 {
        hs.push(r.next() as u16);
    }
    for (i, ch) in hs.chunks(64).enumerate() {
        let ins: Vec<i128> = ch.iter().map(|&h| h as i128).collect();
        let outs: Vec<f32> = ch.iter().map(|&h| f16_to_f32(h)).collect();
        m.row(
            "f16_to_f32",
            'A',
            &Toks::new().is(&ins),
            &Toks::new().gs(&outs),
            &format!("group {i}"),
        );
    }

    // f32 → f16
    let mut xs: Vec<f32> = Vec::new();
    for sign in [0u16, 0x8000] {
        for e in 0..=30u16 {
            for mant in [0u16, 1, 2, 3, 0x1ff, 0x200, 0x201, 0x3fd, 0x3fe, 0x3ff] {
                let h = sign | (e << 10) | mant;
                let v = f16_to_f32(h);
                let next = if h & 0x7fff == 0x7bff {
                    65520.0f32.copysign(v)
                } else {
                    f16_to_f32(h + 1)
                };
                let mid = ((f64::from(v) + f64::from(next)) / 2.0) as f32;
                let mid = if h & 0x7fff == 0x7bff {
                    65520.0f32.copysign(v)
                } else {
                    mid
                };
                xs.extend([v, mid, step(mid, -1), step(mid, 1)]);
            }
        }
    }
    xs.extend([
        0.0,
        -0.0,
        1.0e-10,
        -1.0e-10,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        1.0e6,
        -1.0e6,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0xffc0_0000),
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0x7fff_ffff),
        1.0 + 2f32.powi(-11),
        1.0 + 3.0 * 2f32.powi(-11),
        2.0 - 2f32.powi(-13),
    ]);
    for _ in 0..1024 {
        let b = r.next();
        let exp = 100 + (b >> 8) % 46; // 2⁻²⁷ … 2¹⁸: below the subnormals to past the overflow
        let bits = ((b & 1) << 31) as u32 | ((exp as u32) << 23) | ((b >> 16) as u32 & 0x7f_ffff);
        xs.push(f32::from_bits(bits));
    }
    while xs.len() % 64 != 0 {
        xs.push(0.5);
    }
    for (i, ch) in xs.chunks(64).enumerate() {
        let outs: Vec<i128> = ch.iter().map(|&x| f32_to_f16(x) as i128).collect();
        m.row(
            "f32_to_f16",
            'A',
            &Toks::new().gs(ch),
            &Toks::new().is(&outs),
            &format!("group {i}"),
        );
    }

    // sRGB decode
    let mut cs: Vec<f32> = (0..256).map(|i| i as f32 / 255.0).collect();
    cs.extend((-32..=2080).map(|k| k as f32 / 2048.0));
    let knee = 0.040_45f32;
    cs.extend((-2..=2).map(|k| step(knee, k)));
    cs.extend([
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -0.0,
        4.0,
        -4.0,
        f32::MIN_POSITIVE,
    ]);
    for (i, &c) in cs.iter().enumerate() {
        m.row(
            "srgb_to_linear",
            'C',
            &Toks::new().g(c),
            &Toks::new().g(srgb_to_linear(c)),
            &format!("c {c:?} (case {i})"),
        );
    }

    // sRGB encode
    let mut ls: Vec<f32> = (-32..=2080).map(|k| k as f32 / 2048.0).collect();
    ls.extend((3000..=3600).map(|k| k as f32 / 1_048_576.0));
    let lknee = 0.003_130_8f32;
    ls.extend((-2..=2).map(|k| step(lknee, k)));
    ls.extend([
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -0.0,
        4.0,
        -4.0,
        f32::MIN_POSITIVE,
        1.0e-30,
    ]);
    for (i, &l) in ls.iter().enumerate() {
        m.row(
            "linear_to_srgb",
            'C',
            &Toks::new().g(l),
            &Toks::new().g(linear_to_srgb(l)),
            &format!("l {l:?} (case {i})"),
        );
    }

    m.row(
        "srgb8_decode_table",
        'A',
        &Toks::new(),
        &Toks::new().gs(&srgb8_decode_table()),
        "the 256 entries",
    );
    m
}
