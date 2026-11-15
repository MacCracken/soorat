//! Skeletal animation — joints, skins, animation clips.

use crate::error::{RenderError, Result};
use crate::math_util::{IDENTITY_MAT4, compose_trs, flatten_mat4, mul_mat4};

/// Maximum joints per skin (matches common GPU uniform limits).
pub const MAX_JOINTS: usize = 128;

/// A joint in a skeleton hierarchy.
#[derive(Debug, Clone)]
pub struct Joint {
    /// Index of parent joint (-1 for root).
    pub parent: i32,
    /// Inverse bind matrix (column-major 4x4).
    pub inverse_bind: [f32; 16],
    /// Local transform: translation.
    pub translation: [f32; 3],
    /// Local transform: rotation (quaternion xyzw).
    pub rotation: [f32; 4],
    /// Local transform: scale.
    pub scale: [f32; 3],
}

impl Default for Joint {
    fn default() -> Self {
        Self {
            parent: -1,
            inverse_bind: IDENTITY_MAT4,
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0], // identity quaternion
            scale: [1.0, 1.0, 1.0],
        }
    }
}

/// A skeleton (skin) — hierarchy of joints with their inverse bind matrices.
#[derive(Debug, Clone)]
pub struct Skeleton {
    pub joints: Vec<Joint>,
}

impl Skeleton {
    /// Compute the joint matrices for the current pose.
    /// Returns up to MAX_JOINTS matrices (each 4x4 column-major).
    ///
    /// Parents may appear in any order relative to their children (glTF does
    /// not require topological joint order). A parent index that is out of
    /// range, refers to the joint itself, or closes a cycle is treated as a
    /// root.
    #[must_use]
    pub fn compute_joint_matrices(&self) -> Vec<[f32; 16]> {
        let count = self.joints.len().min(MAX_JOINTS);
        let joints = &self.joints[..count];

        // Joint matrix = world_transform * inverse_bind
        world_transforms(joints)
            .iter()
            .zip(joints)
            .map(|(wt, joint)| mul_mat4(*wt, joint.inverse_bind))
            .collect()
    }
}

/// A joint's local transform (T·R·S, column-major).
#[must_use]
#[inline]
pub(crate) fn local_transform(joint: &Joint) -> [f32; 16] {
    compose_trs(joint.translation, joint.rotation, joint.scale)
}

/// World transforms of `joints` (before the inverse bind). Parents that
/// precede their children take a single forward pass; any other order goes
/// through [`resolve_world_transforms`].
#[must_use]
pub(crate) fn world_transforms(joints: &[Joint]) -> Vec<[f32; 16]> {
    let mut world = vec![IDENTITY_MAT4; joints.len()];

    // Fast path: parents precede children (the common, sorted case).
    let sorted = joints
        .iter()
        .enumerate()
        .all(|(i, j)| j.parent < 0 || (j.parent as usize) < i);
    if sorted {
        for (i, joint) in joints.iter().enumerate() {
            world[i] = if joint.parent >= 0 {
                mul_mat4(world[joint.parent as usize], local_transform(joint))
            } else {
                local_transform(joint)
            };
        }
    } else {
        resolve_world_transforms(joints, &mut world, local_transform);
    }
    world
}

/// Resolve world transforms for joints in arbitrary parent order, walking each
/// unresolved ancestor chain once (O(n)) and breaking cycles.
pub(crate) fn resolve_world_transforms(
    joints: &[Joint],
    world: &mut [[f32; 16]],
    local: impl Fn(&Joint) -> [f32; 16],
) {
    const UNVISITED: u8 = 0;
    const VISITING: u8 = 1;
    const DONE: u8 = 2;
    let count = joints.len();
    let parent_of = |i: usize| -> Option<usize> {
        let p = joints[i].parent;
        (p >= 0 && (p as usize) < count && p as usize != i).then_some(p as usize)
    };

    let mut state = vec![UNVISITED; count];
    let mut chain = Vec::new();
    for start in 0..count {
        if state[start] == DONE {
            continue;
        }
        // Climb to the first ancestor that is resolved, a root, or on the
        // current chain (a cycle).
        chain.clear();
        let mut j = start;
        loop {
            state[j] = VISITING;
            chain.push(j);
            match parent_of(j) {
                Some(p) if state[p] == UNVISITED => j = p,
                _ => break,
            }
        }
        // Resolve top-down.
        for &k in chain.iter().rev() {
            world[k] = match parent_of(k) {
                Some(p) if state[p] == DONE => mul_mat4(world[p], local(&joints[k])),
                _ => local(&joints[k]),
            };
            state[k] = DONE;
        }
    }
}

