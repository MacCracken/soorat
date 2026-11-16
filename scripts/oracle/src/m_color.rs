//! `color`: soorat 1.2.0's `color` module (M1 part b), without the `optics` functions (M1 part b2).
//!
//! Colours are quantized outputs, so the port follows Rust's f32 sequence exactly (D4 rule a): class **A** where a
//! function only converts, clamps, negates or quantizes, class **B** (≤ 2 ulp, expected 0) where it does f32
//! arithmetic. `validate.py` re-derives every row by emulating f32 in Python.
//!
//! The grid (port plan §7, M1: "the full sRGB domain"; colour inputs):
//! - `from_rgba8`: every byte value in each channel;
//! - `to_rgba8`: every k/510 (both sides of every rounding boundary), every (k + ½)/255 rounded to f32 (the halfway
//!   points, where Rust's half-away-from-zero `round` and ties-to-even differ), out-of-range values, ±0, ±∞, NaN;
//! - the colour maps: a dense grid across and beyond their domain, each stop ± 1 ulp, ±0, ±∞, NaN.

use crate::{Module, Rng, Toks};
use soorat::Color;
use soorat::color::{signed_value_color, visualization_heat_map};

fn c4(c: Color) -> Toks {
    Toks::new().gs(&c.to_array())
}

/// A colour component in [0, 1): 23 random bits, exact in f32.
fn unit(r: &mut Rng) -> f32 {
    ((r.next() >> 41) as f32) / 8_388_608.0
}

fn next_up(x: f32) -> f32 {
    f32::from_bits(x.to_bits() + 1)
}

fn next_down(x: f32) -> f32 {
    f32::from_bits(x.to_bits() - 1)
}

/// The non-finite and signed-zero inputs every scalar grid ends with.
fn specials() -> [f32; 6] {
    [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        -f32::MIN_POSITIVE,
    ]
}

pub fn module() -> Module {
    let mut m = Module::new("color");
    let mut r = Rng(0xC010_4B1D_2026_1115);

    for v in 0u32..256 {
        let b = [v, 255 - v, (v * 37) & 255, (v * 101 + 7) & 255];
        let c = Color::from_rgba8(b[0] as u8, b[1] as u8, b[2] as u8, b[3] as u8);
        let ins: Vec<i128> = b.iter().map(|&x| x as i128).collect();
        m.row(
            "Color::from_rgba8",
            'A',
            &Toks::new().is(&ins),
            &c4(c),
            &format!("bytes {b:?}"),
        );
    }

    let mut hexes = vec![
        0u32,
        0xFFFF_FFFF,
        0x3366_99FF,
        0xFF80_40FF,
        0xFF00_00FF,
        0x8080_8080,
    ];
    for _ in 0..64 {
        hexes.push(r.next() as u32);
    }
    for h in hexes {
        m.row(
            "Color::from_hex",
            'A',
            &Toks::new().i(h as i128),
            &c4(Color::from_hex(h)),
            &format!("0x{h:08X}"),
        );
    }

    let mut q: Vec<f32> = (0..=510).map(|k| k as f32 / 510.0).collect();
    q.extend((0..255).map(|k| ((k as f64 + 0.5) / 255.0) as f32));
    q.extend((0..255).map(|k| next_down(((k as f64 + 0.5) / 255.0) as f32)));
    q.extend([
        2.0,
        -1.0,
        1.0 + f32::EPSILON,
        next_up(1.0),
        0.5,
        0.25,
        0.75,
        1e-30,
        -1e-30,
    ]);
    q.extend(specials());
    while q.len() % 4 != 0 {
        q.push(0.5);
    }
    for (i, ch) in q.chunks(4).enumerate() {
        let c = Color::new(ch[0], ch[1], ch[2], ch[3]);
        let b = c.to_rgba8();
        let outs: Vec<i128> = b.iter().map(|&x| x as i128).collect();
        m.row(
            "Color::to_rgba8",
            'A',
            &Toks::new().gs(ch),
            &Toks::new().is(&outs),
            &format!("group {i}"),
        );
    }

    for i in 0..20 {
        let ch = [unit(&mut r), r.f(), unit(&mut r), unit(&mut r)];
        let w = Color::new(ch[0], ch[1], ch[2], ch[3]).to_wgpu();
        m.row(
            "Color::to_wgpu",
            'A',
            &Toks::new().gs(&ch),
            &Toks::new().f(w.r).f(w.g).f(w.b).f(w.a),
            &format!("case {i}"),
        );
    }

    for i in 0..200 {
        let ch = if i < 180 {
            [unit(&mut r), unit(&mut r), unit(&mut r), 1.0]
        } else {
            [r.f(), r.f(), r.f(), 1.0]
        };
        let c = Color::new(ch[0], ch[1], ch[2], ch[3]);
        m.row(
            "Color::luminance",
            'B',
            &Toks::new().gs(&ch),
            &Toks::new().g(c.luminance()),
            &format!("case {i}"),
        );
    }
    for (i, c) in [
        Color::WHITE,
        Color::BLACK,
        Color::RED,
        Color::GREEN,
        Color::BLUE,
    ]
    .iter()
    .enumerate()
    {
        m.row(
            "Color::luminance",
            'B',
            &Toks::new().gs(&c.to_array()),
            &Toks::new().g(c.luminance()),
            &format!("constant {i}"),
        );
    }

    let ts = [
        f32::NAN,
        -0.0,
        0.0,
        1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.5,
        next_down(1.0),
    ];
    for i in 0..200 + ts.len() {
        let a = [unit(&mut r), unit(&mut r), unit(&mut r), unit(&mut r)];
        let b = [unit(&mut r), unit(&mut r), unit(&mut r), unit(&mut r)];
        let t = if i < 100 {
            unit(&mut r)
        } else if i < 200 {
            unit(&mut r) * 2.0 - 0.5
        } else {
            ts[i - 200]
        };
        let c = Color::from(a).lerp(Color::from(b), t);
        m.row(
            "Color::lerp",
            'B',
            &Toks::new().gs(&a).gs(&b).g(t),
            &c4(c),
            &format!("case {i}"),
        );
    }

    let mut hm: Vec<f32> = (-64..=1088).map(|k| k as f32 / 1024.0).collect();
    for s in [0.25f32, 0.5, 0.75, 1.0] {
        hm.push(next_down(s));
        hm.push(next_up(s));
    }
    hm.extend(specials());
    for (i, &t) in hm.iter().enumerate() {
        m.row(
            "visualization_heat_map",
            'B',
            &Toks::new().g(t),
            &c4(visualization_heat_map(t)),
            &format!("t {t:?} (case {i})"),
        );
    }

    let mut sv: Vec<f32> = (-600..=600).map(|k| k as f32 / 512.0).collect();
    sv.extend([next_down(-1.0), next_up(-1.0), next_down(1.0), next_up(1.0)]);
    sv.extend(specials());
    for (i, &v) in sv.iter().enumerate() {
        m.row(
            "signed_value_color",
            'A',
            &Toks::new().g(v),
            &c4(signed_value_color(v)),
            &format!("v {v:?} (case {i})"),
        );
    }

    m.row(
        "Color::default",
        'A',
        &Toks::new(),
        &c4(Color::default()),
        "Color::default()",
    );
    m
}
