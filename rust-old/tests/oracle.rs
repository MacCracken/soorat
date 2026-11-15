//! Tests for the hidden `oracle` hooks that the Cyrius port's golden-value
//! generator reads (see `docs/development/soorat-port-plan.md`, D1).
//!
//! The hooks must agree with the public paths they back, and must be
//! read-only: pure functions with no hidden state, so calling them changes no
//! public output.

#![cfg(feature = "oracle")]

use soorat::animation::{
    AnimationChannel, AnimationClip, AnimationProperty, Joint, Keyframe, Skeleton,
};
use soorat::oracle::*;
use soorat::shadow::{ShadowAtlasConfig, compute_practical_splits, tile_uv, tile_viewport};

// ── Helpers ────────────────────────────────────────────────────────────────

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

fn mat_bits(m: &[[f32; 16]]) -> Vec<u32> {
    m.iter().flat_map(|x| bits(x)).collect()
}

fn joint(parent: i32, t: [f32; 3], r: [f32; 4], s: [f32; 3]) -> Joint {
    let mut inverse_bind = IDENTITY_MAT4;
    inverse_bind[12] = -t[0];
    inverse_bind[13] = -t[1];
    inverse_bind[14] = -t[2];
    Joint {
        parent,
        inverse_bind,
        translation: t,
        rotation: normalize4(r),
        scale: s,
    }
}

fn normalize4(q: [f32; 4]) -> [f32; 4] {
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    [q[0] / len, q[1] / len, q[2] / len, q[3] / len]
}

/// A five-joint chain whose parents precede their children.
fn sorted_skeleton() -> Skeleton {
    Skeleton {
        joints: vec![
            joint(-1, [0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0]),
            joint(0, [0.0, 0.5, 0.0], [0.1, 0.2, 0.0, 1.0], [1.0, 2.0, 1.0]),
            joint(1, [0.3, 0.5, 0.0], [0.0, 0.3, 0.1, 1.0], [0.5, 1.0, 1.0]),
            joint(2, [0.0, 0.5, 0.2], [0.2, 0.0, 0.3, 1.0], [1.0, 1.0, 3.0]),
            joint(1, [-0.3, 0.2, 0.1], [0.0, 0.0, 0.7, 0.7], [1.0, 1.0, 1.0]),
        ],
    }
}

/// The same chain stored children-first, plus a self-parent and a 2-cycle.
fn unsorted_skeleton() -> Skeleton {
    let s = sorted_skeleton().joints;
    let mut j = vec![
        s[3].clone(), // child of index 2 here
        s[2].clone(), // child of index 3 here
        s[4].clone(), // child of index 3 here
        s[1].clone(), // child of index 4 here
        s[0].clone(), // root
    ];
    j[0].parent = 1;
    j[1].parent = 3;
    j[2].parent = 3;
    j[3].parent = 4;
    j[4].parent = -1;
    j.push(joint(5, [1.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0; 3])); // self-parent
    j.push(joint(7, [0.0, 2.0, 0.0], [0.0, 0.1, 0.0, 1.0], [1.0; 3])); // cycle 6 ↔ 7
    j.push(joint(6, [0.0, 0.0, 2.0], [0.1, 0.0, 0.0, 1.0], [1.0; 3]));
    Skeleton { joints: j }
}

fn key(time: f32, value: &[f32]) -> Keyframe {
    Keyframe {
        time,
        value: value.to_vec(),
    }
}

fn translation_track() -> Vec<Keyframe> {
    vec![
        key(0.0, &[0.0, 0.0, 0.0]),
        key(0.5, &[1.0, 2.0, -1.0]),
        key(1.25, &[0.25, 0.5, 4.0]),
    ]
}

fn rotation_track() -> Vec<Keyframe> {
    vec![
        key(0.0, &[0.0, 0.0, 0.0, 1.0]),
        key(0.75, &normalize4([0.0, 0.7, 0.0, 0.7])),
        key(1.25, &normalize4([0.3, 0.0, 0.4, -0.8])),
    ]
}