/// Joint matrices uniform buffer for the GPU.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct JointUniforms {
    pub joint_count: [f32; 4], // x = count, yzw padding
    pub joints: [[f32; 16]; MAX_JOINTS],
}

impl Default for JointUniforms {
    fn default() -> Self {
        Self {
            joint_count: [0.0; 4],
            joints: [IDENTITY_MAT4; MAX_JOINTS],
        }
    }
}

impl JointUniforms {
    /// Update from computed joint matrices.
    pub fn set_joints(&mut self, matrices: &[[f32; 16]]) {
        let count = matrices.len().min(MAX_JOINTS);
        self.joint_count[0] = count as f32;
        for (i, mat) in matrices.iter().take(count).enumerate() {
            self.joints[i] = *mat;
        }
    }
}

/// An animation clip — a set of channels that animate joint transforms over time.
#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub channels: Vec<AnimationChannel>,
}

/// A single animation channel targeting one joint's property.
#[derive(Debug, Clone)]
pub struct AnimationChannel {
    pub joint_index: usize,
    pub property: AnimationProperty,
    pub keyframes: Vec<Keyframe>,
}

/// What property the animation channel targets.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationProperty {
    Translation,
    Rotation,
    Scale,
}

/// A keyframe: time + value.
#[derive(Debug, Clone)]
pub struct Keyframe {
    pub time: f32,
    pub value: Vec<f32>, // 3 for translation/scale, 4 for rotation
}

impl AnimationClip {
    /// Sample the animation at a given time, applying to a skeleton.
    ///
    /// Time wraps into `[0, duration)` (negative times wrap backwards);
    /// non-finite times sample the first keyframe. Translation and scale are
    /// interpolated linearly; rotation uses shortest-path spherical linear
    /// interpolation, so joints never shear or shrink mid-blend.
    pub fn sample(&self, skeleton: &mut Skeleton, time: f32) {
        let t = if self.duration > 0.0 && time.is_finite() {
            time.rem_euclid(self.duration)
        } else {
            0.0
        };

        for channel in &self.channels {
            let Some(joint) = skeleton.joints.get_mut(channel.joint_index) else {
                continue;
            };

            match channel.property {
                AnimationProperty::Translation => {
                    if let [x, y, z, ..] = *interpolate_keyframes(&channel.keyframes, t) {
                        joint.translation = [x, y, z];
                    }
                }
                AnimationProperty::Rotation => {
                    if let [x, y, z, w, ..] = *interpolate_rotation(&channel.keyframes, t) {
                        joint.rotation = [x, y, z, w];
                    }
                }
                AnimationProperty::Scale => {
                    if let [x, y, z, ..] = *interpolate_keyframes(&channel.keyframes, t) {
                        joint.scale = [x, y, z];
                    }
                }
            }
        }
    }
}

/// Position of `time` within a keyframe track.
pub(crate) enum Bracket<'a> {
    /// Exactly one keyframe applies (before the first, after the last, or a
    /// single-key track).
    At(&'a [f32]),
    /// Between two keyframes at normalized parameter `t ∈ [0, 1]`.
    Between(&'a [f32], &'a [f32], f32),
}

/// Locate `time` in a time-sorted keyframe track (binary search).
/// Returns `None` for an empty track.
pub(crate) fn bracket(keyframes: &[Keyframe], time: f32) -> Option<Bracket<'_>> {
    let (first, last) = (keyframes.first()?, keyframes.last()?);
    if keyframes.len() == 1 || time.is_nan() || time <= first.time {
        return Some(Bracket::At(&first.value));
    }
    if time >= last.time {
        return Some(Bracket::At(&last.value));
    }
    // First keyframe strictly after `time`; clamped in case the track is not
    // sorted (partition_point stays in bounds but may then return 0 or len).
    let hi = keyframes
        .partition_point(|k| k.time <= time)
        .clamp(1, keyframes.len() - 1);
    let (a, b) = (&keyframes[hi - 1], &keyframes[hi]);
    let dt = b.time - a.time;
    let t = if dt.abs() > f32::EPSILON {
        ((time - a.time) / dt).clamp(0.0, 1.0)
    } else {
        0.0
    };
    Some(Bracket::Between(&a.value, &b.value, t))
}

