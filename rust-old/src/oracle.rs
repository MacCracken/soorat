//! Oracle hooks for the Cyrius port's golden-value generator.
//!
//! **Not public API.** This module exists only behind the non-default `oracle`
//! feature, is hidden from the docs, and carries no stability promise. It lets
//! the generator in `scripts/golden/` (see `docs/development/soorat-port-plan.md`,
//! D1 and M0 step 0) call the crate-private helpers behind soorat's public API:
//! - matrix and vector helpers (`math_util`);
//! - visualization: grid checks, normalization, colour maps, and the heatmap,
//!   height-field and lattice meshes (`viz`);
//! - animation: keyframe bracketing and interpolation, slerp, glTF sampler
//!   pairing, local and world joint transforms;
//! - environment maps: the sRGB transfer functions and decode table, f16
//!   conversion, face decode/encode, the box downsample and the mip chain;
//! - shadows: the atlas tile math;
//! - meshes: the normal matrix.
//!
//! Every hook is a pure function of its arguments: inputs are taken by value
//! or by shared reference, and nothing here reads or writes global state. The
//! hooks call the same functions the library uses, so enabling the feature
//! changes no rendering behaviour. `tests/oracle.rs` checks both.

use crate::animation::{self, Bracket, Joint, Keyframe};
use crate::environment::{self, FaceEncoding};

// ── Crate-private modules, re-exported as they are ─────────────────────────

pub use crate::math_util::{
    IDENTITY_MAT4, compose_trs, cross, flatten_mat4, look_at, mul_mat4, normal_to_basis,
    normalize3, perspective_90,
};
pub use crate::viz::{
    HeightField, MAX_QUADS, MAX_SEGMENTS, XZ_QUAD_INDICES, XzGrid, finite_min_max, grid_is_backed,
    grid_len, heat_rgba, heightfield_mesh, lattice_indices, magnitude_t, signed_rgba, signed_t,
    unit_t, with_alpha, xz_heatmap_mesh,
};

// ── Animation ──────────────────────────────────────────────────────────────

/// Where a time falls in a keyframe track: an owned copy of the crate's
/// keyframe bracket.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum BracketDump {
    /// One keyframe's value applies (before the first key, after the last, a
    /// single-key track, or a NaN time).
    At(Vec<f32>),
    /// Between two keyframes' values, at the normalized parameter `t ∈ [0, 1]`.
    Between(Vec<f32>, Vec<f32>, f32),
}

/// Locate `time` in a time-sorted keyframe track. `None` for an empty track.
#[must_use]
pub fn bracket(keyframes: &[Keyframe], time: f32) -> Option<BracketDump> {
    animation::bracket(keyframes, time).map(|b| match b {
        Bracket::At(v) => BracketDump::At(v.to_vec()),
        Bracket::Between(a, b, t) => BracketDump::Between(a.to_vec(), b.to_vec(), t),
    })
}

/// Component-wise linear interpolation of a keyframe track at `time`.
#[must_use]
pub fn interpolate_keyframes(keyframes: &[Keyframe], time: f32) -> Vec<f32> {
    animation::interpolate_keyframes(keyframes, time).into_owned()
}

/// Rotation track sampling: shortest-path slerp between quaternion keys
/// (linear for keys with fewer than 4 components).
#[must_use]
pub fn interpolate_rotation(keyframes: &[Keyframe], time: f32) -> Vec<f32> {
    animation::interpolate_rotation(keyframes, time).into_owned()
}

/// Shortest-path quaternion slerp (xyzw), normalized; identity if degenerate.
#[must_use]
pub fn slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    animation::slerp(a, b, t)
}

/// A glTF sampler's interpolation mode, without the `gltf` crate's type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SamplerMode {
    /// `LINEAR`.
    Linear,
    /// `STEP`.
    Step,
    /// `CUBICSPLINE` (outputs are in-tangent, value, out-tangent triplets).
    CubicSpline,
}

/// Pair a sampler's input times with its outputs, as the glTF loader does.
#[must_use]
pub fn build_keyframes(times: Vec<f32>, values: Vec<Vec<f32>>, mode: SamplerMode) -> Vec<Keyframe> {
    use gltf::animation::Interpolation;
    let interpolation = match mode {
        SamplerMode::Linear => Interpolation::Linear,
        SamplerMode::Step => Interpolation::Step,
        SamplerMode::CubicSpline => Interpolation::CubicSpline,
    };
    animation::build_keyframes(times, values, interpolation)
}

/// A joint's local transform (T·R·S, column-major).
#[must_use]
pub fn local_transform(joint: &Joint) -> [f32; 16] {
    animation::local_transform(joint)
}

