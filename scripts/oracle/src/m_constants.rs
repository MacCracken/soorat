//! `constants`: soorat 1.2.0's public constants and the crate-private ones the oracle exposes (class A).
//! M1 generates `src/constants.cyr` from the same values; nothing the port reads is typed by hand.

use crate::{Module, Toks};
use soorat::animation::MAX_JOINTS;
use soorat::batch::MAX_SPRITES_PER_BATCH;
use soorat::color::Color;
use soorat::gpu_particles::MAX_PARTICLES;
use soorat::lights::MAX_LIGHTS;
use soorat::oracle::{IDENTITY_MAT4, MAX_QUADS, MAX_SEGMENTS, XZ_QUAD_INDICES};
use soorat::shadow::{DEFAULT_SHADOW_MAP_SIZE, MAX_CASCADES};
use soorat::sprite::UvRect;

pub fn module() -> Module {
    let mut m = Module::new("constants");
    let none = Toks::new();
    let int = |m: &mut Module, name: &str, v: i128| {
        m.row(&format!("const::{name}"), 'A', &none, &Toks::new().i(v), &format!("{name} = {v}"));
    };
    int(&mut m, "MAX_JOINTS", MAX_JOINTS as i128);
    int(&mut m, "MAX_LIGHTS", MAX_LIGHTS as i128);
    int(&mut m, "MAX_CASCADES", MAX_CASCADES as i128);
    int(&mut m, "MAX_PARTICLES", i128::from(MAX_PARTICLES));
    int(&mut m, "MAX_SPRITES_PER_BATCH", MAX_SPRITES_PER_BATCH as i128);
    int(&mut m, "DEFAULT_SHADOW_MAP_SIZE", i128::from(DEFAULT_SHADOW_MAP_SIZE));
    int(&mut m, "MAX_SEGMENTS", i128::from(MAX_SEGMENTS));
    int(&mut m, "MAX_QUADS", MAX_QUADS as i128);
    let idx: Vec<i128> = XZ_QUAD_INDICES.iter().map(|&x| i128::from(x)).collect();
    m.row("const::XZ_QUAD_INDICES", 'A', &none, &Toks::new().is(&idx), "XZ_QUAD_INDICES");
    m.row("const::IDENTITY_MAT4", 'A', &none, &Toks::new().gs(&IDENTITY_MAT4), "IDENTITY_MAT4 (column-major)");
    for (name, c) in [
        ("WHITE", Color::WHITE),
        ("BLACK", Color::BLACK),
        ("RED", Color::RED),
        ("GREEN", Color::GREEN),
        ("BLUE", Color::BLUE),
        ("TRANSPARENT", Color::TRANSPARENT),
        ("CORNFLOWER_BLUE", Color::CORNFLOWER_BLUE),
    ] {
        m.row(&format!("const::Color::{name}"), 'A', &none, &Toks::new().gs(&c.to_array()), &format!("Color::{name} rgba"));
    }
    let f = UvRect::FULL;
    m.row("const::UvRect::FULL", 'A', &none, &Toks::new().gs(&[f.u_min, f.v_min, f.u_max, f.v_max]), "UvRect::FULL");
    m
}
