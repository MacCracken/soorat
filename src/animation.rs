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
    pub fn compute_joint_matrices(&self) -> Vec<[f32; 16]> {
        let count = self.joints.len().min(MAX_JOINTS);
        let mut world_transforms = vec![IDENTITY_MAT4; count];
        let mut joint_matrices = Vec::with_capacity(count);

        // Forward pass: compute world transforms from local transforms
        for i in 0..count {
            let joint = &self.joints[i];
            let local = compose_trs(joint.translation, joint.rotation, joint.scale);

            if joint.parent >= 0 && (joint.parent as usize) < i {
                world_transforms[i] = mul_mat4(world_transforms[joint.parent as usize], local);
            } else {
                world_transforms[i] = local;
            }
        }

        // Joint matrix = world_transform * inverse_bind
        for (wt, joint) in world_transforms.iter().zip(self.joints.iter()).take(count) {
            joint_matrices.push(mul_mat4(*wt, joint.inverse_bind));
        }

        joint_matrices
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
    pub fn sample(&self, skeleton: &mut Skeleton, time: f32) {
        let t = if self.duration > 0.0 {
            time % self.duration
        } else {
            0.0
        };

        for channel in &self.channels {
            if channel.joint_index >= skeleton.joints.len() {
                continue;
            }

            let value = interpolate_keyframes(&channel.keyframes, t);
            let joint = &mut skeleton.joints[channel.joint_index];

            match channel.property {
                AnimationProperty::Translation => {
                    if value.len() >= 3 {
                        joint.translation = [value[0], value[1], value[2]];
                    }
                }
                AnimationProperty::Rotation => {
                    if value.len() >= 4 {
                        joint.rotation = [value[0], value[1], value[2], value[3]];
                    }
                }
                AnimationProperty::Scale => {
                    if value.len() >= 3 {
                        joint.scale = [value[0], value[1], value[2]];
                    }
                }
            }
        }
    }
}

/// Linear interpolation between keyframes.
fn interpolate_keyframes(keyframes: &[Keyframe], time: f32) -> Vec<f32> {
    if keyframes.is_empty() {
        return Vec::new();
    }
    if keyframes.len() == 1 || time <= keyframes[0].time {
        return keyframes[0].value.clone();
    }
    if time >= keyframes.last().unwrap().time {
        return keyframes.last().unwrap().value.clone();
    }

    // Find the two surrounding keyframes
    let mut i = 0;
    while i < keyframes.len() - 1 && keyframes[i + 1].time < time {
        i += 1;
    }

    let a = &keyframes[i];
    let b = &keyframes[i + 1];
    let t = (time - a.time) / (b.time - a.time);

    // Lerp each component
    a.value
        .iter()
        .zip(b.value.iter())
        .map(|(&va, &vb)| va + (vb - va) * t)
        .collect()
}

/// Load animation clips from a glTF.
pub fn load_gltf_animations(bytes: &[u8]) -> Result<(Vec<Skeleton>, Vec<AnimationClip>)> {
    let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| RenderError::Model(e.to_string()))?;

    let blob = gltf.blob.as_deref().unwrap_or(&[]);
    let buffer_sources: Vec<&[u8]> = gltf
        .buffers()
        .map(|buffer| match buffer.source() {
            gltf::buffer::Source::Bin => blob,
            gltf::buffer::Source::Uri(_) => &[] as &[u8],
        })
        .collect();

    let mut skeletons = Vec::new();
    let mut clips = Vec::new();

    // Load skins → skeletons
    for skin in gltf.skins() {
        let reader = skin.reader(|buf| buffer_sources.get(buf.index()).copied());

        let inverse_binds: Vec<[[f32; 4]; 4]> = reader
            .read_inverse_bind_matrices()
            .map(|iter| iter.collect())
            .unwrap_or_default();

        let gltf_joints = skin.joints().collect::<Vec<_>>();
        let mut joints = Vec::with_capacity(gltf_joints.len());

        for (i, gltf_joint) in gltf_joints.iter().enumerate() {
            let (t, r, s) = gltf_joint.transform().decomposed();
            let ibm = if i < inverse_binds.len() {
                flatten_mat4(inverse_binds[i])
            } else {
                IDENTITY_MAT4
            };

            // Find parent index within the joints list
            let parent = gltf_joints
                .iter()
                .position(|j| {
                    gltf_joint
                        .children()
                        .any(|_| false) // check if this joint's parent is another joint
                        || j.index() == gltf_joint.index()
                })
                .and_then(|_| {
                    // Search for this joint's parent in the joint list
                    gltf_joints.iter().position(|candidate| {
                        candidate
                            .children()
                            .any(|child| child.index() == gltf_joint.index())
                    })
                })
                .map(|idx| idx as i32)
                .unwrap_or(-1);

            joints.push(Joint {
                parent,
                inverse_bind: ibm,
                translation: t,
                rotation: r,
                scale: s,
            });
        }

        skeletons.push(Skeleton { joints });
    }

    // Load animations
    for anim in gltf.animations() {
        let mut channels = Vec::new();
        let mut duration = 0.0f32;

        for channel in anim.channels() {
            let reader = channel.reader(|buf| buffer_sources.get(buf.index()).copied());
            let target = channel.target();

            // Find joint index
            let joint_index = gltf
                .skins()
                .next()
                .and_then(|skin| {
                    skin.joints()
                        .position(|j| j.index() == target.node().index())
                })
                .unwrap_or(0);

            let property = match target.property() {
                gltf::animation::Property::Translation => AnimationProperty::Translation,
                gltf::animation::Property::Rotation => AnimationProperty::Rotation,
                gltf::animation::Property::Scale => AnimationProperty::Scale,
                _ => continue,
            };

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

            if let Some(&last_time) = times.last() {
                duration = duration.max(last_time);
            }

            let keyframes: Vec<Keyframe> = times
                .into_iter()
                .zip(values)
                .map(|(time, value)| Keyframe { time, value })
                .collect();

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

    Ok((skeletons, clips))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