/// Linear interpolation between keyframes.
///
/// Returns a `Cow::Borrowed` when an exact keyframe is returned (no interpolation),
/// avoiding allocation in the common boundary cases (before first, after last, single keyframe).
pub(crate) fn interpolate_keyframes(
    keyframes: &[Keyframe],
    time: f32,
) -> std::borrow::Cow<'_, [f32]> {
    use std::borrow::Cow;
    match bracket(keyframes, time) {
        None => Cow::Borrowed(&[]),
        Some(Bracket::At(v)) => Cow::Borrowed(v),
        Some(Bracket::Between(a, b, t)) => Cow::Owned(
            a.iter()
                .zip(b.iter())
                .map(|(&va, &vb)| va + (vb - va) * t)
                .collect(),
        ),
    }
}

/// Rotation interpolation: shortest-path slerp between unit quaternions (xyzw).
/// Keyframes with fewer than 4 components fall back to linear interpolation.
pub(crate) fn interpolate_rotation(
    keyframes: &[Keyframe],
    time: f32,
) -> std::borrow::Cow<'_, [f32]> {
    use std::borrow::Cow;
    match bracket(keyframes, time) {
        Some(Bracket::Between(a, b, t)) if a.len() >= 4 && b.len() >= 4 => {
            Cow::Owned(slerp([a[0], a[1], a[2], a[3]], [b[0], b[1], b[2], b[3]], t).to_vec())
        }
        _ => interpolate_keyframes(keyframes, time),
    }
}

/// Shortest-path spherical linear interpolation of quaternions (xyzw).
/// The result is normalized; a degenerate result falls back to identity.
#[must_use]
#[inline]
pub(crate) fn slerp(a: [f32; 4], mut b: [f32; 4], t: f32) -> [f32; 4] {
    let mut cos = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    // q and -q are the same rotation: take the shorter arc.
    if cos < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
        cos = -cos;
    }
    let (wa, wb) = if cos > 0.9995 {
        // Nearly parallel: normalized lerp is accurate and avoids 0/0.
        (1.0 - t, t)
    } else {
        let theta = cos.clamp(-1.0, 1.0).acos();
        let sin = theta.sin();
        (((1.0 - t) * theta).sin() / sin, (t * theta).sin() / sin)
    };
    let q = [
        wa * a[0] + wb * b[0],
        wa * a[1] + wb * b[1],
        wa * a[2] + wb * b[2],
        wa * a[3] + wb * b[3],
    ];
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if len.is_finite() && len > 1e-12 {
        [q[0] / len, q[1] / len, q[2] / len, q[3] / len]
    } else {
        [0.0, 0.0, 0.0, 1.0]
    }
}

