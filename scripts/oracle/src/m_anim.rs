//! `animation` (M3 part b): joints, skeletons and their joint matrices, joint uniforms, keyframe brackets and
//! interpolation, slerp, sampler pairing and clip sampling. `load_gltf_animations` is part c (the GLB loader).
//!
//! Encodings (tokens in `scripts/oracle/README.md`):
//! - a keyframe track: `i:n`, then per key `g:time` and its value `v:len g:…`;
//! - a joint: `i:parent`, `v:16` inverse bind, `v:3` translation, `v:4` rotation, `v:3` scale;
//!   a skeleton: `i:n` then its joints;
//! - a channel: `i:joint_index i:property` (0 translation, 1 rotation, 2 scale) then its track;
//!   a clip: `g:duration i:channels` then its channels;
//! - matrices: one `v:16n` sequence.
//!
//! Joint matrices, local and world transforms, slerp, rotation tracks and sampling are class **E** (D4 rule b:
//! `validate.py --write` writes `animation.e.tsv`, Rust's formula in f64 rounded once, where that differs).
//! Brackets, linear interpolation, sampler pairing, defaults and uniforms are class A. No grid here calls a
//! transcendental function: unit quaternions are normalized with `sqrt`.

use crate::{Module, Rng, Toks};
use soorat::animation::{
    AnimationChannel, AnimationClip, AnimationProperty, Joint, JointUniforms, Keyframe, MAX_JOINTS,
    Skeleton,
};
use soorat::oracle::IDENTITY_MAT4;
use soorat::oracle::{
    BracketDump, SamplerMode, bracket, build_keyframes, interpolate_keyframes,
    interpolate_rotation, local_transform, slerp, world_transforms, world_transforms_resolved,
};

fn track(mut t: Toks, keys: &[Keyframe]) -> Toks {
    t = t.i(keys.len() as i128);
    for k in keys {
        t = t.g(k.time).gs(&k.value);
    }
    t
}

fn joint_toks(t: Toks, j: &Joint) -> Toks {
    t.i(j.parent as i128)
        .gs(&j.inverse_bind)
        .gs(&j.translation)
        .gs(&j.rotation)
        .gs(&j.scale)
}

fn skeleton_toks(mut t: Toks, joints: &[Joint]) -> Toks {
    t = t.i(joints.len() as i128);
    for j in joints {
        t = joint_toks(t, j);
    }
    t
}

fn mats(ms: &[[f32; 16]]) -> Toks {
    let f: Vec<f32> = ms.iter().flatten().copied().collect();
    Toks::new().gs(&f)
}

fn unit_quat(r: &mut Rng) -> [f32; 4] {
    loop {
        let q = [r.f(), r.f(), r.f(), r.f()];
        let l2 = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
        if l2 > 1e-3 {
            let l = l2.sqrt();
            return [q[0] / l, q[1] / l, q[2] / l, q[3] / l];
        }
    }
}

fn normalize4(q: [f32; 4]) -> [f32; 4] {
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    [q[0] / l, q[1] / l, q[2] / l, q[3] / l]
}

/// A joint with a random small pose: translation within ±2, a unit rotation, scale near 1.
fn random_joint(r: &mut Rng, parent: i32, bind: bool) -> Joint {
    let inverse_bind = if bind {
        let mut m = IDENTITY_MAT4;
        for v in m.iter_mut().take(15) {
            *v += r.f() / 64.0;
        }
        m
    } else {
        IDENTITY_MAT4
    };
    Joint {
        parent,
        inverse_bind,
        translation: [r.f() / 4.0, r.f() / 4.0, r.f() / 4.0],
        rotation: unit_quat(r),
        scale: [1.0 + r.f() / 32.0, 1.0 + r.f() / 32.0, 1.0 + r.f() / 32.0],
    }
}

fn key(time: f32, value: &[f32]) -> Keyframe {
    Keyframe {
        time,
        value: value.to_vec(),
    }
}

