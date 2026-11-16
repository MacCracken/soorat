//! `env` (M4 part b): environment's CPU half, the crate-private face codecs, box downsample and mip chain (through the
//! oracle hooks), with mabda's `mip_level_count`; `pbr_material`: `MaterialUniforms`' constructors; `pbr_optics`: its
//! `optics` half, `from_ior` and the BRDF LUT (prakash 1.2.0), replayed by tests/parity_features.tcyr.
//!
//! Classes: decoding and `mip_level_count` are exact (A); the downsample is f32 arithmetic kept bit-faithful (B, plan
//! §5); sRGB encoding (and so an sRGB mip chain) goes through `linear_to_srgb`'s `powf`, class C at bound 0 against
//! `env.cr.tsv` where glibc misrounds (ADR 0002 #16). The f16 face and the material rows are A.

use crate::{Module, Rng, Toks};
use soorat::color::Color;
use soorat::oracle::{FaceFormat, decode_face, downsample, encode_face, face_mip_chain};
use soorat::pbr_material::{MaterialUniforms, generate_brdf_lut};
use soorat::texture::mip_level_count;

fn bytes(b: &[u8]) -> Toks {
    Toks::new().is(&b.iter().map(|&x| x as i128).collect::<Vec<i128>>())
}

fn texels(t: &[[f32; 4]]) -> Vec<f32> {
    t.iter().flatten().copied().collect()
}

fn fmt_code(f: FaceFormat) -> i128 {
    match f {
        FaceFormat::Srgb8 => 0,
        _ => 1,
    }
}

fn random_texels(r: &mut Rng, n: usize, hdr: bool) -> Vec<[f32; 4]> {
    (0..n)
        .map(|_| {
            if hdr {
                [r.f() * 4.0, r.f(), r.pos() * 64.0, r.f() / 8.0]
            } else {
                [
                    r.pos() / 16.0,
                    r.pos() / 16.0,
                    r.pos() / 16.0,
                    r.pos() / 16.0,
                ]
            }
        })
        .collect()
}