/// Load animation clips from a glTF.
///
/// - Each skin becomes a [`Skeleton`]; joint parents are derived from the node
///   hierarchy (only joints of the same skin count as parents). Skins with
///   more than [`MAX_JOINTS`] joints are loaded fully but only the first
///   `MAX_JOINTS` are posed by [`Skeleton::compute_joint_matrices`].
/// - Animation channels are mapped onto the **first** skin's joints; channels
///   that target nodes outside that skin, or morph-target weights, are skipped.
/// - `LINEAR` and `STEP` samplers are reproduced exactly (`STEP` is encoded as
///   duplicate-time keyframes).
/// - **Known limitation of soorat 1.x:** `CUBICSPLINE` keeps each key's value
///   and drops its tangents, so playback is linear (slerp for rotations)
///   between keys instead of the authored cubic Hermite curve. Values at key
///   times are exact. [`Keyframe`] has no place for tangents without breaking
///   its public fields, so the fix lands in the Cyrius port (soorat 2.0; see
///   `docs/development/soorat-port-plan.md`, ADR 0002 #13).
pub fn load_gltf_animations(bytes: &[u8]) -> Result<(Vec<Skeleton>, Vec<AnimationClip>)> {
    let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| RenderError::Model(e.to_string()))?;

    let blob = gltf.blob.as_deref().unwrap_or(&[]);
    let buffer_sources = crate::gltf_loader::collect_buffer_sources(&gltf, blob);
    let node_count = gltf.nodes().len();

    let mut skeletons = Vec::new();
    let mut first_skin_joint_of_node: Option<Vec<Option<usize>>> = None;

    // Load skins → skeletons
    for skin in gltf.skins() {
        let reader = skin.reader(|buf| buffer_sources.get(buf.index()).copied());
        let inverse_binds: Vec<[[f32; 4]; 4]> = reader
            .read_inverse_bind_matrices()
            .map(|iter| iter.collect())
            .unwrap_or_default();

        let gltf_joints: Vec<gltf::Node<'_>> = skin.joints().collect();
        if gltf_joints.len() > MAX_JOINTS {
            tracing::warn!(
                joints = gltf_joints.len(),
                max = MAX_JOINTS,
                "glTF skin exceeds MAX_JOINTS — extra joints will not be posed"
            );
        }

        // Node index → joint index within this skin (node indices are
        // validated by gltf to be < node_count).
        let mut joint_of_node = vec![None; node_count];
        for (ji, node) in gltf_joints.iter().enumerate() {
            joint_of_node[node.index()] = Some(ji);
        }
        let mut parents = vec![-1i32; gltf_joints.len()];
        for (ji, node) in gltf_joints.iter().enumerate() {
            for child in node.children() {
                if let Some(cj) = joint_of_node[child.index()] {
                    parents[cj] = ji as i32;
                }
            }
        }

        let joints = gltf_joints
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let (translation, rotation, scale) = node.transform().decomposed();
                Joint {
                    parent: parents[i],
                    inverse_bind: inverse_binds
                        .get(i)
                        .map_or(IDENTITY_MAT4, |m| flatten_mat4(*m)),
                    translation,
                    rotation,
                    scale,
                }
            })
            .collect();

        skeletons.push(Skeleton { joints });
        if first_skin_joint_of_node.is_none() {
            first_skin_joint_of_node = Some(joint_of_node);
        }
    }

    // Load animations
    let joint_of_node = first_skin_joint_of_node.unwrap_or_default();
    let mut clips = Vec::new();
    for anim in gltf.animations() {
        let mut channels = Vec::new();
        let mut duration = 0.0f32;

        for channel in anim.channels() {
            let target = channel.target();
            let Some(joint_index) = joint_of_node.get(target.node().index()).copied().flatten()
            else {
                tracing::debug!(
                    node = target.node().index(),
                    "animation channel targets a non-joint node — skipped"
                );
                continue;
            };

            let property = match target.property() {
                gltf::animation::Property::Translation => AnimationProperty::Translation,
                gltf::animation::Property::Rotation => AnimationProperty::Rotation,
                gltf::animation::Property::Scale => AnimationProperty::Scale,
                _ => continue,
            };

            let reader = channel.reader(|buf| buffer_sources.get(buf.index()).copied());
            let times: Vec<f32> = reader
                .read_inputs()
                .map(|iter| iter.collect())
                .unwrap_or_default();
            let values: Vec<Vec<f32>> = match reader.read_outputs() {
                Some(gltf::animation::util::ReadOutputs::Translations(iter)) => {
                    iter.map(|v| v.to_vec()).collect()
                }
                Some(gltf::animation::util::ReadOutputs::Rotations(iter)) => {
                    iter.into_f32().map(|v| v.to_vec()).collect()
                }
                Some(gltf::animation::util::ReadOutputs::Scales(iter)) => {
                    iter.map(|v| v.to_vec()).collect()
                }
                _ => continue,
            };

            if let Some(&last_time) = times.last()
                && last_time.is_finite()
            {
                duration = duration.max(last_time);
            }

            let keyframes = build_keyframes(times, values, channel.sampler().interpolation());
            channels.push(AnimationChannel {
                joint_index,
                property,
                keyframes,
            });
        }

        clips.push(AnimationClip {
            name: anim.name().unwrap_or("unnamed").to_string(),
            duration,
            channels,
        });
    }

    tracing::debug!(
        skeletons = skeletons.len(),
        clips = clips.len(),
        "loaded gltf animations"
    );
    Ok((skeletons, clips))
}

