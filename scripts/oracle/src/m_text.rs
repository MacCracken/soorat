//! `text`: soorat 1.2.0's `text` and `ui` modules (M2 part b), class **A**: glyph rectangles, and the sprites a text
//! batch and a UI batch hold. A font is its 8 fields (`v:8`: texture_id, glyph_width, glyph_height, columns,
//! atlas_width, atlas_height, first_char, glyph_count); a sprite is 13 f32 (x, y, width, height, rotation, color, uv)
//! then its texture_id and z_order.

use crate::{Module, Rng, Toks};
use soorat::color::Color;
use soorat::sprite::Sprite;
use soorat::text::{BitmapFont, TextBatch};
use soorat::ui::{UiBatch, UiPanel};

fn font8(f: &BitmapFont) -> Toks {
    Toks::new().is(&[
        f.texture_id as i128,
        f.glyph_width as i128,
        f.glyph_height as i128,
        f.columns as i128,
        f.atlas_width as i128,
        f.atlas_height as i128,
        f.first_char as i128,
        f.glyph_count as i128,
    ])
}

fn sprites(list: &[Sprite]) -> Toks {
    let mut t = Toks::new().i(list.len() as i128);
    for s in list {
        let mut v = vec![s.x, s.y, s.width, s.height, s.rotation];
        v.extend_from_slice(&s.color.to_array());
        v.extend_from_slice(&[s.uv.u_min, s.uv.v_min, s.uv.u_max, s.uv.v_max]);
        t = t.gs(&v).i(s.texture_id as i128).i(s.z_order as i128);
    }
    t
}

fn font(
    tex: u64,
    gw: u32,
    gh: u32,
    cols: u32,
    aw: u32,
    ah: u32,
    first: u8,
    count: u32,
) -> BitmapFont {
    BitmapFont {
        texture_id: tex,
        glyph_width: gw,
        glyph_height: gh,
        columns: cols,
        atlas_width: aw,
        atlas_height: ah,
        first_char: first,
        glyph_count: count,
    }
}

pub fn module() -> Module {
    let mut m = Module::new("text");
    let mut r = Rng(0x7E47_2026_1115);
    let test = font(1, 8, 16, 16, 128, 128, 32, 96);
    let fonts = [
        test.clone(),
        font(2, 8, 16, 16, 0, 0, 32, 96),
        font(3, 8, 16, 0, 128, 128, 32, 96),
        font(4, 6, 9, 3, 100, 77, 0, 7),
        font(5, 10, 10, 1, 1000, 1000, 255, 1),
        font(6, 0x1_0000, 0x1_0000, 1, 4096, 4096, 65, 1_000_000),
        font(7, 7, 7, 13, 91, 91, 33, 0),
    ];
    for (fi, f) in fonts.iter().enumerate() {
        let mut chars: Vec<u32> = if fi == 0 {
            (0..300).collect()
        } else {
            vec![0, 31, 32, 33, 64, 65, 66, 126, 127, 255, 256]
        };
        chars.extend([0x200, 0xE9, 0x20AC, 0x1F600, 0x10FFFF]);
        for &c in &chars {
            let ch = char::from_u32(c).unwrap_or('\u{FFFD}');
            let uv = f.glyph_uv(ch);
            m.row(
                "BitmapFont::glyph_uv",
                'A',
                &font8(f).i(ch as u32 as i128),
                &Toks::new().gs(&[uv.u_min, uv.v_min, uv.u_max, uv.v_max]),
                &format!("font {fi}, U+{:04X}", ch as u32),
            );
        }
    }

    let long: String = (0..200)
        .map(|_| char::from(32 + (r.next() % 95) as u8))
        .collect();
    let texts = [
        "Hello",
        "A B",
        "",
        "   ",
        "é A€😀",
        "Score: 42",
        "\t\n\r\u{7f}\u{80}\u{ff}",
        "日本語 テキスト",
        long.as_str(),
    ];
    let scales = [1.0f32, 2.0, 0.5, -1.0, 0.0, f32::NAN, 1e30, 1.0 / 3.0];
    for (ti, text) in texts.iter().enumerate() {
        for (k, &scale) in scales.iter().enumerate() {
            if ti > 2 && k > 2 && k != 5 {
                continue;
            }
            let f = &fonts[(ti + k) % 5];
            let (x, y) = (r.f() * 100.0, r.f() * 100.0);
            let color = Color::new(((r.next() >> 41) as f32) / 8_388_608.0, 0.5, 0.25, 1.0);
            let z = (r.next() as i32) >> (k as u32 * 3);
            let mut tb = TextBatch::new();
            tb.draw_text_z(f, text, x, y, scale, color, z);
            m.row(
                "TextBatch::draw_text_z",
                'A',
                &font8(f)
                    .s(text)
                    .g(x)
                    .g(y)
                    .g(scale)
                    .gs(&color.to_array())
                    .i(z as i128),
                &sprites(&tb.batch.sprites),
                &format!("text {ti}, scale {scale:?}, font {}", (ti + k) % 5),
            );
        }
    }

    for i in 0..30 {
        let p = UiPanel::new(r.f() * 500.0, r.f() * 500.0, r.pos() * 10.0, r.f() * 10.0)
            .with_color(Color::new(0.1, 0.2, 0.3, 0.4))
            .with_texture(r.next() >> 3)
            .with_z_order(r.next() as i32);
        let mut u = UiBatch::new();
        u.add_panel(&p);
        m.row(
            "UiBatch::add_panel",
            'A',
            &Toks::new()
                .gs(&[p.x, p.y, p.width, p.height])
                .gs(&p.color.to_array())
                .i(p.texture_id as i128)
                .i(p.z_order as i128),
            &sprites(&u.panels.sprites),
            &format!("case {i}"),
        );
    }
    m
}
