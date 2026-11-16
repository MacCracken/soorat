//! `json`: soorat 1.2.0's serde documents, as serde_json 1.0.154 reads them (M1 part b2: `Color`).
//!
//! Each row is a document and what `serde_json::from_str` gives: the value (class **A**, bit for bit) or
//! `E:Deserialize` (serde_json's error is no `RenderError`; the Cyrius reader's is the Cyrius-only
//! `SR_ERR_DESERIALIZE`, ADR 0002 #19). The documents are serde_json's own output for a grid of colours (so every
//! Rust document reads back), then hand-written ones for serde's derive semantics: field order, the array form, extra
//! and missing fields, duplicates, null, strings, integers, signed zeros, exponents, f32 overflow, f64 overflow and
//! trailing text. Three documents serde_json reads and bayan's tree can't (the integer `-0`, an integer above
//! i64::MAX, a duplicated unknown field) are rows of their own, `Color::from_json_bayan_*`, which the harness leaves
//! pending until bayan changes (pravash's practice; ADR 0002 #19, bayan's open issues).

use crate::{Module, Rng, Toks};
use soorat::Color;

fn decode(doc: &str) -> Toks {
    match serde_json::from_str::<Color>(doc) {
        Ok(c) => Toks::new().gs(&c.to_array()),
        Err(_) => Toks::new().err("Deserialize"),
    }
}

fn unit(r: &mut Rng) -> f32 {
    ((r.next() >> 41) as f32) / 8_388_608.0
}

pub fn module() -> Module {
    let mut m = Module::new("json");
    let mut r = Rng(0x15_0B_2026_1115);

    let mut colors: Vec<[f32; 4]> = (0..120)
        .map(|_| [unit(&mut r), unit(&mut r), unit(&mut r), unit(&mut r)])
        .collect();
    for _ in 0..30 {
        colors.push([
            r.f(),
            r.f() * 1e6,
            r.f() / 1e6,
            f32::from_bits((r.next() as u32) & 0x7f7f_ffff),
        ]);
    }
    colors.extend([
        [0.1, 0.2, 0.3, 0.9],
        [0.0, -0.0, 1.0, f32::MIN_POSITIVE],
        [f32::from_bits(1), f32::MAX, -f32::MAX, 1e-38],
        [0.392, 0.584, 0.929, 1.0],
    ]);
    for (i, c) in colors.iter().enumerate() {
        let doc = serde_json::to_string(&Color::new(c[0], c[1], c[2], c[3])).unwrap_or_default();
        m.row(
            "Color::from_json",
            'A',
            &Toks::new().s(&doc),
            &decode(&doc),
            &format!("serde_json's document {i}"),
        );
    }

    let docs = [
        r#"{"a":1,"b":0.5,"g":0.25,"r":0.125}"#,
        r#"[0.1,0.2,0.3,0.4]"#,
        r#"[0.1,0.2,0.3]"#,
        r#"[0.1,0.2,0.3,0.4,0.5]"#,
        r#"{"r":0.1,"g":0.2,"b":0.3}"#,
        r#"{"r":0.1,"g":0.2,"b":0.3,"a":0.4,"extra":[1,{"x":null}]}"#,
        r#"{"r":0.1,"r":0.2,"g":0,"b":0,"a":0}"#,
        r#"{"r":null,"g":0,"b":0,"a":0}"#,
        r#"{"r":"0.1","g":0,"b":0,"a":0}"#,
        r#"{"r":true,"g":0,"b":0,"a":0}"#,
        " { \"r\" : 1e-3 , \"g\":-0.0,\"b\":0,\"a\":1E2 } \n",
        r#"{"r":1e39,"g":-1e39,"b":3.4028235e38,"a":3.4028236e38}"#,
        r#"{"r":1e309,"g":0,"b":0,"a":0}"#,
        r#"{"r":16777217,"g":-16777217,"b":16777219,"a":9007199254740993}"#,
        r#"{"r":0.1,"g":0.2,"b":0.3,"a":0.4} x"#,
        "",
        "null",
        r#"{"r":1.401298464324817e-45,"g":7e-46,"b":7.1e-46,"a":-2.1e-45}"#,
        r#"{"r":0.30000001192092896,"g":0.1000000015,"b":1.00000000000000000000000000001,"a":0}"#,
        r#"{"r":-9223372036854775808,"g":9223372036854775807,"b":0,"a":0}"#,
        r#"{"r":1,"g":2,"b":3,"a":4,"R":5}"#,
        r#"[]"#,
        r#"{}"#,
    ];
    for (i, doc) in docs.iter().enumerate() {
        m.row(
            "Color::from_json",
            'A',
            &Toks::new().s(doc),
            &decode(doc),
            &format!("hand-written document {i}"),
        );
    }

    // Documents serde_json reads and bayan's tree can't: rows of their own, pending until bayan changes (pravash's
    // `*_bayan_*` rows; ADR 0002 #19).
    let bayan = [
        (
            "Color::from_json_bayan_neg_zero",
            r#"{"r":0,"g":0,"b":-0,"a":0}"#,
            "the integer -0 (serde_json: -0.0)",
        ),
        (
            "Color::from_json_bayan_above_i64",
            r#"{"r":18446744073709551615,"g":0,"b":0,"a":0}"#,
            "an integer above i64::MAX",
        ),
        (
            "Color::from_json_bayan_duplicate_unknown",
            r#"{"r":0.1,"g":0.2,"b":0.3,"a":0.4,"x":1,"x":2}"#,
            "a duplicated unknown field",
        ),
    ];
    for (name, doc, what) in bayan {
        m.row(name, 'A', &Toks::new().s(doc), &decode(doc), what);
    }
    m
}