/// Pair sampler inputs with outputs according to the glTF interpolation mode.
pub(crate) fn build_keyframes(
    times: Vec<f32>,
    values: Vec<Vec<f32>>,
    interpolation: gltf::animation::Interpolation,
) -> Vec<Keyframe> {
    use gltf::animation::Interpolation;
    match interpolation {
        Interpolation::Linear => times
            .into_iter()
            .zip(values)
            .map(|(time, value)| Keyframe { time, value })
            .collect(),
        // Outputs are (in-tangent, value, out-tangent) triplets: keep values.
        // Tangents are dropped (a 1.x limitation, see `load_gltf_animations`).
        Interpolation::CubicSpline => times
            .into_iter()
            .zip(values.into_iter().skip(1).step_by(3))
            .map(|(time, value)| Keyframe { time, value })
            .collect(),
        // Hold each value until the next key: add a duplicate-time key carrying
        // the previous value, so linear sampling reproduces the step exactly.
        Interpolation::Step => {
            let n = times.len().min(values.len());
            let mut out = Vec::with_capacity(n.saturating_mul(2));
            for k in 0..n {
                if k > 0 {
                    out.push(Keyframe {
                        time: times[k],
                        value: values[k - 1].clone(),
                    });
                }
                out.push(Keyframe {
                    time: times[k],
                    value: values[k].clone(),
                });
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quat_len(q: [f32; 4]) -> f32 {
        (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt()
    }

    #[test]
    fn slerp_halfway_is_half_angle_and_unit() {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let q = slerp([0.0, 0.0, 0.0, 1.0], [0.0, 0.0, h, h], 0.5);
        // 45° about Z: (0, 0, sin 22.5°, cos 22.5°)
        let half = std::f32::consts::FRAC_PI_8;
        assert!((q[2] - half.sin()).abs() < 1e-5, "{q:?}");
        assert!((q[3] - half.cos()).abs() < 1e-5, "{q:?}");
        assert!((quat_len(q) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn slerp_takes_shortest_path() {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        // -q is the same rotation; the blend must not swing the long way.
        let a = slerp([0.0, 0.0, 0.0, 1.0], [0.0, 0.0, -h, -h], 0.5);
        let b = slerp([0.0, 0.0, 0.0, 1.0], [0.0, 0.0, h, h], 0.5);
        for k in 0..4 {
            assert!((a[k] - b[k]).abs() < 1e-5, "{a:?} vs {b:?}");
        }
        // Endpoints are reproduced.
        let e = slerp([0.0, 0.0, 0.0, 1.0], [0.0, 0.0, h, h], 1.0);
        assert!((e[2] - h).abs() < 1e-5 && (e[3] - h).abs() < 1e-5);
    }

    #[test]
    fn sample_rotation_stays_unit_length() {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let clip = AnimationClip {
            name: "spin".into(),
            duration: 1.0,
            channels: vec![AnimationChannel {
                joint_index: 0,
                property: AnimationProperty::Rotation,
                keyframes: vec![
                    Keyframe {
                        time: 0.0,
                        value: vec![0.0, 0.0, 0.0, 1.0],
                    },
                    Keyframe {
                        time: 1.0,
                        value: vec![h, 0.0, 0.0, h],
                    },
                ],
            }],
        };
        let mut skel = Skeleton {
            joints: vec![Joint::default()],
        };
        for i in 0..=10 {
            clip.sample(&mut skel, i as f32 * 0.099);
            let len = quat_len(skel.joints[0].rotation);
            assert!(
                (len - 1.0).abs() < 1e-5,
                "non-unit rotation {len} at step {i}"
            );
        }
    }

    #[test]
    fn sample_wraps_negative_and_non_finite_time() {
        let clip = AnimationClip {
            name: "move".into(),
            duration: 2.0,
            channels: vec![AnimationChannel {
                joint_index: 0,
                property: AnimationProperty::Translation,
                keyframes: vec![
                    Keyframe {
                        time: 0.0,
                        value: vec![0.0, 0.0, 0.0],
                    },
                    Keyframe {
                        time: 2.0,
                        value: vec![2.0, 0.0, 0.0],
                    },
                ],
            }],
        };
        let mut skel = Skeleton {
            joints: vec![Joint::default()],
        };
        clip.sample(&mut skel, -0.5); // wraps to 1.5
        assert!((skel.joints[0].translation[0] - 1.5).abs() < 1e-5);
        clip.sample(&mut skel, f32::NAN);
        assert_eq!(skel.joints[0].translation, [0.0, 0.0, 0.0]);
        // Out-of-range joint index is ignored, not a panic.
        let mut empty = Skeleton { joints: vec![] };
        clip.sample(&mut empty, 1.0);
    }

    fn joint(parent: i32, translation: [f32; 3]) -> Joint {
        Joint {
            parent,
            translation,
            ..Joint::default()
        }
    }

    #[test]
    fn joint_matrices_handle_unsorted_parents() {
        // Sorted: 0 root, 1 child of 0, 2 child of 1.
        let sorted = Skeleton {
            joints: vec![
                joint(-1, [1.0, 0.0, 0.0]),
                joint(0, [0.0, 2.0, 0.0]),
                joint(1, [0.0, 0.0, 3.0]),
            ],
        };
        // Same hierarchy listed leaf-first: 0 = leaf (parent 1), 1 = mid (parent 2), 2 = root.
        let unsorted = Skeleton {
            joints: vec![
                joint(1, [0.0, 0.0, 3.0]),
                joint(2, [0.0, 2.0, 0.0]),
                joint(-1, [1.0, 0.0, 0.0]),
            ],
        };
        let a = sorted.compute_joint_matrices();
        let b = unsorted.compute_joint_matrices();
        assert_eq!(a[2], b[0], "leaf world transform differs");
        assert_eq!(a[1], b[1]);
        assert_eq!(a[0], b[2]);
        assert_eq!(&a[2][12..15], &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn joint_matrices_break_cycles() {
        let skel = Skeleton {
            joints: vec![
                joint(1, [1.0, 0.0, 0.0]),
                joint(0, [0.0, 1.0, 0.0]),
                joint(2, [0.0, 0.0, 1.0]),  // self-parent
                joint(99, [0.0, 0.0, 0.0]), // out of range
            ],
        };
        let m = skel.compute_joint_matrices();
        assert_eq!(m.len(), 4);
        assert!(m.iter().flatten().all(|v| v.is_finite()));
        assert_eq!(&m[2][12..15], &[0.0, 0.0, 1.0]); // treated as root
    }

    #[test]
    fn build_keyframes_step_and_cubic() {
        use gltf::animation::Interpolation;
        let step = build_keyframes(
            vec![0.0, 1.0, 2.0],
            vec![vec![0.0], vec![10.0], vec![20.0]],
            Interpolation::Step,
        );
        let at = |t: f32| interpolate_keyframes(&step, t)[0];
        assert_eq!(at(0.5), 0.0);
        assert_eq!(at(0.999), 0.0);
        assert_eq!(at(1.0), 10.0);
        assert_eq!(at(1.5), 10.0);
        assert_eq!(at(2.5), 20.0);

        let cubic = build_keyframes(
            vec![0.0, 1.0],
            vec![
                vec![9.0],
                vec![1.0],
                vec![9.0],
                vec![9.0],
                vec![3.0],
                vec![9.0],
            ],
            Interpolation::CubicSpline,
        );
        assert_eq!(cubic.len(), 2);
        assert_eq!(cubic[0].value, vec![1.0]);
        assert_eq!(cubic[1].value, vec![3.0]);
    }

    #[test]
    fn load_fixture_animations() {
        let bytes = crate::test_support::skinned_animation_glb();
        let (skeletons, clips) = load_gltf_animations(&bytes).expect("fixture should parse");

        assert_eq!(skeletons.len(), 1);
        let joints = &skeletons[0].joints;
        assert_eq!(joints.len(), 2);
        assert_eq!(
            joints[0].parent, 1,
            "child joint (node 1) parented to node 0"
        );
        assert_eq!(joints[1].parent, -1);

        assert_eq!(clips.len(), 1);
        let clip = &clips[0];
        assert_eq!(clip.name, "fixture");
        assert_eq!(clip.duration, 1.0);
        // Channel targeting the non-joint node 2 is skipped.
        assert_eq!(clip.channels.len(), 2);

        let rot = &clip.channels[0];
        assert_eq!(rot.property, AnimationProperty::Rotation);
        assert_eq!(rot.joint_index, 1);
        assert_eq!(rot.keyframes.len(), 3, "STEP encodes a duplicate-time key");

        let tr = &clip.channels[1];
        assert_eq!(tr.property, AnimationProperty::Translation);
        assert_eq!(tr.joint_index, 0);
        assert_eq!(tr.keyframes[0].value, vec![1.0, 0.0, 0.0]);
        assert_eq!(tr.keyframes[1].value, vec![2.0, 0.0, 0.0]);

        let mut skel = skeletons[0].clone();
        clip.sample(&mut skel, 0.5);
        assert_eq!(
            skel.joints[1].rotation,
            [0.0, 0.0, 0.0, 1.0],
            "STEP holds key 0"
        );
        assert!((skel.joints[0].translation[0] - 1.5).abs() < 1e-5);
        assert!(
            skel.compute_joint_matrices()
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
    }

    #[test]
    fn joint_default() {
        let j = Joint::default();
        assert_eq!(j.parent, -1);
        assert_eq!(j.rotation, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(j.scale, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn skeleton_identity_pose() {
        let skeleton = Skeleton {
            joints: vec![Joint::default()],
        };
        let matrices = skeleton.compute_joint_matrices();
        assert_eq!(matrices.len(), 1);
        assert_eq!(matrices[0], IDENTITY_MAT4);
    }

    #[test]
    fn joint_uniforms_size() {
        // 4 floats (count) + 128 * 16 floats (joints) = 4 + 2048 = 2052 floats = 8208 bytes
        let expected = 16 + MAX_JOINTS * 64; // 16 for count vec4 + 128*64 for matrices
        assert_eq!(std::mem::size_of::<JointUniforms>(), expected);
    }

    #[test]
    fn joint_uniforms_set_joints() {
        let mut u = JointUniforms::default();
        let matrices = vec![IDENTITY_MAT4, IDENTITY_MAT4, IDENTITY_MAT4];
        u.set_joints(&matrices);
        assert_eq!(u.joint_count[0], 3.0);
    }

    #[test]
    fn compose_trs_identity() {
        let m = compose_trs([0.0; 3], [0.0, 0.0, 0.0, 1.0], [1.0; 3]);
        for i in 0..16 {
            assert!(
                (m[i] - IDENTITY_MAT4[i]).abs() < 0.001,
                "mismatch at {i}: {} vs {}",
                m[i],
                IDENTITY_MAT4[i]
            );
        }
    }

    #[test]
    fn compose_trs_translation() {
        let m = compose_trs([5.0, 3.0, 1.0], [0.0, 0.0, 0.0, 1.0], [1.0; 3]);
        assert_eq!(m[12], 5.0);
        assert_eq!(m[13], 3.0);
        assert_eq!(m[14], 1.0);
    }

    #[test]
    fn interpolate_single_keyframe() {
        let kf = vec![Keyframe {
            time: 0.0,
            value: vec![1.0, 2.0, 3.0],
        }];
        let v = interpolate_keyframes(&kf, 0.5);
        assert_eq!(v, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn interpolate_two_keyframes() {
        let kf = vec![
            Keyframe {
                time: 0.0,
                value: vec![0.0],
            },
            Keyframe {
                time: 1.0,
                value: vec![10.0],
            },
        ];
        let v = interpolate_keyframes(&kf, 0.5);
        assert!((v[0] - 5.0).abs() < 0.001);
    }

    #[test]
    fn interpolate_clamps_to_bounds() {
        let kf = vec![
            Keyframe {
                time: 0.0,
                value: vec![0.0],
            },
            Keyframe {
                time: 1.0,
                value: vec![10.0],
            },
        ];
        assert_eq!(interpolate_keyframes(&kf, -1.0), vec![0.0]);
        assert_eq!(interpolate_keyframes(&kf, 5.0), vec![10.0]);
    }

    #[test]
    fn load_invalid_gltf_animations() {
        let result = load_gltf_animations(b"not gltf");
        assert!(result.is_err());
    }

    #[test]
    fn interpolate_keyframes_empty() {
        let kf: Vec<Keyframe> = vec![];
        let v = interpolate_keyframes(&kf, 0.5);
        assert!(v.is_empty(), "empty keyframes should return empty result");
    }

    #[test]
    fn skeleton_single_joint() {
        let skeleton = Skeleton {
            joints: vec![Joint {
                parent: -1,
                inverse_bind: IDENTITY_MAT4,
                translation: [1.0, 2.0, 3.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            }],
        };
        let matrices = skeleton.compute_joint_matrices();
        assert_eq!(matrices.len(), 1);
        // With identity inverse bind, joint matrix = world transform
        // Translation should appear in column 3
        assert_eq!(matrices[0][12], 1.0);
        assert_eq!(matrices[0][13], 2.0);
        assert_eq!(matrices[0][14], 3.0);
    }

    #[test]
    fn animation_property_values() {
        // Verify all enum variants exist and are distinct
        let t = AnimationProperty::Translation;
        let r = AnimationProperty::Rotation;
        let s = AnimationProperty::Scale;
        assert_ne!(t, r);
        assert_ne!(r, s);
        assert_ne!(t, s);
        assert_eq!(t, AnimationProperty::Translation);
    }
}