pub fn module() -> Module {
    let mut m = Module::new("animation");
    let mut r = Rng(0x0A11_A710_5EED_0003);

    // ── defaults and uniforms ──
    let j = Joint::default();
    m.row(
        "Joint::default",
        'A',
        &Toks::new(),
        &joint_toks(Toks::new(), &j),
        "Joint::default()",
    );
    let u = JointUniforms::default();
    m.row(
        "JointUniforms::default",
        'A',
        &Toks::new(),
        &Toks::new().gs(&u.joint_count).gs(&u
            .joints
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<f32>>()),
        "JointUniforms::default()",
    );
    for n in [0usize, 1, 3, MAX_JOINTS, MAX_JOINTS + 1] {
        let ms: Vec<[f32; 16]> = (0..n)
            .map(|_| {
                let mut a = [0.0f32; 16];
                for v in &mut a {
                    *v = r.f();
                }
                a
            })
            .collect();
        let mut u = JointUniforms::default();
        u.set_joints(&ms);
        m.row(
            "JointUniforms::set_joints",
            'A',
            &Toks::new()
                .i(n as i128)
                .gs(&ms.iter().flatten().copied().collect::<Vec<f32>>()),
            &Toks::new().gs(&u.joint_count).gs(&u
                .joints
                .iter()
                .flatten()
                .copied()
                .collect::<Vec<f32>>()),
            &format!("set_joints({n} matrices) on the default"),
        );
    }

    // ── slerp: the cos sweep across the 0.9995 switch, random pairs, degenerate inputs ──
    let sw = 0.9995f32.to_bits();
    let mut cs: Vec<f32> = vec![
        1.0, 0.99999, 0.9999, 0.9996, 0.9994, 0.999, 0.99, 0.9, 0.7, 0.5, 0.0, -0.3, -0.99999, -1.0,
    ];
    for d in -3i32..=3 {
        cs.push(f32::from_bits((sw as i32 + d) as u32));
        cs.push(-f32::from_bits((sw as i32 + d) as u32));
    }
    let ts = [0.0f32, 0.25, 0.5, 0.75, 1.0, -0.5, 1.5, 1e-6];
    let id = [0.0f32, 0.0, 0.0, 1.0];
    for &c in &cs {
        let s = (1.0 - c * c).max(0.0).sqrt();
        let b = [0.0, 0.0, s, c];
        for &t in &ts {
            let q = slerp(id, b, t);
            m.row(
                "slerp",
                'E',
                &Toks::new().gs(&id).gs(&b).g(t),
                &Toks::new().gs(&q),
                &format!("slerp(identity, cos {c:e} about z, {t})"),
            );
        }
    }
    for k in 0..150 {
        let a = unit_quat(&mut r);
        let b = unit_quat(&mut r);
        let t = r.f().abs() / 8.0;
        m.row(
            "slerp",
            'E',
            &Toks::new().gs(&a).gs(&b).g(t),
            &Toks::new().gs(&slerp(a, b, t)),
            &format!("random pair {k}"),
        );
    }
    for k in 0..60 {
        let a = unit_quat(&mut r);
        let eps = 0.02 + 0.0004 * k as f32;
        let d = unit_quat(&mut r);
        let b = normalize4([
            a[0] + eps * d[0],
            a[1] + eps * d[1],
            a[2] + eps * d[2],
            a[3] + eps * d[3],
        ]);
        let t = r.f().abs() / 8.0;
        m.row(
            "slerp",
            'E',
            &Toks::new().gs(&a).gs(&b).g(t),
            &Toks::new().gs(&slerp(a, b, t)),
            &format!("near the switch {k} (eps {eps})"),
        );
    }
    let a = unit_quat(&mut r);
    let neg = [-a[0], -a[1], -a[2], -a[3]];
    let specials: Vec<([f32; 4], [f32; 4], f32, &str)> = vec![
        (a, a, 0.3, "a = b"),
        (a, neg, 0.3, "b = -a"),
        ([0.0; 4], [0.0; 4], 0.5, "zero quaternions"),
        ([0.0; 4], id, 0.5, "zero to identity"),
        (id, [0.0, 0.0, 0.0, 2.0], 0.5, "non-unit, parallel"),
        (
            [0.0, 0.0, 0.0, 0.5],
            [0.0, 0.5, 0.0, 0.0],
            0.5,
            "non-unit, orthogonal",
        ),
        (
            [0.0, 0.0, 0.0, 2.0],
            [0.0, 0.0, 2.0, 0.0],
            0.25,
            "non-unit (cos 0 after scaling)",
        ),
        (
            [3.0, 0.0, 0.0, 4.0],
            [0.0, 4.0, 3.0, 0.0],
            0.5,
            "non-unit, cos 0",
        ),
        (
            [1.0, 1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            0.5,
            "non-unit, cos 1 (clamped acos)",
        ),
        (id, [0.0, 0.0, f32::NAN, 1.0], 0.5, "NaN component"),
        (
            id,
            [0.0, 0.0, f32::INFINITY, 1.0],
            0.5,
            "infinite component",
        ),
        (id, [0.0, 0.0, 1.0, 0.0], f32::NAN, "NaN t"),
        (id, [0.0, 0.0, 1.0, 0.0], f32::INFINITY, "infinite t"),
        (id, [0.0, 0.0, 1.0, 0.0], -0.0, "t = -0"),
        (
            [-0.0, -0.0, -0.0, 1.0],
            [0.0, 0.0, 0.0, -1.0],
            0.5,
            "signed zeros, b = -a",
        ),
        (
            [1e-20, 0.0, 0.0, 0.0],
            [0.0, 1e-20, 0.0, 0.0],
            0.5,
            "tiny: length below 1e-12",
        ),
        (
            [1e30, 0.0, 0.0, 1e30],
            [1e30, 0.0, 0.0, 1e30],
            0.5,
            "huge: the cos overflows",
        ),
    ];
    for (a, b, t, what) in specials {
        m.row(
            "slerp",
            'E',
            &Toks::new().gs(&a).gs(&b).g(t),
            &Toks::new().gs(&slerp(a, b, t)),
            what,
        );
    }

    // ── brackets and interpolation over tracks ──
    let eps = f32::EPSILON;
    let next1 = f32::from_bits(1.0f32.to_bits() + 1);
    let mut tracks: Vec<(Vec<f32>, usize, &str)> = vec![
        (vec![0.0, 0.5, 1.0, 2.0, 4.0], 3, "sorted"),
        (vec![0.0, 0.5, 1.0, 2.0, 4.0], 4, "sorted, quaternions"),
        (
            vec![0.0, 1.0, 1.0, 2.0, 2.0, 3.0],
            4,
            "duplicate times (STEP-encoded)",
        ),
        (vec![1.0], 3, "one key"),
        (vec![], 3, "empty"),
        (vec![0.0, 3.0, 1.0, 2.0, 5.0, 4.0], 4, "unsorted"),
        (vec![0.0, f32::NAN, 2.0, 3.0], 3, "a NaN time"),
        (
            vec![0.0, 1.0, next1, 2.0],
            4,
            "keys one ulp apart (dt = EPSILON)",
        ),
        (
            vec![0.0, 1.0, 1.0 + 2.0 * eps, 2.0],
            3,
            "keys two ulps apart",
        ),
        (
            vec![-2.0, -1.0, -0.0, 0.0, 1e-7],
            4,
            "negative times, signed zeros, tiny dt",
        ),
        (
            vec![f32::NEG_INFINITY, 0.0, f32::INFINITY],
            4,
            "infinite times",
        ),
        (
            (0..32).map(|k| k as f32 / 31.0).collect(),
            4,
            "32 keys at k / 31 (the bench track)",
        ),
        (vec![5.0, 4.0, 3.0, 2.0, 1.0, 0.0], 4, "descending"),
    ];
    tracks.push((vec![0.0, 1.0, 2.0], 0, "mixed value lengths"));
    for (times, width, what) in &tracks {
        let keys: Vec<Keyframe> = times
            .iter()
            .enumerate()
            .map(|(i, &t)| {
                let w = if *width == 0 {
                    [3usize, 4, 2][i % 3]
                } else {
                    *width
                };
                let v: Vec<f32> = if w == 4 {
                    unit_quat(&mut r).to_vec()
                } else {
                    (0..w).map(|_| r.f()).collect()
                };
                key(t, &v)
            })
            .collect();
        let mut qs: Vec<f32> = vec![
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            -0.0,
            0.0,
            -100.0,
            100.0,
        ];
        for w in times.windows(2) {
            qs.push(w[0]);
            qs.push(w[0] + (w[1] - w[0]) * 0.25);
            qs.push(w[0] + (w[1] - w[0]) * 0.5);
        }
        if let Some(&l) = times.last() {
            qs.push(l);
        }
        for _ in 0..4 {
            qs.push(r.f().abs() / 2.0);
        }
        for &q in &qs {
            let ins = track(Toks::new(), &keys).g(q);
            let b = match bracket(&keys, q) {
                None => Toks::new().none(),
                Some(BracketDump::At(v)) => Toks::new().s("At").gs(&v),
                Some(BracketDump::Between(a, b, t)) => Toks::new().s("Between").gs(&a).gs(&b).g(t),
                Some(_) => Toks::new().s("?"),
            };
            m.row("bracket", 'A', &ins, &b, &format!("{what} at {q}"));
            m.row(
                "interpolate_keyframes",
                'A',
                &ins,
                &Toks::new().gs(&interpolate_keyframes(&keys, q)),
                &format!("{what} at {q}"),
            );
            m.row(
                "interpolate_rotation",
                'E',
                &ins,
                &Toks::new().gs(&interpolate_rotation(&keys, q)),
                &format!("{what} at {q}"),
            );
        }
    }

    // ── sampler pairing ──
    type Pairing = (SamplerMode, i128, Vec<f32>, usize, Vec<f32>, &'static str);
    let pairing: Vec<Pairing> = vec![
        (
            SamplerMode::Linear,
            0,
            vec![0.0, 1.0, 2.0],
            1,
            vec![0.0, 10.0, 20.0],
            "linear",
        ),
        (
            SamplerMode::Step,
            1,
            vec![0.0, 1.0, 2.0],
            1,
            vec![0.0, 10.0, 20.0],
            "step (the Rust test)",
        ),
        (
            SamplerMode::CubicSpline,
            2,
            vec![0.0, 1.0],
            1,
            vec![9.0, 1.0, 9.0, 9.0, 3.0, 9.0],
            "cubic (the Rust test)",
        ),
        (
            SamplerMode::Linear,
            0,
            vec![0.0, 1.0, 2.0, 3.0],
            3,
            (0..6).map(|k| k as f32).collect(),
            "linear, fewer values",
        ),
        (
            SamplerMode::Linear,
            0,
            vec![0.0, 1.0],
            3,
            (0..12).map(|k| k as f32).collect(),
            "linear, fewer times",
        ),
        (
            SamplerMode::Step,
            1,
            vec![0.0, 0.5, 1.0, 1.5],
            4,
            (0..16).map(|k| k as f32 * 0.5).collect(),
            "step, quaternion width",
        ),
        (
            SamplerMode::Step,
            1,
            vec![0.0, 1.0, 2.0],
            3,
            (0..15).map(|k| k as f32).collect(),
            "step, more values",
        ),
        (
            SamplerMode::Step,
            1,
            vec![0.0, 1.0, 2.0, 3.0, 4.0],
            2,
            vec![1.0, 2.0, 3.0, 4.0],
            "step, fewer values",
        ),
        (
            SamplerMode::Step,
            1,
            vec![2.0],
            3,
            vec![1.0, 2.0, 3.0],
            "step, one key",
        ),
        (
            SamplerMode::CubicSpline,
            2,
            vec![0.0, 1.0, 2.0],
            3,
            (0..27).map(|k| k as f32).collect(),
            "cubic, width 3",
        ),
        (
            SamplerMode::CubicSpline,
            2,
            vec![0.0, 1.0, 2.0],
            1,
            (0..8).map(|k| k as f32).collect(),
            "cubic, a truncated triplet (8 values)",
        ),
        (
            SamplerMode::CubicSpline,
            2,
            vec![0.0, 1.0, 2.0],
            1,
            (0..5).map(|k| k as f32).collect(),
            "cubic, 5 values",
        ),
        (
            SamplerMode::CubicSpline,
            2,
            vec![0.0],
            1,
            vec![7.0],
            "cubic, one value",
        ),
        (SamplerMode::Linear, 0, vec![], 3, vec![], "empty"),
        (SamplerMode::Step, 1, vec![], 3, vec![], "step, empty"),
        (
            SamplerMode::CubicSpline,
            2,
            vec![0.0, 1.0],
            4,
            (0..24).map(|k| k as f32 * 0.25).collect(),
            "cubic, width 4",
        ),
    ];
    for (mode, code, times, width, flat, what) in pairing {
        let values: Vec<Vec<f32>> = flat.chunks(width).map(|c| c.to_vec()).collect();
        let keys = build_keyframes(times.clone(), values, mode);
        m.row(
            "build_keyframes",
            'A',
            &Toks::new().i(code).gs(&times).i(width as i128).gs(&flat),
            &track(Toks::new(), &keys),
            what,
        );
    }

    // ── local and world transforms, joint matrices ──
    for k in 0..60 {
        let mut jt = random_joint(&mut r, -1, false);
        match k % 6 {
            1 => jt.rotation = [r.f(), r.f(), r.f(), r.f()],
            2 => jt.scale = [r.f(), r.f(), r.f()],
            3 => jt.scale = [0.0, -1.0, 2.0],
            4 => jt.translation = [r.f() * 1000.0, -r.f(), 0.0],
            _ => {}
        }
        m.row(
            "local_transform",
            'E',
            &joint_toks(Toks::new(), &jt),
            &Toks::new().gs(&local_transform(&jt)),
            &format!("random joint {k}"),
        );
    }
    let mut skeletons: Vec<(Vec<Joint>, String)> = Vec::new();
    for n in [1usize, 2, 5, 16, 64] {
        let joints = (0..n)
            .map(|i| random_joint(&mut r, i as i32 - 1, i % 2 == 1))
            .collect();
        skeletons.push((joints, format!("sorted chain of {n}")));
    }
    for n in [3usize, 64] {
        let joints = (0..n)
            .map(|i| random_joint(&mut r, if i + 1 < n { i as i32 + 1 } else { -1 }, true))
            .collect();
        skeletons.push((joints, format!("leaf-first chain of {n}")));
    }
    for n in [10usize, 30] {
        let joints = (0..n)
            .map(|_| {
                let p = (r.next() % (n as u64 + 1)) as i32 - 1;
                random_joint(&mut r, p, true)
            })
            .collect();
        skeletons.push((joints, format!("random parents, {n} joints")));
    }
    {
        let mut joints = Vec::new();
        for (p, t) in [
            (1, [1.0, 0.0, 0.0]),
            (0, [0.0, 1.0, 0.0]),
            (2, [0.0, 0.0, 1.0]),
            (99, [0.0, 0.0, 0.0]),
        ] {
            joints.push(Joint {
                parent: p,
                translation: t,
                ..Joint::default()
            });
        }
        skeletons.push((
            joints,
            "a cycle, a self-parent, an out-of-range parent (the Rust test)".into(),
        ));
    }
    {
        let joints = (0..10)
            .map(|i| {
                random_joint(
                    &mut r,
                    if i == 0 {
                        -1
                    } else if i == 9 {
                        0
                    } else {
                        i + 1
                    },
                    true,
                )
            })
            .collect();
        skeletons.push((
            joints,
            "a chain whose root is listed last but one, and a long cycle".into(),
        ));
    }
    {
        let joints = (0..(MAX_JOINTS + 2))
            .map(|i| random_joint(&mut r, i as i32 - 1, false))
            .collect();
        skeletons.push((
            joints,
            format!("sorted chain of {} (past MAX_JOINTS)", MAX_JOINTS + 2),
        ));
    }
    {
        let joints = (0..8)
            .map(|i| {
                let mut jt = random_joint(&mut r, i - 1, true);
                jt.scale = [2.0, 2.0, 2.0];
                jt.translation = [100.0 + r.f(), 0.0, -100.0];
                jt
            })
            .collect();
        skeletons.push((joints, "chain of 8, scale 2, far translations".into()));
    }
    skeletons.push((Vec::new(), "empty".into()));
    for (joints, what) in &skeletons {
        let ins = skeleton_toks(Toks::new(), joints);
        m.row(
            "world_transforms",
            'E',
            &ins,
            &mats(&world_transforms(joints)),
            what,
        );
        m.row(
            "world_transforms_resolved",
            'E',
            &ins,
            &mats(&world_transforms_resolved(joints)),
            what,
        );
        let sk = Skeleton {
            joints: joints.clone(),
        };
        m.row(
            "Skeleton::compute_joint_matrices",
            'E',
            &ins,
            &mats(&sk.compute_joint_matrices()),
            what,
        );
    }

    // ── clip sampling ──
    let tr = |a: f32, b: f32| vec![key(0.0, &[a, 0.0, 0.0]), key(2.0, &[b, 0.0, 0.0])];
    let mut clips: Vec<(AnimationClip, Vec<Joint>, Vec<f32>, &str)> = Vec::new();
    let times_a = vec![
        -0.5,
        0.0,
        0.5,
        1.5,
        2.0,
        2.5,
        7.3,
        -7.3,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e9,
        -1e-9,
        -0.0,
        1e-30,
    ];
    clips.push((
        AnimationClip {
            name: "move".into(),
            duration: 2.0,
            channels: vec![AnimationChannel {
                joint_index: 0,
                property: AnimationProperty::Translation,
                keyframes: tr(0.0, 2.0),
            }],
        },
        vec![Joint::default()],
        times_a.clone(),
        "translation 0 → 2 over 2 s (the Rust test)",
    ));
    let rot_keys: Vec<Keyframe> = (0..5)
        .map(|k| key(k as f32 * 0.25, &unit_quat(&mut r)))
        .collect();
    let scale_keys: Vec<Keyframe> = (0..3)
        .map(|k| key(k as f32 * 0.5, &[1.0 + r.f() / 8.0, 1.0, 2.0]))
        .collect();
    let tr_keys: Vec<Keyframe> = (0..4)
        .map(|k| key(k as f32 / 3.0, &[r.f(), r.f(), r.f()]))
        .collect();
    let mixed = AnimationClip {
        name: "mixed".into(),
        duration: 1.0,
        channels: vec![
            AnimationChannel {
                joint_index: 1,
                property: AnimationProperty::Rotation,
                keyframes: rot_keys.clone(),
            },
            AnimationChannel {
                joint_index: 0,
                property: AnimationProperty::Scale,
                keyframes: scale_keys,
            },
            AnimationChannel {
                joint_index: 2,
                property: AnimationProperty::Translation,
                keyframes: tr_keys,
            },
            AnimationChannel {
                joint_index: 5,
                property: AnimationProperty::Rotation,
                keyframes: rot_keys.clone(),
            },
            AnimationChannel {
                joint_index: 0,
                property: AnimationProperty::Translation,
                keyframes: vec![key(0.0, &[1.0, 2.0]), key(1.0, &[3.0, 4.0])],
            },
            AnimationChannel {
                joint_index: 2,
                property: AnimationProperty::Rotation,
                keyframes: vec![key(0.0, &[0.0, 0.0, 0.0]), key(1.0, &[1.0, 0.0, 0.0])],
            },
            AnimationChannel {
                joint_index: 1,
                property: AnimationProperty::Scale,
                keyframes: vec![],
            },
        ],
    };
    let three: Vec<Joint> = (0..3).map(|i| random_joint(&mut r, i - 1, false)).collect();
    let mut times_b: Vec<f32> = (0..=20).map(|k| k as f32 * 0.05).collect();
    times_b.extend([-0.3, 1.7, 0.999_999_9, 13.37]);
    clips.push((
        mixed,
        three.clone(),
        times_b.clone(),
        "rotation, scale, translation; out-of-range and short channels",
    ));
    for (d, what) in [
        (0.0f32, "duration 0"),
        (-1.0, "duration -1"),
        (f32::NAN, "duration NaN"),
        (f32::INFINITY, "duration +inf"),
    ] {
        let c = AnimationClip {
            name: "d".into(),
            duration: d,
            channels: vec![AnimationChannel {
                joint_index: 1,
                property: AnimationProperty::Rotation,
                keyframes: rot_keys.clone(),
            }],
        };
        clips.push((c, three.clone(), vec![0.6, -0.6, 3.0], what));
    }
    let step_rot = build_keyframes(
        vec![0.0, 0.5],
        vec![id.to_vec(), unit_quat(&mut r).to_vec()],
        SamplerMode::Step,
    );
    clips.push((
        AnimationClip {
            name: "step".into(),
            duration: 1.0,
            channels: vec![AnimationChannel {
                joint_index: 0,
                property: AnimationProperty::Rotation,
                keyframes: step_rot,
            }],
        },
        vec![Joint::default()],
        vec![0.0, 0.25, 0.4999, 0.5, 0.75, 0.9999],
        "STEP rotation (duplicate-time keys)",
    ));
    let bench_channels: Vec<AnimationChannel> = (0..64)
        .map(|j| AnimationChannel {
            joint_index: j,
            property: AnimationProperty::Rotation,
            keyframes: (0..32)
                .map(|k| key(k as f32 / 31.0, &unit_quat(&mut r)))
                .collect(),
        })
        .collect();
    clips.push((
        AnimationClip {
            name: "bench".into(),
            duration: 1.0,
            channels: bench_channels,
        },
        vec![Joint::default(); 64],
        vec![0.437, 0.0, 0.999],
        "64 rotation channels × 32 keys (the bench clip, random unit keys)",
    ));
    for (clip, joints, times, what) in &clips {
        for &t in times {
            let mut ins = Toks::new().g(clip.duration).i(clip.channels.len() as i128);
            for c in &clip.channels {
                let p = match c.property {
                    AnimationProperty::Translation => 0,
                    AnimationProperty::Rotation => 1,
                    AnimationProperty::Scale => 2,
                    _ => 9,
                };
                ins = track(ins.i(c.joint_index as i128).i(p), &c.keyframes);
            }
            ins = skeleton_toks(ins, joints).g(t);
            let mut sk = Skeleton {
                joints: joints.clone(),
            };
            clip.sample(&mut sk, t);
            let out: Vec<f32> = sk
                .joints
                .iter()
                .flat_map(|j| {
                    j.translation
                        .iter()
                        .chain(&j.rotation)
                        .chain(&j.scale)
                        .copied()
                        .collect::<Vec<f32>>()
                })
                .collect();
            m.row(
                "AnimationClip::sample",
                'E',
                &ins,
                &Toks::new().gs(&out),
                &format!("{what}, at {t}"),
            );
        }
    }
    m
}