/// World transforms of `joints` before the inverse bind, by the library's own
/// path: one forward pass when parents precede children, the general walk
/// otherwise. Joints past [`animation::MAX_JOINTS`] are ignored, as the
/// library ignores them.
#[must_use]
pub fn world_transforms(joints: &[Joint]) -> Vec<[f32; 16]> {
    let count = joints.len().min(animation::MAX_JOINTS);
    animation::world_transforms(&joints[..count])
}

/// World transforms by the general walk alone (any parent order; cycles,
/// self-parents and out-of-range parents become roots), even for joints whose
/// parents precede them.
#[must_use]
pub fn world_transforms_resolved(joints: &[Joint]) -> Vec<[f32; 16]> {
    let count = joints.len().min(animation::MAX_JOINTS);
    let mut world = vec![IDENTITY_MAT4; count];
    animation::resolve_world_transforms(&joints[..count], &mut world, animation::local_transform);
    world
}

// ── Environment maps ───────────────────────────────────────────────────────

/// An environment-map face encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FaceFormat {
    /// `Rgba8UnormSrgb`: 4 bytes per texel, sRGB-encoded.
    Srgb8,
    /// `Rgba16Float`: 8 bytes per texel, linear half floats.
    F16,
}

fn encoding(format: FaceFormat) -> FaceEncoding {
    match format {
        FaceFormat::Srgb8 => FaceEncoding::Srgb8,
        FaceFormat::F16 => FaceEncoding::F16,
    }
}

/// sRGB decode (IEC 61966-2-1), f32.
#[must_use]
pub fn srgb_to_linear(c: f32) -> f32 {
    environment::srgb_to_linear(c)
}

/// sRGB encode (IEC 61966-2-1), f32.
#[must_use]
pub fn linear_to_srgb(l: f32) -> f32 {
    environment::linear_to_srgb(l)
}

/// The linear value of each sRGB-encoded byte, as `Srgb8` faces decode.
#[must_use]
pub fn srgb8_decode_table() -> [f32; 256] {
    environment::srgb8_decode_table()
}

/// IEEE 754 binary16 → binary32.
#[must_use]
pub fn f16_to_f32(h: u16) -> f32 {
    environment::f16_to_f32(h)
}

/// IEEE 754 binary32 → binary16, round to nearest even; overflow → ±inf.
#[must_use]
pub fn f32_to_f16(v: f32) -> u16 {
    environment::f32_to_f16(v)
}

/// Decode a face into linear RGBA texels (a trailing partial texel is ignored).
#[must_use]
pub fn decode_face(format: FaceFormat, bytes: &[u8]) -> Vec<[f32; 4]> {
    encoding(format).decode(bytes)
}

/// Encode linear RGBA texels into a face.
#[must_use]
pub fn encode_face(format: FaceFormat, texels: &[[f32; 4]]) -> Vec<u8> {
    encoding(format).encode(texels)
}

/// 2×2 box downsample of a `size × size` linear image (empty for `size == 0`
/// or a short input).
#[must_use]
pub fn downsample(src: &[[f32; 4]], size: usize) -> Vec<[f32; 4]> {
    environment::downsample(src, size)
}

/// The CPU mip chain one face uploads for a `size × size` map: levels
/// `1..mip_level_count(size, size)`, each as `(edge size, encoded bytes)`.
#[must_use]
pub fn face_mip_chain(format: FaceFormat, face: &[u8], size: u32) -> Vec<(usize, Vec<u8>)> {
    let mip_count = crate::texture::mip_level_count(size, size);
    environment::face_mip_chain(encoding(format), face, size as usize, mip_count)
}

// ── Shadows ────────────────────────────────────────────────────────────────

/// Tiles per row of a `size`-texel atlas with `tile_size` tiles (at least 1).
#[must_use]
pub fn atlas_columns(size: u32, tile_size: u32) -> u32 {
    crate::shadow::atlas_columns(size, tile_size)
}

/// Pixel viewport `(x, y, w, h)` of tile `index` in an atlas with `columns`
/// tiles per row.
#[must_use]
pub fn tile_viewport_in(columns: u32, tile_size: u32, index: u32) -> (u32, u32, u32, u32) {
    crate::shadow::tile_viewport_in(columns, tile_size, index)
}

/// UV rectangle `(u, v, w, h)` of tile `index` in a `size`-texel atlas.
#[must_use]
pub fn tile_uv_in(columns: u32, tile_size: u32, size: u32, index: u32) -> [f32; 4] {
    crate::shadow::tile_uv_in(columns, tile_size, size, index)
}

// ── Meshes ─────────────────────────────────────────────────────────────────

/// The rows of `M⁻ᵀ` for the upper-left 3×3 of a column-major model matrix
/// (the cofactor matrix when `|det| ≤ 1e-10`), as the mesh pipeline uploads it.
#[must_use]
pub fn inverse_transpose_3x3(m: &[f32; 16]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    crate::mesh_pipeline::inverse_transpose_3x3(m)
}