fn sample_times() -> Vec<f32> {
    vec![
        -0.5,
        0.0,
        0.1,
        0.5,
        0.6,
        0.75,
        1.0,
        1.249,
        1.25,
        2.0,
        f32::NAN,
    ]
}

/// Every public output the hooks back, as bits.
fn public_outputs() -> Vec<u32> {
    let mut out = mat_bits(&sorted_skeleton().compute_joint_matrices());
    out.extend(mat_bits(&unsorted_skeleton().compute_joint_matrices()));
    out.extend(bits(&compute_practical_splits(0.1, 250.0, 4, 0.7)));
    #[cfg(feature = "optics")]
    out.extend(
        soorat::pbr_material::generate_brdf_lut(16, 32)
            .into_iter()
            .map(u32::from),
    );
    let (v, i) = soorat::primitives::sphere(12, 8);
    out.extend(v.iter().flat_map(|v| bits(&v.position)));
    out.extend(i);
    let config = ShadowAtlasConfig {
        size: 2048,
        tile_size: 512,
    };
    for index in 0..20 {
        let (x, y, w, h) = tile_viewport(&config, index);
        out.extend([x, y, w, h]);
        out.extend(bits(&tile_uv(&config, index)));
    }
    out
}

/// Every hook, called once, as bits (for the purity check).
fn hook_outputs() -> Vec<u32> {
    let mut out = Vec::new();
    let sorted = sorted_skeleton();
    let unsorted = unsorted_skeleton();
    out.extend(mat_bits(&world_transforms(&sorted.joints)));
    out.extend(mat_bits(&world_transforms(&unsorted.joints)));
    out.extend(mat_bits(&world_transforms_resolved(&unsorted.joints)));
    out.extend(bits(&local_transform(&sorted.joints[1])));
    for t in sample_times() {
        out.extend(bits(&interpolate_keyframes(&translation_track(), t)));
        out.extend(bits(&interpolate_rotation(&rotation_track(), t)));
    }
    out.extend(bits(&slerp(
        [0.0, 0.0, 0.0, 1.0],
        normalize4([0.0, 1.0, 0.0, 1.0]),
        0.3,
    )));
    out.extend(bits(&srgb8_decode_table()));
    out.extend((0..=u16::MAX).step_by(97).map(|h| f16_to_f32(h).to_bits()));
    let face: Vec<u8> = (0..4 * 4 * 4).map(|i| (i * 37 % 256) as u8).collect();
    for (_, level) in face_mip_chain(FaceFormat::Srgb8, &face, 4) {
        out.extend(level.into_iter().map(u32::from));
    }
    let (r0, r1, r2) = inverse_transpose_3x3(&compose_trs(
        [1.0, 2.0, 3.0],
        normalize4([0.2, 0.3, 0.1, 0.9]),
        [1.0, 4.0, 0.5],
    ));
    out.extend(bits(&r0));
    out.extend(bits(&r1));
    out.extend(bits(&r2));
    out.extend(lattice_indices(4, 6));
    out
}

// ── Purity ─────────────────────────────────────────────────────────────────

#[test]
fn hooks_are_pure_and_leave_public_outputs_unchanged() {
    let before = public_outputs();
    let first = hook_outputs();
    let second = hook_outputs();
    let after = public_outputs();
    assert_eq!(
        first, second,
        "a hook returned different bits on a repeat call"
    );
    assert_eq!(before, after, "calling the hooks changed a public output");
}

// ── Animation ──────────────────────────────────────────────────────────────

#[test]
fn joint_matrices_are_world_transforms_times_inverse_bind() {
    for skeleton in [sorted_skeleton(), unsorted_skeleton()] {
        let expected: Vec<[f32; 16]> = world_transforms(&skeleton.joints)
            .iter()
            .zip(&skeleton.joints)
            .map(|(w, j)| mul_mat4(*w, j.inverse_bind))
            .collect();
        assert_eq!(
            mat_bits(&skeleton.compute_joint_matrices()),
            mat_bits(&expected)
        );
    }
}

