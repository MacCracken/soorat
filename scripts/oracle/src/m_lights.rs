//! `lights`: soorat 1.2.0's `lights` module (M1 part c).
//!
//! Every output is a copy except a spot light's cone cosines, Rust's f32 `cos` from host glibc: class **C** with
//! bound 0 against the correctly rounded value, which `validate.py --write` puts in `lights.cr.tsv` where glibc
//! misrounds (ADR 0002 #16). The rest are class **A**. A light is written as its 16 f32 (`v:16`), a light array as
//! its 136 f32 (`v:136`: ambient, light_count, then 8 lights).

use crate::{Module, Rng, Toks};
use soorat::lights::{GpuLight, LightArrayUniforms, MAX_LIGHTS};

fn light16(l: &GpuLight) -> Vec<f32> {
    let mut v = Vec::with_capacity(16);
    v.extend_from_slice(&l.position_type);
    v.extend_from_slice(&l.direction_range);
    v.extend_from_slice(&l.color_intensity);
    v.extend_from_slice(&l.spot_params);
    v
}

fn array136(u: &LightArrayUniforms) -> Vec<f32> {
    let mut v = Vec::with_capacity(136);
    v.extend_from_slice(&u.ambient);
    v.extend_from_slice(&u.light_count);
    for l in &u.lights {
        v.extend(light16(l));
    }
    v
}

fn v3(r: &mut Rng) -> [f32; 3] {
    [r.f(), r.f(), r.f()]
}

pub fn module() -> Module {
    let mut m = Module::new("lights");
    let mut r = Rng(0x0119_4715_2026_1115);

    for i in 0..20 {
        let (d, c, k) = (v3(&mut r), v3(&mut r), r.f());
        let l = GpuLight::directional(d, c, k);
        m.row(
            "GpuLight::directional",
            'A',
            &Toks::new().gs(&d).gs(&c).g(k),
            &Toks::new().gs(&light16(&l)),
            &format!("case {i}"),
        );
    }
    for i in 0..20 {
        let (p, c, k, rg) = (v3(&mut r), v3(&mut r), r.f(), r.pos());
        let l = GpuLight::point(p, c, k, rg);
        m.row(
            "GpuLight::point",
            'A',
            &Toks::new().gs(&p).gs(&c).g(k).g(rg),
            &Toks::new().gs(&light16(&l)),
            &format!("case {i}"),
        );
    }
    // Cone half-angles across 0..1.5 rad in steps of 0.0015, then the edges.
    let mut cones: Vec<f32> = (0..1000).map(|k| k as f32 * 0.0015).collect();
    cones.extend([
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -0.0,
        -0.3,
        100.0,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
    ]);
    for (i, &inner) in cones.iter().enumerate() {
        let outer = if inner.is_finite() { inner + 0.1 } else { 0.5 };
        let (p, d, c, k, rg) = (v3(&mut r), v3(&mut r), v3(&mut r), r.f(), r.pos());
        let l = GpuLight::spot(p, d, c, k, rg, inner, outer);
        m.row(
            "GpuLight::spot",
            'C',
            &Toks::new()
                .gs(&p)
                .gs(&d)
                .gs(&c)
                .g(k)
                .g(rg)
                .g(inner)
                .g(outer),
            &Toks::new().gs(&light16(&l)),
            &format!("inner {inner:?} outer {outer:?} (case {i})"),
        );
    }

    m.row(
        "LightArrayUniforms::default",
        'A',
        &Toks::new(),
        &Toks::new().gs(&array136(&LightArrayUniforms::default())),
        "default",
    );
    for n in 0..=MAX_LIGHTS + 3 {
        let lights: Vec<GpuLight> = (0..n)
            .map(|_| GpuLight::point(v3(&mut r), v3(&mut r), r.f(), r.pos()))
            .collect();
        let mut u = LightArrayUniforms::default();
        u.set_lights(&lights);
        let mut ins = Toks::new().i(n as i128);
        for l in &lights {
            ins = ins.gs(&light16(l));
        }
        m.row(
            "LightArrayUniforms::set_lights",
            'A',
            &ins,
            &Toks::new().gs(&array136(&u)),
            &format!("{n} lights on the default"),
        );
    }
    for i in 0..5 {
        let (c, k) = (v3(&mut r), r.f());
        let mut u = LightArrayUniforms::default();
        u.set_ambient(c, k);
        m.row(
            "LightArrayUniforms::set_ambient",
            'A',
            &Toks::new().gs(&c).g(k),
            &Toks::new().gs(&array136(&u)),
            &format!("case {i}"),
        );
    }
    m
}