pub fn module() -> Module {
    let mut m = Module::new("env");
    let mut r = Rng(0xE4F0_0004_B000_0001);

    // mip_level_count (mabda's): max(w, h).ilog2() + 1, 1 for a zero edge.
    let dims = [0u32, 1, 2, 3, 4, 255, 256, 1023, 1024, 4096, u32::MAX];
    for &w in &dims {
        for &h in &dims {
            m.row(
                "mip_level_count",
                'A',
                &Toks::new().i(w as i128).i(h as i128),
                &Toks::new().i(mip_level_count(w, h) as i128),
                "",
            );
        }
    }

    // decode: every byte in every channel, a trailing partial texel, f16 specials.
    let mut all = Vec::new();
    for b in 0..=255u8 {
        all.extend([b, 255 - b, b ^ 0x55, b]);
    }
    m.row(
        "decode_face",
        'A',
        &Toks::new()
            .i(0)
            .is(&all.iter().map(|&x| x as i128).collect::<Vec<_>>()),
        &Toks::new().gs(&texels(&decode_face(FaceFormat::Srgb8, &all))),
        "every byte, sRGB",
    );
    let mut partial = all[..8].to_vec();
    partial.extend([1, 2, 3]);
    for (f, b, what) in [
        (
            FaceFormat::Srgb8,
            partial.clone(),
            "a trailing partial texel",
        ),
        (FaceFormat::Srgb8, Vec::new(), "empty"),
        (FaceFormat::F16, partial, "f16, a trailing partial texel"),
    ] {
        m.row(
            "decode_face",
            'A',
            &Toks::new()
                .i(fmt_code(f))
                .is(&b.iter().map(|&x| x as i128).collect::<Vec<_>>()),
            &Toks::new().gs(&texels(&decode_face(f, &b))),
            what,
        );
    }
    let mut hb = Vec::new();
    for h in [
        0u16, 0x8000, 0x0001, 0x03FF, 0x0400, 0x3C00, 0x7BFF, 0x7C00, 0xFC00, 0x7E00, 0x7C01,
        0xFFFF,
    ] {
        hb.extend(h.to_le_bytes());
    }
    for _ in 0..64 {
        hb.extend(((r.next() >> 48) as u16).to_le_bytes());
    }
    m.row(
        "decode_face",
        'A',
        &Toks::new()
            .i(1)
            .is(&hb.iter().map(|&x| x as i128).collect::<Vec<_>>()),
        &Toks::new().gs(&texels(&decode_face(FaceFormat::F16, &hb))),
        "f16 specials and random halves",
    );

    // encode: linear values at each sRGB byte's rounding boundary ± a few ulps (where a misrounded powf flips a byte),
    // specials, and random texels.
    let mut edge: Vec<[f32; 4]> = Vec::new();
    for k in 0..255u32 {
        // the linear value whose sRGB · 255 is k + 0.5, found in f64 and narrowed
        let s = (f64::from(k) + 0.5) / 255.0;
        let lin = if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        };
        let c = lin as f32;
        for d in -3i32..=3 {
            let v = f32::from_bits((c.to_bits() as i32 + d) as u32);
            edge.push([v, v, v, (k as f32 + 0.5) / 255.0]);
        }
    }
    for chunk in edge.chunks(64) {
        m.row(
            "encode_face",
            'C',
            &Toks::new().i(0).gs(&texels(chunk)),
            &bytes(&encode_face(FaceFormat::Srgb8, chunk)),
            "sRGB byte boundaries",
        );
    }
    let specials: Vec<[f32; 4]> = vec![
        [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.0],
        [-1.0, 2.0, 0.0031308, 0.0031309],
        [1e-30, 65504.0, 65520.0, 1e10],
        [6.1e-5, 5.96e-8, 2.98e-8, -2.98e-8],
    ];
    for f in [FaceFormat::Srgb8, FaceFormat::F16] {
        m.row(
            "encode_face",
            'C',
            &Toks::new().i(fmt_code(f)).gs(&texels(&specials)),
            &bytes(&encode_face(f, &specials)),
            "specials",
        );
        let t = random_texels(&mut r, 256, f == FaceFormat::F16);
        m.row(
            "encode_face",
            'C',
            &Toks::new().i(fmt_code(f)).gs(&texels(&t)),
            &bytes(&encode_face(f, &t)),
            "random texels",
        );
        m.row(
            "encode_face",
            'C',
            &Toks::new().i(fmt_code(f)).gs(&[]),
            &bytes(&encode_face(f, &[])),
            "empty",
        );
    }

    // downsample: the Rust test's cases, every size to 9, then 16, 33, 64; a short input.
    let two = vec![
        [0.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
    ];
    let three: Vec<[f32; 4]> = (0..9).map(|i| [i as f32, 0.0, 0.0, 1.0]).collect();
    let mut cases: Vec<(usize, Vec<[f32; 4]>, String)> = vec![
        (2, two, "the Rust test's 2×2".into()),
        (3, three, "the Rust test's 3×3".into()),
        (1, vec![[7.0; 4]], "1×1".into()),
        (0, vec![], "0×0".into()),
        (4, vec![[1.0; 4]; 15], "4×4 from 15 texels: short".into()),
    ];
    for s in [2usize, 3, 4, 5, 6, 7, 8, 9, 16, 33, 64] {
        cases.push((
            s,
            random_texels(&mut r, s * s, true),
            format!("{s}×{s}, random"),
        ));
    }
    cases.push((
        8,
        vec![[f32::MAX, -f32::MAX, f32::NAN, 1e-45]; 64],
        "8×8: overflow, NaN, subnormals".into(),
    ));
    for (s, t, what) in cases {
        m.row(
            "downsample",
            'B',
            &Toks::new().i(s as i128).gs(&texels(&t)),
            &Toks::new().gs(&texels(&downsample(&t, s))),
            &what,
        );
    }

    // face_mip_chain: both encodings, sizes 1 … 64, a short face.
    for f in [FaceFormat::Srgb8, FaceFormat::F16] {
        let cls = if f == FaceFormat::Srgb8 { 'C' } else { 'B' };
        for s in [1u32, 2, 3, 4, 5, 8, 16, 31, 64] {
            let n = (s * s) as usize;
            let face = encode_face(f, &random_texels(&mut r, n, f == FaceFormat::F16));
            let mut out = Toks::new();
            let chain = face_mip_chain(f, &face, s);
            out = out.i(chain.len() as i128);
            for (sz, b) in &chain {
                out = out
                    .i(*sz as i128)
                    .is(&b.iter().map(|&x| x as i128).collect::<Vec<_>>());
            }
            m.row(
                "face_mip_chain",
                cls,
                &Toks::new()
                    .i(fmt_code(f))
                    .i(s as i128)
                    .is(&face.iter().map(|&x| x as i128).collect::<Vec<_>>()),
                &out,
                &format!("{s}×{s}"),
            );
        }
        let face = encode_face(f, &random_texels(&mut r, 10, false));
        let chain = face_mip_chain(f, &face, 4);
        let mut out = Toks::new().i(chain.len() as i128);
        for (sz, b) in &chain {
            out = out
                .i(*sz as i128)
                .is(&b.iter().map(|&x| x as i128).collect::<Vec<_>>());
        }
        m.row(
            "face_mip_chain",
            cls,
            &Toks::new()
                .i(fmt_code(f))
                .i(4)
                .is(&face.iter().map(|&x| x as i128).collect::<Vec<_>>()),
            &out,
            "4×4 from 10 texels: short",
        );
    }
    m
}