#[test]
fn general_walk_matches_fast_path_on_sorted_joints() {
    let joints = sorted_skeleton().joints;
    assert_eq!(
        mat_bits(&world_transforms(&joints)),
        mat_bits(&world_transforms_resolved(&joints))
    );
}

#[test]
fn unsorted_order_resolves_to_the_sorted_poses() {
    let sorted = world_transforms(&sorted_skeleton().joints);
    let unsorted = world_transforms(&unsorted_skeleton().joints);
    // unsorted[k] holds sorted joint perm[k].
    let perm = [3, 2, 4, 1, 0];
    for (k, &s) in perm.iter().enumerate() {
        assert_eq!(bits(&unsorted[k]), bits(&sorted[s]), "joint {k}");
    }
    // Self-parent and the 2-cycle are roots: world = local.
    let joints = unsorted_skeleton().joints;
    assert_eq!(bits(&unsorted[5]), bits(&local_transform(&joints[5])));
    // In a cycle one member is broken to a root; the other hangs off it.
    let roots = [6, 7]
        .iter()
        .filter(|&&i| bits(&unsorted[i]) == bits(&local_transform(&joints[i])))
        .count();
    assert_eq!(roots, 1);
}

#[test]
fn local_transform_is_trs() {
    for j in &sorted_skeleton().joints {
        assert_eq!(
            bits(&local_transform(j)),
            bits(&compose_trs(j.translation, j.rotation, j.scale))
        );
    }
}

#[test]
fn clip_sampling_uses_the_interpolation_hooks() {
    let clip = AnimationClip {
        name: "t".into(),
        duration: 1.25,
        channels: vec![
            AnimationChannel {
                joint_index: 1,
                property: AnimationProperty::Translation,
                keyframes: translation_track(),
            },
            AnimationChannel {
                joint_index: 1,
                property: AnimationProperty::Rotation,
                keyframes: rotation_track(),
            },
        ],
    };
    for time in [0.0, 0.1, 0.5, 0.6, 0.75, 1.0, 1.249] {
        let mut skeleton = sorted_skeleton();
        clip.sample(&mut skeleton, time);
        let j = &skeleton.joints[1];
        assert_eq!(
            bits(&j.translation),
            bits(&interpolate_keyframes(&translation_track(), time)),
            "translation at {time}"
        );
        assert_eq!(
            bits(&j.rotation),
            bits(&interpolate_rotation(&rotation_track(), time)),
            "rotation at {time}"
        );
    }
}

#[test]
fn bracket_drives_interpolation() {
    let track = translation_track();
    for time in sample_times() {
        let lerped = interpolate_keyframes(&track, time);
        match bracket(&track, time).expect("non-empty track") {
            BracketDump::At(v) => assert_eq!(bits(&lerped), bits(&v)),
            BracketDump::Between(a, b, t) => {
                assert!((0.0..=1.0).contains(&t));
                let manual: Vec<f32> = a.iter().zip(&b).map(|(x, y)| x + (y - x) * t).collect();
                assert_eq!(bits(&lerped), bits(&manual));
            }
            other => panic!("unexpected bracket {other:?}"),
        }
    }
    assert_eq!(bracket(&[], 0.5), None);
}