fn material(t: Toks, u: &MaterialUniforms) -> Toks {
    t.gs(&u.base_color_factor)
        .g(u.metallic)
        .g(u.roughness)
        .g(u._pad0)
        .g(u._pad1)
}

pub fn material_module() -> Module {
    let mut m = Module::new("pbr_material");
    let mut r = Rng(0x9B12_0004_B000_0002);
    m.row(
        "MaterialUniforms::default",
        'A',
        &Toks::new(),
        &material(Toks::new(), &MaterialUniforms::default()),
        "",
    );
    let mut cs: Vec<(Color, f32)> = vec![
        (Color::RED, 0.8),
        (Color::new(0.9, 0.8, 0.2, 1.0), 0.3),
        (Color::WHITE, f32::NAN),
        (Color::new(-1.0, 2.0, f32::NAN, 0.5), -0.0),
    ];
    for _ in 0..30 {
        cs.push((Color::new(r.f(), r.f(), r.f(), r.f()), r.f()));
    }
    for (c, rough) in &cs {
        let ins = Toks::new().gs(&c.to_array()).g(*rough);
        m.row(
            "MaterialUniforms::dielectric",
            'A',
            &ins,
            &material(Toks::new(), &MaterialUniforms::dielectric(*c, *rough)),
            "",
        );
        m.row(
            "MaterialUniforms::metal",
            'A',
            &ins,
            &material(Toks::new(), &MaterialUniforms::metal(*c, *rough)),
            "",
        );
    }
    m
}

pub fn optics_module() -> Module {
    let mut m = Module::new("pbr_optics");
    let mut r = Rng(0x0F71_0004_B000_0003);
    let mut iors: Vec<f64> = vec![
        1.0,
        1.5,
        1.33,
        2.42,
        0.0,
        -1.0,
        1e10,
        f64::NAN,
        f64::INFINITY,
        -0.0,
    ];
    for _ in 0..40 {
        iors.push(1.0 + f64::from(r.pos()) / 4.0);
    }
    for ior in iors {
        let c = Color::new(r.pos() / 16.0, r.pos() / 16.0, r.pos() / 16.0, 1.0);
        let rough = r.pos() / 16.0;
        let u = MaterialUniforms::from_ior(c, ior, rough);
        m.row(
            "MaterialUniforms::from_ior",
            'A',
            &Toks::new().gs(&c.to_array()).f(ior).g(rough),
            &material(Toks::new(), &u),
            &format!("ior {ior}"),
        );
    }
    for size in [0u32, 1, 2, 3, 4, 8, 16, 32] {
        for samples in [0u32, 1, 2, 3, 16, 32, 64, 128] {
            m.row(
                "generate_brdf_lut",
                'A',
                &Toks::new().i(size as i128).i(samples as i128),
                &bytes(&generate_brdf_lut(size, samples)),
                &format!("{size}×{size}, {samples} samples"),
            );
        }
    }
    m
}