#[test]
fn build_keyframes_pairs_each_sampler_mode() {
    let times = vec![0.0, 1.0];
    let linear = build_keyframes(
        times.clone(),
        vec![vec![1.0], vec![2.0]],
        SamplerMode::Linear,
    );
    assert_eq!(linear.len(), 2);
    let step = build_keyframes(times.clone(), vec![vec![1.0], vec![2.0]], SamplerMode::Step);
    // The step is a duplicate-time key holding the previous value.
    let step: Vec<(f32, Vec<f32>)> = step.into_iter().map(|k| (k.time, k.value)).collect();
    assert_eq!(
        step,
        vec![(0.0, vec![1.0]), (1.0, vec![1.0]), (1.0, vec![2.0])]
    );
    let cubic = build_keyframes(
        times,
        vec![
            vec![9.0],
            vec![1.0],
            vec![9.0],
            vec![9.0],
            vec![2.0],
            vec![9.0],
        ],
        SamplerMode::CubicSpline,
    );
    let values: Vec<Vec<f32>> = cubic.into_iter().map(|k| k.value).collect();
    assert_eq!(values, vec![vec![1.0], vec![2.0]]);
}

#[test]
fn slerp_endpoints_and_shortest_arc() {
    let a = [0.0, 0.0, 0.0, 1.0];
    let b = normalize4([0.0, 1.0, 0.0, 1.0]);
    assert_eq!(bits(&slerp(a, b, 0.0)), bits(&a));
    let end = slerp(a, b, 1.0);
    for c in 0..4 {
        assert!((end[c] - b[c]).abs() <= 2.0 * f32::EPSILON);
    }
    // -b is the same rotation: the result must match slerp toward b.
    let neg_b = [-b[0], -b[1], -b[2], -b[3]];
    assert_eq!(bits(&slerp(a, neg_b, 0.4)), bits(&slerp(a, b, 0.4)));
}

// ── Environment maps ───────────────────────────────────────────────────────

#[test]
fn srgb_table_is_the_decode_transfer() {
    let table = srgb8_decode_table();
    for (i, v) in table.iter().enumerate() {
        assert_eq!(v.to_bits(), srgb_to_linear(i as f32 / 255.0).to_bits());
    }
    let face: Vec<u8> = (0..=255).flat_map(|b| [b, b, b, 255]).collect();
    let texels = decode_face(FaceFormat::Srgb8, &face);
    for (b, px) in texels.iter().enumerate() {
        assert_eq!(px[0].to_bits(), table[b].to_bits());
        assert_eq!(px[3], 1.0);
    }
}

#[test]
fn srgb8_faces_round_trip_every_byte() {
    let face: Vec<u8> = (0..=255u8).flat_map(|b| [b, 255 - b, b, b]).collect();
    let back = encode_face(FaceFormat::Srgb8, &decode_face(FaceFormat::Srgb8, &face));
    assert_eq!(back, face);
}

#[test]
fn f16_round_trips_every_non_nan_pattern() {
    for h in 0..=u16::MAX {
        let is_nan = (h >> 10) & 0x1f == 0x1f && h & 0x3ff != 0;
        let back = f32_to_f16(f16_to_f32(h));
        if is_nan {
            assert!(f16_to_f32(h).is_nan());
            assert_eq!(back & 0x7c00, 0x7c00);
            assert_ne!(back & 0x3ff, 0);
        } else {
            assert_eq!(back, h, "pattern {h:#06x}");
        }
    }
}

#[test]
fn f16_faces_round_trip() {
    let texels: Vec<[f32; 4]> = (0..16)
        .map(|i| [i as f32 * 0.25, -(i as f32), 1.0 / (i as f32 + 1.0), 1.0])
        .collect();
    let bytes = encode_face(FaceFormat::F16, &texels);
    let back = decode_face(FaceFormat::F16, &bytes);
    let again = encode_face(FaceFormat::F16, &back);
    assert_eq!(bytes, again);
}

#[test]
fn mip_chain_is_repeated_downsample() {
    for size in [1u32, 2, 3, 4, 5, 8] {
        let side = size as usize;
        let face: Vec<u8> = (0..side * side * 4).map(|i| (i * 53 % 256) as u8).collect();
        let chain = face_mip_chain(FaceFormat::Srgb8, &face, size);
        let expected_levels = (32 - size.leading_zeros()) as usize - 1;
        assert_eq!(chain.len(), expected_levels, "size {size}");

        let mut level = decode_face(FaceFormat::Srgb8, &face);
        let mut level_size = side;
        for (edge, bytes) in chain {
            level = downsample(&level, level_size);
            level_size = (level_size / 2).max(1);
            assert_eq!(edge, level_size);
            assert_eq!(bytes, encode_face(FaceFormat::Srgb8, &level));
        }
    }
}

#[test]
fn downsample_rejects_bad_input_without_panicking() {
    assert!(downsample(&[], 0).is_empty());
    assert!(downsample(&[[0.0; 4]; 3], 2).is_empty());
    assert_eq!(downsample(&[[1.0; 4]; 4], 2), vec![[1.0; 4]]);
}

// ── Shadows and meshes ─────────────────────────────────────────────────────

#[test]
fn atlas_hooks_match_public_tile_math() {
    for (size, tile_size) in [(2048, 512), (1000, 300), (64, 0), (0, 16), (512, 1024)] {
        let config = ShadowAtlasConfig { size, tile_size };
        let columns = atlas_columns(size, tile_size);
        assert!(columns >= 1);
        for index in 0..40 {
            assert_eq!(
                tile_viewport(&config, index),
                tile_viewport_in(columns, tile_size, index)
            );
            assert_eq!(
                bits(&tile_uv(&config, index)),
                bits(&tile_uv_in(columns, tile_size, size, index))
            );
        }
    }
}

#[test]
fn normal_matrix_inverts_the_transpose() {
    let m = compose_trs(
        [1.0, 2.0, 3.0],
        normalize4([0.2, 0.3, 0.1, 0.9]),
        [1.0, 4.0, 0.5],
    );
    let (r0, r1, r2) = inverse_transpose_3x3(&m);
    // (M⁻ᵀ · Mᵀ)ᵢⱼ = Σₖ rᵢ[k] · M[j][k], with M column-major: M[j][k] = m[k*4 + j].
    for (i, r) in [r0, r1, r2].iter().enumerate() {
        for j in 0..3 {
            let v: f32 = (0..3).map(|k| r[k] * m[k * 4 + j]).sum();
            let want = if i == j { 1.0 } else { 0.0 };
            assert!((v - want).abs() < 1e-5, "({i},{j}) = {v}");
        }
    }
}

// ── Visualization ──────────────────────────────────────────────────────────

#[test]
fn viz_reexports_are_reachable() {
    assert_eq!(grid_len(&[3, 4, 5]), Some(60));
    assert_eq!(grid_len(&[usize::MAX, 2]), None);
    assert!(grid_is_backed(&[2, 3], 6, "test"));
    assert!(!grid_is_backed(&[2, 3], 5, "test"));
    assert_eq!(
        finite_min_max([1.0, f64::NAN, -2.0, f64::INFINITY]),
        Some((-2.0, 1.0))
    );
    assert_eq!(lattice_indices(2, 3).len(), 2 * 3 * 6);
    assert_eq!(XZ_QUAD_INDICES.len(), 6);
    const { assert!(MAX_SEGMENTS >= 4 && MAX_QUADS > 0) };
    let grid = XzGrid {
        nx: 2,
        nz: 3,
        origin_x: 0.0,
        origin_z: 0.0,
        y: 0.0,
        dx: 1.0,
        dz: 1.0,
    };
    let (v, i) = xz_heatmap_mesh(&grid, |_, _| heat_rgba(0.5, 1.0));
    assert_eq!((v.len(), i.len()), (2 * 3 * 4, 2 * 3 * 6));
    let hf = HeightField {
        cols: 3,
        rows: 2,
        origin_x: 0.0,
        origin_z: 0.0,
        dx: 1.0,
        dz: 1.0,
    };
    let (v, _) = heightfield_mesh(&hf, |_, _| 0.0, |_, _, _| signed_rgba(0.0, 1.0));
    assert_eq!(v.len(), 6);
}
