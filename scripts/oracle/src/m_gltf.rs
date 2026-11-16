//! `gltf` (M3 part c): soorat 1.2.0's `load_gltf_meshes` and `load_gltf_animations` on GLB and glTF fixtures.
//!
//! The fixtures are written as `tests/golden/gltf/<name>` and a row names its fixture (`s:<name>`). There are two
//! rows per fixture, one per loader, both class A:
//! - `load_gltf_meshes`: `i:meshes`; per mesh its vertices (`v:12n`: position, normal, UV, colour), its indices
//!   (`v:m`) and its material (`i:` or `n:`); then `i:images` and each image's bytes (`v:len`);
//! - `load_gltf_animations`: `i:skeletons`; per skeleton `i:joints`, and per joint `i:parent` then its inverse bind,
//!   translation, rotation and scale (`v:16 v:3 v:4 v:3`); then `i:clips`; per clip `s:name g:duration i:channels`;
//!   per channel `i:joint i:property i:keys`, and per key `g:time` and its value (`v:n`).
//!
//! An error is `E:Model`. A panic in Rust 1.2.0 is `s:panic`: no verdict to copy (ADR 0002 #14). This crate builds
//! with overflow checks, so the two inputs that panic only under them (a GLB header length below 12, an accessor
//! count of 0) are panics here too.
//!
//! The probe P9 fixtures (`scripts/probes/rust/src/bin/p9_glb.rs`, M0) are reused verbatim, `test_support`'s two
//! builders among them. The rest are variants for the schema checks, sparse accessors, matrix joints and the
//! reader's rules (port-parity §4.8).

use crate::{Module, Toks};
use soorat::animation::{AnimationProperty, load_gltf_animations};
use soorat::gltf_loader::load_gltf_meshes;
use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn f32_bytes(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn u16_bytes(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn glb(json: &str, bin: &[u8]) -> Vec<u8> {
    let mut json_chunk = json.as_bytes().to_vec();
    while !json_chunk.len().is_multiple_of(4) {
        json_chunk.push(b' ');
    }
    let mut bin_chunk = bin.to_vec();
    while !bin_chunk.len().is_multiple_of(4) {
        bin_chunk.push(0);
    }
    let total = 12 + 8 + json_chunk.len() + 8 + bin_chunk.len();

    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_chunk.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json_chunk);
    out.extend_from_slice(&(bin_chunk.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin_chunk);
    out
}

fn skinned_animation_glb() -> Vec<u8> {
    let h = std::f32::consts::FRAC_1_SQRT_2;
    let mut bin = f32_bytes(&[0.0, 1.0]); // times @0 (8 bytes)
    bin.extend(f32_bytes(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, h, h])); // rotations @8 (32)
    bin.extend(f32_bytes(&[5.0, 5.0, 5.0, 6.0, 6.0, 6.0])); // node2 translations @40 (24)
    bin.extend(f32_bytes(&[
        9.0, 9.0, 9.0, 1.0, 0.0, 0.0, 9.0, 9.0, 9.0, // key 0: in, value, out
        9.0, 9.0, 9.0, 2.0, 0.0, 0.0, 9.0, 9.0, 9.0, // key 1: in, value, out
    ])); // cubic @64 (72)
    let json = format!(
        r#"{{
  "asset": {{"version": "2.0"}},
  "buffers": [{{"byteLength": {len}}}],
  "bufferViews": [
    {{"buffer": 0, "byteOffset": 0, "byteLength": 8}},
    {{"buffer": 0, "byteOffset": 8, "byteLength": 32}},
    {{"buffer": 0, "byteOffset": 40, "byteLength": 24}},
    {{"buffer": 0, "byteOffset": 64, "byteLength": 72}}
  ],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 2, "type": "SCALAR", "min": [0.0], "max": [1.0]}},
    {{"bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC4"}},
    {{"bufferView": 2, "componentType": 5126, "count": 2, "type": "VEC3"}},
    {{"bufferView": 3, "componentType": 5126, "count": 6, "type": "VEC3"}}
  ],
  "nodes": [{{"children": [1]}}, {{}}, {{}}],
  "skins": [{{"joints": [1, 0]}}],
  "animations": [{{
    "name": "fixture",
    "samplers": [
      {{"input": 0, "output": 2, "interpolation": "LINEAR"}},
      {{"input": 0, "output": 1, "interpolation": "STEP"}},
      {{"input": 0, "output": 3, "interpolation": "CUBICSPLINE"}}
    ],
    "channels": [
      {{"sampler": 0, "target": {{"node": 2, "path": "translation"}}}},
      {{"sampler": 1, "target": {{"node": 0, "path": "rotation"}}}},
      {{"sampler": 2, "target": {{"node": 1, "path": "translation"}}}}
    ]
  }}]
}}"#,
        len = bin.len()
    );
    glb(&json, &bin)
}

fn mesh_edge_cases_bin() -> Vec<u8> {
    let mut bin = f32_bytes(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]); // positions @0 (36)
    bin.extend(u16_bytes(&[0, 2, 1])); // good indices @36 (6)
    bin.extend(u16_bytes(&[0])); // pad to 4-byte alignment @42 (2) — unused
    bin.extend(u16_bytes(&[0, 1, 5])); // bad indices @44 (6)
    bin
}

fn mesh_edge_cases_json(len: usize) -> String {
    format!(
        r#"{{
  "asset": {{"version": "2.0"}},
  "buffers": [{{"byteLength": {len}}}],
  "bufferViews": [
    {{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
    {{"buffer": 0, "byteOffset": 36, "byteLength": 6}},
    {{"buffer": 0, "byteOffset": 44, "byteLength": 6}},
    {{"buffer": 0, "byteOffset": 4000, "byteLength": 64}}
  ],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0.0, 0.0, 0.0], "max": [1.0, 0.0, 1.0]}},
    {{"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"}},
    {{"bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR"}}
  ],
  "images": [{{"bufferView": 3, "mimeType": "image/png"}}],
  "meshes": [{{"primitives": [
    {{"attributes": {{"POSITION": 0}}, "indices": 1}},
    {{"attributes": {{"POSITION": 0}}, "indices": 2}},
    {{"attributes": {{"POSITION": 0}}, "indices": 1, "mode": 1}}
  ]}}]
}}"#
    )
}

fn mesh_edge_cases_glb() -> Vec<u8> {
    let bin = mesh_edge_cases_bin();
    glb(&mesh_edge_cases_json(bin.len()), &bin)
}

// ---------------------------------------------------------------------------
// Probe fixtures (new)
// ---------------------------------------------------------------------------

/// One triangle (3 positions @0, u16 indices @36), with `{prims}` spliced into
/// the mesh and `{acc_extra}` / `{view_extra}` appended to accessors / views.
fn tri_glb(prims: &str, acc_extra: &str, view_extra: &str, extra_root: &str) -> Vec<u8> {
    let mut bin = f32_bytes(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]); // @0 (36)
    bin.extend(u16_bytes(&[0, 1, 2, 0, 1, 3])); // @36 (12): [0,1,2] good, [0,1,3] boundary-bad
    bin.extend(f32_bytes(&[0.0, 1.0, 2.0])); // @48 (12): float "indices"
    let json = format!(
        r#"{{
  "asset": {{"version": "2.0"}},
  "buffers": [{{"byteLength": {len}}}],
  "bufferViews": [
    {{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
    {{"buffer": 0, "byteOffset": 36, "byteLength": 6}},
    {{"buffer": 0, "byteOffset": 42, "byteLength": 6}},
    {{"buffer": 0, "byteOffset": 48, "byteLength": 12}}{view_extra}
  ],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0, 0, 0], "max": [1, 0, 1]}},
    {{"bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR"}},
    {{"bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR"}},
    {{"bufferView": 3, "componentType": 5126, "count": 3, "type": "SCALAR"}}{acc_extra}
  ],
  "meshes": [{{"primitives": [{prims}]}}]{extra_root}
}}"#,
        len = bin.len()
    );
    glb(&json, &bin)
}

/// Every glTF feature soorat's loaders read, in one file.
fn rich_parts() -> (String, Vec<u8>) {
    let mut bin = Vec::new();
    // @0 positions (4 × VEC3 f32, 48)
    bin.extend(f32_bytes(&[
        0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0,
    ]));
    // @48 normals (48)
    bin.extend(f32_bytes(&[
        0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.6, 0.8, 0.0, -0.6, 0.8,
    ]));
    // @96 uv f32 (4 × VEC2, 32)
    bin.extend(f32_bytes(&[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.25, 0.75]));
    // @128 u8 indices (6) + 2 pad
    bin.extend([0u8, 1, 2, 0, 2, 3, 0, 0]);
    // @136 u32 indices (24)
    for i in [3u32, 2, 0, 2, 1, 0] {
        bin.extend(i.to_le_bytes());
    }
    // @160 uv u16 normalized (4 × VEC2, 16)
    bin.extend(u16_bytes(&[0, 0, 65535, 0, 65535, 65535, 1, 32768]));
    // @176 inverse bind matrices (2 × MAT4, 128)
    bin.extend(f32_bytes(&[
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, -2.5, 0.0, 1.0,
    ]));
    bin.extend(f32_bytes(&[
        2.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.1, 0.2, 0.3, 1.0,
    ]));
    // @304 times (3 × f32, 12)
    bin.extend(f32_bytes(&[0.0, 0.5, 1.25]));
    // @316 rotations f32 (3 × VEC4, 48)
    let h = std::f32::consts::FRAC_1_SQRT_2;
    bin.extend(f32_bytes(&[
        0.0, 0.0, 0.0, 1.0, 0.0, h, 0.0, h, 0.0, 1.0, 0.0, 0.0,
    ]));
    // @364 scales (3 × VEC3, 36)
    bin.extend(f32_bytes(&[1.0, 1.0, 1.0, 2.0, 2.0, 2.0, 0.5, 1.5, 3.0]));
    // @400 rotations i16 normalized (3 × VEC4, 24)
    for v in [0i16, 0, 0, 32767, 0, 23170, 0, 23170, -32767, 0, 0, -32768] {
        bin.extend(v.to_le_bytes());
    }
    // @424 weights (3 × f32, 12)
    bin.extend(f32_bytes(&[0.0, 0.5, 1.0]));
    // @436 embedded "image" bytes (8)
    bin.extend([0x89u8, b'P', b'N', b'G', 1, 2, 3, 4]);
    // @444 interleaved position+normal, stride 24 (3 vertices, 72)
    bin.extend(f32_bytes(&[
        -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, //
        1.0, 0.0, 0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, -1.0, 0.0, 0.0, 1.0,
    ]));
    // @516 rotations u8 normalized (3 × VEC4, 12)
    bin.extend([0u8, 0, 0, 255, 0, 255, 0, 0, 128, 0, 0, 127]);
    // @528 end
    let json = format!(
        r#"{{
  "asset": {{"version": "2.0", "generator": "soorat P9 probe \u2603", "copyright": "tab\there \"q\" \\ \/"}},
  "extensionsUsed": ["KHR_materials_unlit"],
  "buffers": [{{"byteLength": {len}}}],
  "bufferViews": [
    {{"buffer": 0, "byteOffset": 0, "byteLength": 48, "target": 34962}},
    {{"buffer": 0, "byteOffset": 48, "byteLength": 48}},
    {{"buffer": 0, "byteOffset": 96, "byteLength": 32}},
    {{"buffer": 0, "byteOffset": 128, "byteLength": 6, "target": 34963}},
    {{"buffer": 0, "byteOffset": 136, "byteLength": 24}},
    {{"buffer": 0, "byteOffset": 160, "byteLength": 16}},
    {{"buffer": 0, "byteOffset": 176, "byteLength": 128}},
    {{"buffer": 0, "byteOffset": 304, "byteLength": 12}},
    {{"buffer": 0, "byteOffset": 316, "byteLength": 48}},
    {{"buffer": 0, "byteOffset": 364, "byteLength": 36}},
    {{"buffer": 0, "byteOffset": 400, "byteLength": 24}},
    {{"buffer": 0, "byteOffset": 424, "byteLength": 12}},
    {{"buffer": 0, "byteOffset": 436, "byteLength": 8}},
    {{"buffer": 0, "byteOffset": 444, "byteLength": 72, "byteStride": 24}},
    {{"buffer": 0, "byteOffset": 516, "byteLength": 12}}
  ],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3", "min": [0, 0, 0], "max": [1.0, 1E0, 0e0]}},
    {{"bufferView": 1, "componentType": 5126, "count": 4, "type": "VEC3"}},
    {{"bufferView": 2, "componentType": 5126, "count": 4, "type": "VEC2"}},
    {{"bufferView": 3, "componentType": 5121, "count": 6, "type": "SCALAR"}},
    {{"bufferView": 4, "componentType": 5125, "count": 6, "type": "SCALAR"}},
    {{"bufferView": 5, "componentType": 5123, "normalized": true, "count": 4, "type": "VEC2"}},
    {{"bufferView": 6, "componentType": 5126, "count": 2, "type": "MAT4"}},
    {{"bufferView": 7, "componentType": 5126, "count": 3, "type": "SCALAR", "min": [0], "max": [1.25]}},
    {{"bufferView": 8, "componentType": 5126, "count": 3, "type": "VEC4"}},
    {{"bufferView": 9, "componentType": 5126, "count": 3, "type": "VEC3"}},
    {{"bufferView": 10, "componentType": 5122, "normalized": true, "count": 3, "type": "VEC4"}},
    {{"bufferView": 11, "componentType": 5126, "count": 3, "type": "SCALAR"}},
    {{"bufferView": 13, "byteOffset": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1, 0, -1], "max": [1, 0, 0]}},
    {{"bufferView": 13, "byteOffset": 12, "componentType": 5126, "count": 3, "type": "VEC3"}},
    {{"bufferView": 14, "componentType": 5121, "normalized": true, "count": 3, "type": "VEC4"}}
  ],
  "materials": [{{"name": "m0", "pbrMetallicRoughness": {{"baseColorFactor": [1, 0.5, 0.25, 1.0], "metallicFactor": 0.0}}, "extensions": {{"KHR_materials_unlit": {{}}}}}}],
  "images": [
    {{"bufferView": 12, "mimeType": "image/png", "name": "embedded"}},
    {{"uri": "external.png"}}
  ],
  "meshes": [
    {{"name": "quad", "primitives": [
      {{"attributes": {{"POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2}}, "indices": 3, "material": 0}},
      {{"attributes": {{"TEXCOORD_0": 5, "POSITION": 0}}, "indices": 4, "mode": 4}},
      {{"attributes": {{"POSITION": 0}}}}
    ]}},
    {{"primitives": [
      {{"attributes": {{"POSITION": 12, "NORMAL": 13}}}}
    ]}}
  ],
  "nodes": [
    {{"name": "root", "translation": [1, 2.5e0, -0.0], "rotation": [0, 0, 0.38268343236508984, 0.9238795325112867], "scale": [1, 1, 1], "children": [1]}},
    {{"name": "mid", "translation": [0.1, 0.2, 0.30000001192092896], "children": [2]}},
    {{"name": "leaf", "translation": [1e-7, -123.456789012345678, 3.4028235e38], "rotation": [0, 0, 0, 1], "scale": [1.5E-3, 2E+2, 7]}},
    {{"name": "skinned", "mesh": 0, "skin": 0, "matrix": [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0.5, -1.25e1, 3, 1]}}
  ],
  "skins": [{{"joints": [0, 1, 2], "inverseBindMatrices": 6, "skeleton": 0}}],
  "animations": [
    {{
      "name": "w\u00e4lk \"q\" \ud83d\ude00",
      "samplers": [
        {{"input": 7, "output": 8, "interpolation": "LINEAR"}},
        {{"input": 7, "output": 9, "interpolation": "STEP"}},
        {{"input": 7, "output": 10}},
        {{"input": 7, "output": 11, "interpolation": "LINEAR"}},
        {{"input": 7, "output": 14, "interpolation": "STEP"}}
      ],
      "channels": [
        {{"sampler": 0, "target": {{"node": 0, "path": "rotation"}}}},
        {{"sampler": 1, "target": {{"node": 1, "path": "scale"}}}},
        {{"sampler": 2, "target": {{"node": 2, "path": "rotation"}}}},
        {{"sampler": 3, "target": {{"node": 3, "path": "weights"}}}},
        {{"sampler": 3, "target": {{"node": 2, "path": "weights"}}}},
        {{"sampler": 4, "target": {{"node": 1, "path": "rotation"}}}}
      ]
    }},
    {{
      "samplers": [{{"input": 7, "output": 9}}],
      "channels": [{{"sampler": 0, "target": {{"node": 2, "path": "scale"}}}}]
    }}
  ],
  "scenes": [{{"nodes": [0, 3], "name": "s"}}],
  "scene": 0,
  "extras": {{"nested": [[1, [2, [3.5e-3, {{"k": null, "e": [], "o": {{}}}}]]], true, false, -17]}}
}}"#,
        len = bin.len()
    );
    (json, bin)
}

/// xorshift64*, deterministic.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

/// 400 joint nodes whose TRS are 4,000 decimal numbers aimed at JSON → f32 parsing:
/// half shortest-round-trip f64 strings, half 20-digit decimals within a few f64 ulps of an
/// f32 rounding midpoint (where a 1-ulp f64 error changes the f32).
fn float_stress_glb() -> Vec<u8> {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut num = |k: usize| -> String {
        let r = rng.next();
        if k.is_multiple_of(2) {
            // random f64 in a wide but finite-as-f32 range
            let mant = (r >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
            let exp = ((r & 0x3f) as i32) - 32;
            let sign = if r & 0x40 != 0 { -1.0 } else { 1.0 };
            let v = sign * mant * 2f64.powi(exp);
            format!("{v:e}")
        } else {
            // midpoint between two adjacent f32s, nudged by a few f64 ulps, 20 significant digits
            let bits = ((r >> 32) as u32 & 0x007f_ffff) | (((r & 0x3f) as u32 + 100) << 23);
            let a = f32::from_bits(bits) as f64;
            let b = f32::from_bits(bits + 1) as f64;
            let mid = (a + b) / 2.0;
            let nudge = ((r >> 8) & 7) as i64 - 3; // -3..=4 ulps
            let m = f64::from_bits((mid.to_bits() as i64 + nudge) as u64);
            let sign = if r & 0x80 != 0 { "-" } else { "" };
            format!("{sign}{m:.19e}")
        }
    };
    let n = 400;
    let mut nodes = String::new();
    let mut k = 0usize;
    for i in 0..n {
        let mut f = |c: usize| -> String {
            let v: Vec<String> = (0..c)
                .map(|_| {
                    k += 1;
                    num(k)
                })
                .collect();
            v.join(", ")
        };
        let t = f(3);
        let r = f(4);
        let s = f(3);
        let children = if i + 1 < n {
            format!(", \"children\": [{}]", i + 1)
        } else {
            String::new()
        };
        let sep = if i + 1 < n { ",\n" } else { "\n" };
        let _ = write!(
            nodes,
            "    {{\"translation\": [{t}], \"rotation\": [{r}], \"scale\": [{s}]{children}}}{sep}"
        );
    }
    let joints: Vec<String> = (0..n).map(|i| i.to_string()).collect();
    let json = format!(
        "{{\n  \"asset\": {{\"version\": \"2.0\"}},\n  \"nodes\": [\n{nodes}  ],\n  \"skins\": [{{\"joints\": [{}]}}]\n}}",
        joints.join(", ")
    );
    // No BIN chunk at all: a JSON-only GLB.
    let mut json_chunk = json.into_bytes();
    while !json_chunk.len().is_multiple_of(4) {
        json_chunk.push(b' ');
    }
    let total = 12 + 8 + json_chunk.len();
    let mut out = Vec::new();
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_chunk.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json_chunk);
    out
}

fn put_u32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn probe_fixtures() -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    let edge = mesh_edge_cases_glb();
    let json_len = u32::from_le_bytes(edge[12..16].try_into().unwrap_or([0; 4])) as usize;
    let bin_hdr = 20 + json_len;

    v.push(("skinned_animation.glb".into(), skinned_animation_glb()));
    v.push(("mesh_edge_cases.glb".into(), edge.clone()));
    let (rj, rb) = rich_parts();
    v.push(("rich.glb".into(), glb(&rj, &rb)));
    v.push(("float_stress.glb".into(), float_stress_glb()));

    // --- container-level malformations (mesh_edge_cases as the base) ---
    for cut in [0usize, 4, 12, 20] {
        v.push((format!("trunc_{cut}.glb"), edge[..cut].to_vec()));
    }
    v.push(("trunc_half.glb".into(), edge[..edge.len() / 2].to_vec()));
    v.push(("trunc_in_bin.glb".into(), edge[..edge.len() - 4].to_vec()));
    let mut b = edge.clone();
    put_u32(&mut b, 12, (edge.len() as u32) * 4);
    v.push(("json_len_past_end.glb".into(), b));
    let mut b = edge.clone();
    put_u32(&mut b, bin_hdr, 4096);
    v.push(("bin_len_past_end.glb".into(), b));
    let mut b = edge.clone();
    put_u32(&mut b, 8, 8);
    v.push(("header_len_short.glb".into(), b));
    let mut b = edge.clone();
    put_u32(&mut b, 8, edge.len() as u32 + 64);
    v.push(("header_len_long.glb".into(), b));
    let mut b = edge.clone();
    put_u32(&mut b, 8, 100);
    v.push(("header_len_small_ok.glb".into(), b));
    let mut b = edge.clone();
    b[0] = b'g';
    b[1] = b'L';
    v.push(("bad_magic.glb".into(), b));
    let mut b = edge.clone();
    put_u32(&mut b, 4, 1);
    v.push(("version1.glb".into(), b));
    let mut b = edge.clone();
    b[16..20].copy_from_slice(b"BIN\0");
    v.push(("first_chunk_bin.glb".into(), b));
    let mut b = edge.clone();
    b[bin_hdr + 4..bin_hdr + 8].copy_from_slice(b"XYZW");
    v.push(("unknown_chunk.glb".into(), b));
    let mut b = edge[..bin_hdr].to_vec();
    b.extend([0u8, 0, 0, 0]);
    let l = b.len() as u32;
    put_u32(&mut b, 8, l);
    v.push(("trailing_4_bytes.glb".into(), b));
    // JSON chunk padded with NULs instead of spaces
    let bin = mesh_edge_cases_bin();
    let mut j = mesh_edge_cases_json(bin.len()).into_bytes();
    j.push(0);
    while !j.len().is_multiple_of(4) {
        j.push(0);
    }
    let mut b = Vec::new();
    b.extend_from_slice(b"glTF");
    b.extend(2u32.to_le_bytes());
    b.extend(((12 + 8 + j.len() + 8 + bin.len()) as u32).to_le_bytes());
    b.extend((j.len() as u32).to_le_bytes());
    b.extend_from_slice(b"JSON");
    b.extend(&j);
    b.extend((bin.len() as u32).to_le_bytes());
    b.extend_from_slice(b"BIN\0");
    b.extend(&bin);
    v.push(("json_nul_padded.glb".into(), b));
    // Plain JSON glTF (not GLB) — the Rust loader accepts it too.
    v.push(("plain.gltf".into(), rj.into_bytes()));
    v.push(("not_json.gltf".into(), b"not a gltf file".to_vec()));

    // --- primitive-level ---
    let pos = r#""attributes": {"POSITION": 0}"#;
    v.push((
        "index_oob_boundary.glb".into(),
        tri_glb(
            &format!("{{{pos}, \"indices\": 2}}, {{{pos}, \"indices\": 1}}"),
            "",
            "",
            "",
        ),
    ));
    v.push((
        "modes.glb".into(),
        tri_glb(
            &(0..=6)
                .map(|m| format!("{{{pos}, \"indices\": 1, \"mode\": {m}}}"))
                .chain(std::iter::once(format!("{{{pos}}}")))
                .collect::<Vec<_>>()
                .join(", "),
            "",
            "",
            "",
        ),
    ));
    v.push((
        "mode_invalid.glb".into(),
        tri_glb(&format!("{{{pos}, \"mode\": 7}}"), "", "", ""),
    ));
    v.push((
        "posview_past_bin.glb".into(),
        tri_glb(
            r#"{"attributes": {"POSITION": 4}}, {"attributes": {"POSITION": 0}}"#,
            r#",
    {"bufferView": 4, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0, 0, 0], "max": [1, 0, 1]}"#,
            r#",
    {"buffer": 0, "byteOffset": 56, "byteLength": 36}"#,
            "",
        ),
    ));
    v.push((
        "accessor_past_view.glb".into(),
        tri_glb(
            r#"{"attributes": {"POSITION": 4}}, {"attributes": {"POSITION": 0}}"#,
            r#",
    {"bufferView": 0, "byteOffset": 4, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0, 0, 0], "max": [1, 0, 1]}"#,
            "",
            "",
        ),
    ));
    v.push((
        "position_stride4.glb".into(),
        tri_glb(
            r#"{"attributes": {"POSITION": 4}}"#,
            r#",
    {"bufferView": 4, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0, 0, 0], "max": [1, 0, 1]}"#,
            r#",
    {"buffer": 0, "byteOffset": 0, "byteLength": 36, "byteStride": 4}"#,
            "",
        ),
    ));
    v.push((
        "position_no_minmax.glb".into(),
        tri_glb(
            r#"{"attributes": {"POSITION": 4}}"#,
            r#",
    {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"}"#,
            "",
            "",
        ),
    ));
    v.push((
        "position_accessor_oob.glb".into(),
        tri_glb(r#"{"attributes": {"POSITION": 99}}"#, "", "", ""),
    ));
    v.push((
        "indices_float.glb".into(),
        tri_glb(&format!("{{{pos}, \"indices\": 3}}"), "", "", ""),
    ));
    v.push((
        "count_zero.glb".into(),
        tri_glb(
            r#"{"attributes": {"POSITION": 4}}"#,
            r#",
    {"bufferView": 0, "componentType": 5126, "count": 0, "type": "VEC3", "min": [0, 0, 0], "max": [1, 0, 1]}"#,
            "",
            "",
        ),
    ));
    v.push((
        "count_float.glb".into(),
        tri_glb(
            r#"{"attributes": {"POSITION": 4}}"#,
            r#",
    {"bufferView": 0, "componentType": 5126, "count": 3.0, "type": "VEC3", "min": [0, 0, 0], "max": [1, 0, 1]}"#,
            "",
            "",
        ),
    ));
    v.push((
        "count_negative.glb".into(),
        tri_glb(
            r#"{"attributes": {"POSITION": 4}}"#,
            r#",
    {"bufferView": 0, "componentType": 5126, "count": -3, "type": "VEC3", "min": [0, 0, 0], "max": [1, 0, 1]}"#,
            "",
            "",
        ),
    ));
    v.push((
        "dup_key_known.glb".into(),
        tri_glb(
            r#"{"attributes": {"POSITION": 4}}"#,
            r#",
    {"bufferView": 0, "componentType": 5126, "count": 3, "count": 3, "type": "VEC3", "min": [0, 0, 0], "max": [1, 0, 1]}"#,
            "",
            "",
        ),
    ));
    v.push((
        "dup_key_extras.glb".into(),
        tri_glb(
            &format!("{{{pos}}}"),
            "",
            "",
            r#",
  "extras": {"a": 1, "a": 2}"#,
        ),
    ));
    v.push((
        "dup_key_unknown.glb".into(),
        tri_glb(
            &format!("{{{pos}, \"x_vendor\": 1, \"x_vendor\": 2}}"),
            "",
            "",
            "",
        ),
    ));
    v.push((
        "huge_int.glb".into(),
        tri_glb(
            &format!("{{{pos}}}"),
            "",
            "",
            r#",
  "extras": {"big": 18446744073709551615}"#,
        ),
    ));
    v.push((
        "deep_nesting.glb".into(),
        tri_glb(
            &format!("{{{pos}}}"),
            "",
            "",
            &format!(
                ",\n  \"extras\": {}1{}",
                "{\"a\": ".repeat(200),
                "}".repeat(200)
            ),
        ),
    ));
    v.push((
        "nesting_100.glb".into(),
        tri_glb(
            &format!("{{{pos}}}"),
            "",
            "",
            &format!(
                ",\n  \"extras\": {}1{}",
                "{\"a\": ".repeat(100),
                "}".repeat(100)
            ),
        ),
    ));
    v.push((
        "extras_array.glb".into(),
        tri_glb(&format!("{{{pos}}}"), "", "", ",\n  \"extras\": [1]"),
    ));
    v.push((
        "bad_escape.glb".into(),
        tri_glb(
            &format!("{{{pos}}}"),
            "",
            "",
            ",\n  \"extensionsUsed\": [\"\\x41\"]",
        ),
    ));
    // --- animation / skin level ---
    let anim_interp = |chan: &str, skin: &str, interp: &str| -> Vec<u8> {
        let bin = f32_bytes(&[0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        let json = format!(
            r#"{{
  "asset": {{"version": "2.0"}},
  "buffers": [{{"byteLength": 32}}],
  "bufferViews": [{{"buffer": 0, "byteOffset": 0, "byteLength": 8}}, {{"buffer": 0, "byteOffset": 8, "byteLength": 24}}],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 2, "type": "SCALAR"}},
    {{"bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC3"}}
  ],
  "nodes": [{{}}, {{}}],
  "skins": [{skin}],
  "animations": [{{"samplers": [{{"input": 0, "output": 1{interp}}}], "channels": [{chan}]}}]
}}"#
        );
        glb(&json, &bin)
    };
    let anim_base = |chan: &str, skin: &str| anim_interp(chan, skin, "");
    v.push((
        "channel_sampler_oob.glb".into(),
        anim_base(
            r#"{"sampler": 3, "target": {"node": 0, "path": "translation"}}"#,
            r#"{"joints": [0, 1]}"#,
        ),
    ));
    v.push((
        "skin_joint_oob.glb".into(),
        anim_base(
            r#"{"sampler": 0, "target": {"node": 0, "path": "translation"}}"#,
            r#"{"joints": [0, 7]}"#,
        ),
    ));
    v.push((
        "channel_bad_path.glb".into(),
        anim_base(
            r#"{"sampler": 0, "target": {"node": 0, "path": "position"}}"#,
            r#"{"joints": [0, 1]}"#,
        ),
    ));
    v.push((
        "channel_no_node.glb".into(),
        anim_base(
            r#"{"sampler": 0, "target": {"path": "translation"}}"#,
            r#"{"joints": [0, 1]}"#,
        ),
    ));
    v.push((
        "sampler_bad_interp.glb".into(),
        anim_interp(
            r#"{"sampler": 0, "target": {"node": 0, "path": "translation"}}"#,
            r#"{"joints": [0, 1]}"#,
            r#", "interpolation": "SMOOTH""#,
        ),
    ));
    v.push((
        "ext_required.glb".into(),
        tri_glb(
            &format!("{{{pos}}}"),
            "",
            "",
            ",\n  \"extensionsUsed\": [\"KHR_materials_unlit\"], \"extensionsRequired\": [\"KHR_materials_unlit\"]",
        ),
    ));
    v.push((
        "bad_semantic.glb".into(),
        tri_glb(
            r#"{"attributes": {"POSITION": 0, "TEXCOORD_x": 0}}"#,
            "",
            "",
            "",
        ),
    ));
    v.push((
        "anim_ok_small.glb".into(),
        anim_base(
            r#"{"sampler": 0, "target": {"node": 1, "path": "scale"}}"#,
            r#"{"joints": [1, 0]}"#,
        ),
    ));
    v
}

// ---------------------------------------------------------------------------
// M3 part c fixtures: the schema, sparse accessors, matrix joints, the reader's rules
// ---------------------------------------------------------------------------

/// The schema document: one valid triangle (tri_glb's buffers) with every top-level collection soorat's schema
/// checks touch. Each `schema_*` fixture replaces one substring of it.
fn schema_parts() -> (String, String, String) {
    let prims = r#"{"attributes": {"POSITION": 0, "TEXCOORD_0": 4}, "indices": 1, "material": 0, "mode": 4, "targets": [{"POSITION": 0, "NORMAL": 0}], "extras": {}, "extensions": {}}"#.to_string();
    let acc = r#",
    {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC2", "normalized": false, "sparse": null, "name": null}"#
        .to_string();
    let root = r#",
  "materials": [{"name": "m", "alphaMode": "OPAQUE", "alphaCutoff": 0.5, "doubleSided": true,
    "pbrMetallicRoughness": {"baseColorFactor": [1, 0.5, 0.25, 1], "baseColorTexture": {"index": 0, "texCoord": 0},
      "metallicFactor": 0, "roughnessFactor": 0.5, "metallicRoughnessTexture": {"index": 0}},
    "normalTexture": {"index": 0, "scale": 1.5, "texCoord": 1}, "occlusionTexture": {"index": 0, "strength": 0.5},
    "emissiveTexture": {"index": 0}, "emissiveFactor": [0, 0.1, 0.2], "extensions": {}, "extras": {}}],
  "textures": [{"sampler": 0, "source": 0, "name": null}],
  "samplers": [{"magFilter": 9729, "minFilter": 9987, "wrapS": 33071, "wrapT": 33648}],
  "images": [{"uri": "x.png", "mimeType": "image/png"}],
  "cameras": [{"type": "perspective", "perspective": {"yfov": 0.8, "znear": 0.01, "zfar": null, "aspectRatio": 1.5}},
    {"type": "orthographic", "orthographic": {"xmag": 1, "ymag": 1, "zfar": 100, "znear": 0}}],
  "nodes": [{"mesh": 0, "camera": 1, "name": null, "matrix": null, "children": null, "translation": [0, 0, 0]}],
  "scenes": [{"nodes": [0]}],
  "scene": 0,
  "extensionsUsed": [],
  "extensionsRequired": [],
  "extras": {}"#
        .to_string();
    (prims, acc, root)
}

fn schema(from: &str, to: &str) -> Vec<u8> {
    let (p, a, r) = schema_parts();
    let rep = |s: &str| {
        if from.is_empty() {
            s.to_string()
        } else {
            s.replacen(from, to, 1)
        }
    };
    let bin_len = 60;
    let mut glb_bytes = tri_glb(&rep(&p), &rep(&a), "", &rep(&r));
    if !from.is_empty() && !p.contains(from) && !a.contains(from) && !r.contains(from) {
        // a replacement in tri_glb's own text (asset, buffers, views, accessors 0–3)
        let (json, bin) = split_glb(&glb_bytes);
        let json = json.replacen(from, to, 1);
        glb_bytes = glb(&json, &bin);
    }
    let _ = bin_len;
    glb_bytes
}

/// The JSON text and BIN chunk of a GLB built by `glb`.
fn split_glb(b: &[u8]) -> (String, Vec<u8>) {
    let jl = u32::from_le_bytes([b[12], b[13], b[14], b[15]]) as usize;
    let json = String::from_utf8_lossy(&b[20..20 + jl])
        .trim_end()
        .to_string();
    let bl = u32::from_le_bytes([b[20 + jl], b[21 + jl], b[22 + jl], b[23 + jl]]) as usize;
    (json, b[28 + jl..28 + jl + bl].to_vec())
}

fn schema_fixtures(v: &mut Vec<(String, Vec<u8>)>) {
    let cases: &[(&str, &str, &str)] = &[
        ("schema_ok.glb", "", ""),
        (
            "schema_alpha_invalid.glb",
            r#""alphaMode": "OPAQUE""#,
            r#""alphaMode": "SHINY""#,
        ),
        (
            "schema_base_color_len3.glb",
            "[1, 0.5, 0.25, 1]",
            "[1, 0.5, 0.25]",
        ),
        ("schema_emissive_len2.glb", "[0, 0.1, 0.2]", "[0, 0.1]"),
        (
            "schema_normal_tex_oob.glb",
            r#""normalTexture": {"index": 0"#,
            r#""normalTexture": {"index": 5"#,
        ),
        (
            "schema_occlusion_string.glb",
            r#""strength": 0.5"#,
            r#""strength": "half""#,
        ),
        (
            "schema_texcoord_negative.glb",
            r#""texCoord": 1"#,
            r#""texCoord": -1"#,
        ),
        ("schema_texture_no_source.glb", r#""source": 0, "#, ""),
        (
            "schema_texture_source_oob.glb",
            r#""source": 0"#,
            r#""source": 3"#,
        ),
        (
            "schema_mag_invalid.glb",
            r#""magFilter": 9729"#,
            r#""magFilter": 9984"#,
        ),
        (
            "schema_wrap_invalid.glb",
            r#""wrapS": 33071"#,
            r#""wrapS": 1"#,
        ),
        (
            "schema_min_negative.glb",
            r#""minFilter": 9987"#,
            r#""minFilter": -1"#,
        ),
        (
            "schema_camera_type_invalid.glb",
            r#""type": "perspective""#,
            r#""type": "fisheye""#,
        ),
        (
            "schema_camera_no_projection.glb",
            r#", "perspective": {"yfov": 0.8, "znear": 0.01, "zfar": null, "aspectRatio": 1.5}"#,
            "",
        ),
        ("schema_persp_no_yfov.glb", r#""yfov": 0.8, "#, ""),
        (
            "schema_ortho_null_zfar.glb",
            r#""zfar": 100"#,
            r#""zfar": null"#,
        ),
        (
            "schema_scene_no_nodes.glb",
            r#""scenes": [{"nodes": [0]}]"#,
            r#""scenes": [{}]"#,
        ),
        ("schema_scene_oob.glb", r#""scene": 0"#, r#""scene": 3"#),
        (
            "schema_name_number.glb",
            r#""name": null, "matrix""#,
            r#""name": 5, "matrix""#,
        ),
        (
            "schema_extras_null.glb",
            "\"extras\": {}\n",
            "\"extras\": null\n",
        ),
        (
            "schema_extras_empty_array.glb",
            "\"extras\": {}\n",
            "\"extras\": []\n",
        ),
        (
            "schema_extras_null_array.glb",
            "\"extras\": {}\n",
            "\"extras\": [null]\n",
        ),
        (
            "schema_extras_one_array.glb",
            "\"extras\": {}\n",
            "\"extras\": [1]\n",
        ),
        (
            "schema_extensions_empty_array.glb",
            r#""extras": {}, "extensions": {}}"#,
            r#""extras": {}, "extensions": []}"#,
        ),
        (
            "schema_extensions_one_array.glb",
            r#""extras": {}, "extensions": {}}"#,
            r#""extras": {}, "extensions": [1]}"#,
        ),
        (
            "schema_extensions_null.glb",
            r#""extras": {}, "extensions": {}}"#,
            r#""extras": {}, "extensions": null}"#,
        ),
        (
            "schema_mode_wraps.glb",
            r#""mode": 4,"#,
            r#""mode": 4294967300,"#,
        ),
        (
            "schema_index_wraps.glb",
            r#""indices": 1,"#,
            r#""indices": 4294967297,"#,
        ),
        (
            "schema_material_wraps.glb",
            r#""material": 0,"#,
            r#""material": 4294967296,"#,
        ),
        (
            "schema_index_float.glb",
            r#""indices": 1,"#,
            r#""indices": 1.0,"#,
        ),
        (
            "schema_texcoord_00.glb",
            r#""TEXCOORD_0": 4"#,
            r#""TEXCOORD_00": 4"#,
        ),
        (
            "schema_texcoord_two_keys.glb",
            r#""TEXCOORD_0": 4"#,
            r#""TEXCOORD_0": 3, "TEXCOORD_+0": 4"#,
        ),
        (
            "schema_texcoord_plus_first.glb",
            r#""TEXCOORD_0": 4"#,
            r#""TEXCOORD_+0": 4, "TEXCOORD_0": 3"#,
        ),
        (
            "schema_morph_oob.glb",
            r#""targets": [{"POSITION": 0"#,
            r#""targets": [{"POSITION": 9"#,
        ),
        (
            "schema_node_camera_oob.glb",
            r#""camera": 1"#,
            r#""camera": 5"#,
        ),
        (
            "schema_root_array_null.glb",
            r#""samplers": [{"#,
            r#""samplersX": [{"#,
        ),
        (
            "schema_nodes_null.glb",
            r#""nodes": [{"mesh": 0"#,
            r#""nodesX": [{"mesh": 0"#,
        ),
        (
            "schema_ext_required_empty.glb",
            r#""extensionsRequired": []"#,
            r#""extensionsRequired": []"#,
        ),
        (
            "schema_ext_used_number.glb",
            r#""extensionsUsed": []"#,
            r#""extensionsUsed": [1]"#,
        ),
        (
            "schema_asset_version_number.glb",
            r#""version": "2.0""#,
            r#""version": 2"#,
        ),
        (
            "schema_asset_missing.glb",
            r#""asset": {"version": "2.0"},"#,
            "",
        ),
        (
            "schema_buffer_no_length.glb",
            r#""buffers": [{"byteLength": 60}]"#,
            r#""buffers": [{}]"#,
        ),
        (
            "schema_stride_zero.glb",
            r#""byteOffset": 36, "byteLength": 6}"#,
            r#""byteOffset": 36, "byteLength": 6, "byteStride": 0}"#,
        ),
        (
            "schema_stride_253.glb",
            r#""byteOffset": 36, "byteLength": 6}"#,
            r#""byteOffset": 36, "byteLength": 6, "byteStride": 253}"#,
        ),
        (
            "schema_target_ok.glb",
            r#""byteOffset": 36, "byteLength": 6}"#,
            r#""byteOffset": 36, "byteLength": 6, "target": 34963}"#,
        ),
        (
            "schema_target_invalid.glb",
            r#""byteOffset": 36, "byteLength": 6}"#,
            r#""byteOffset": 36, "byteLength": 6, "target": 1}"#,
        ),
        (
            "schema_comptype_int.glb",
            r#""count": 3, "type": "VEC2", "normalized""#,
            r#""count": 3, "componentType": 5124, "type": "VEC2", "normalized""#,
        ),
        (
            "schema_type_mat5.glb",
            r#""type": "VEC2""#,
            r#""type": "MAT5""#,
        ),
        (
            "schema_normalized_string.glb",
            r#""normalized": false"#,
            r#""normalized": "no""#,
        ),
        (
            "schema_accessor_no_view.glb",
            r#"{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC2""#,
            r#"{"componentType": 5126, "count": 3, "type": "VEC2""#,
        ),
        (
            "schema_pos_min_len2.glb",
            r#""min": [0, 0, 0], "max": [1, 0, 1]"#,
            r#""min": [0, 0], "max": [1, 0, 1]"#,
        ),
        (
            "schema_pos_max_string.glb",
            r#""min": [0, 0, 0], "max": [1, 0, 1]"#,
            r#""min": [0, 0, 0], "max": "big""#,
        ),
        (
            "schema_count_null.glb",
            r#""componentType": 5126, "count": 3, "type": "VEC2""#,
            r#""componentType": 5126, "count": null, "type": "VEC2""#,
        ),
        (
            "schema_image_view_no_mime.glb",
            r#""images": [{"uri": "x.png", "mimeType": "image/png"}]"#,
            r#""images": [{"bufferView": 0}]"#,
        ),
        (
            "schema_image_neither.glb",
            r#""images": [{"uri": "x.png", "mimeType": "image/png"}]"#,
            r#""images": [{}]"#,
        ),
        (
            "schema_image_view_bytes.glb",
            r#""images": [{"uri": "x.png", "mimeType": "image/png"}]"#,
            r#""images": [{"bufferView": 1, "mimeType": "image/png"}, {"bufferView": 3, "mimeType": "image/jpeg"}]"#,
        ),
        (
            "schema_bool_number.glb",
            r#""doubleSided": true"#,
            r#""doubleSided": 1"#,
        ),
        (
            "schema_weights.glb",
            r#""translation": [0, 0, 0]}"#,
            r#""translation": [0, 0, 0], "weights": [0.5, 1]}"#,
        ),
        (
            "schema_weights_string.glb",
            r#""translation": [0, 0, 0]}"#,
            r#""translation": [0, 0, 0], "weights": ["a"]}"#,
        ),
        (
            "schema_rotation_len3.glb",
            r#""translation": [0, 0, 0]}"#,
            r#""translation": [0, 0, 0], "rotation": [0, 0, 1]}"#,
        ),
        (
            "schema_float_overflow.glb",
            r#""alphaCutoff": 0.5"#,
            r#""alphaCutoff": 1e400"#,
        ),
        (
            "schema_float_big.glb",
            r#""alphaCutoff": 0.5"#,
            r#""alphaCutoff": 1e300"#,
        ),
    ];
    for (name, from, to) in cases {
        v.push(((*name).to_string(), schema(from, to)));
    }
}

/// Sparse accessors: with and without a base view, u8/u16/u32 indices, unsorted and repeated indices, values out of
/// the buffer, and a sparse animation output.
fn sparse_glb(variant: usize) -> Vec<u8> {
    let mut bin = f32_bytes(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]); // @0 positions (36)
    bin.extend([1u8, 0, 0, 0]); // @36 u8 index [1] (+3 pad)
    bin.extend(f32_bytes(&[5.0, 5.0, 5.0])); // @40 one VEC3 value (12)
    bin.extend(u16_bytes(&[0, 2])); // @52 u16 indices (4)
    bin.extend(f32_bytes(&[7.0, 7.0, 7.0, 8.0, 8.0, 8.0])); // @56 two VEC3 values (24)
    for i in [2u32, 0] {
        bin.extend(i.to_le_bytes()); // @80 u32 indices, unsorted (8)
    }
    bin.extend(f32_bytes(&[9.0, 9.0, 9.0, 6.0, 6.0, 6.0])); // @88 two VEC3 values (24)
    bin.extend(f32_bytes(&[0.0, 1.0])); // @112 times (8)
    bin.extend(f32_bytes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0])); // @120 translations (24)
    bin.extend([1u8, 1, 0, 0]); // @144 u8 indices [1, 1] (+2 pad)
    let mm = r#""min": [0, 0, 0], "max": [1, 0, 1]"#;
    let prims = match variant {
        0 => {
            r#"{"attributes": {"POSITION": 1}}, {"attributes": {"POSITION": 2}}, {"attributes": {"POSITION": 3}}, {"attributes": {"POSITION": 4}}, {"attributes": {"POSITION": 0, "NORMAL": 2}}, {"attributes": {"POSITION": 7}}"#
        }
        _ => r#"{"attributes": {"POSITION": 8}}"#,
    };
    let json = format!(
        r#"{{
  "asset": {{"version": "2.0"}},
  "buffers": [{{"byteLength": {len}}}],
  "bufferViews": [
    {{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
    {{"buffer": 0, "byteOffset": 36, "byteLength": 1}},
    {{"buffer": 0, "byteOffset": 40, "byteLength": 12}},
    {{"buffer": 0, "byteOffset": 52, "byteLength": 4}},
    {{"buffer": 0, "byteOffset": 56, "byteLength": 24}},
    {{"buffer": 0, "byteOffset": 80, "byteLength": 8}},
    {{"buffer": 0, "byteOffset": 88, "byteLength": 24}},
    {{"buffer": 0, "byteOffset": 4000, "byteLength": 24}},
    {{"buffer": 0, "byteOffset": 112, "byteLength": 8}},
    {{"buffer": 0, "byteOffset": 120, "byteLength": 24}},
    {{"buffer": 0, "byteOffset": 144, "byteLength": 2}}
  ],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", {mm}}},
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", {mm}, "sparse": {{"count": 1, "indices": {{"bufferView": 1, "componentType": 5121}}, "values": {{"bufferView": 2}}}}}},
    {{"componentType": 5126, "count": 3, "type": "VEC3", {mm}, "sparse": {{"count": 2, "indices": {{"bufferView": 3, "componentType": 5123}}, "values": {{"bufferView": 4}}}}}},
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", {mm}, "sparse": {{"count": 2, "indices": {{"bufferView": 5, "componentType": 5125}}, "values": {{"bufferView": 6}}}}}},
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", {mm}, "sparse": {{"count": 2, "indices": {{"bufferView": 5, "componentType": 5125}}, "values": {{"bufferView": 7}}}}}},
    {{"bufferView": 8, "componentType": 5126, "count": 2, "type": "SCALAR"}},
    {{"bufferView": 9, "componentType": 5126, "count": 2, "type": "VEC3", "sparse": {{"count": 1, "indices": {{"bufferView": 1, "componentType": 5121}}, "values": {{"bufferView": 2}}}}}},
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", {mm}, "sparse": {{"count": 2, "indices": {{"bufferView": 10, "componentType": 5121}}, "values": {{"bufferView": 4}}}}}},
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", {mm}, "sparse": {{"count": 1, "indices": {{"bufferView": 1, "byteOffset": 2, "componentType": 5121}}, "values": {{"bufferView": 2}}}}}}
  ],
  "meshes": [{{"primitives": [{prims}]}}],
  "nodes": [{{}}, {{}}],
  "skins": [{{"joints": [0, 1], "inverseBindMatrices": 6}}],
  "animations": [{{"samplers": [{{"input": 5, "output": 6}}], "channels": [{{"sampler": 0, "target": {{"node": 1, "path": "translation"}}}}]}}]
}}"#,
        len = bin.len()
    );
    glb(&json, &bin)
}

/// Joints given as matrices (column-major): gltf decomposes each in f32 (`Transform::decomposed`).
fn matrix_joints_glb() -> Vec<u8> {
    let mats: &[&str] = &[
        "1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1",
        "0, 2, 0, 0, -3, 0, 0, 0, 0, 0, 4, 0, 1, 2, 3, 1",
        "-1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1",
        "1, 0, 0, 0, 0, -1, 0, 0, 0, 0, -1, 0, 0, 0, 0, 1",
        "-1, 0, 0, 0, 0, 1, 0, 0, 0, 0, -1, 0, 0, 0, 0, 1",
        "-1, 0, 0, 0, 0, -1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1",
        "0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 5, 5, 5, 1",
        "1, 0, 0, 0, 0.5, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1",
        "0.36, -0.8, 0.48, 0, 0.48, 0.6, 0.64, 0, -0.8, 0, 0.6, 0, 0.1, -0.2, 0.3, 1",
        "1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1",
        "1e30, 0, 0, 0, 0, 1e30, 0, 0, 0, 0, 1e30, 0, 0, 0, 0, 1",
        "0.1, 0.2, 0.3, 0, -0.4, 0.5, 0.6, 0, 0.7, 0.8, -0.9, 0, 7, 8, 9, 1",
        "2, 0, 0, 0, 0, 2, 0, 0, 0, 0, 2, 0, 0, 0, 0, 1",
        "0, 0, -1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1",
    ];
    let mut nodes: Vec<String> = mats
        .iter()
        .map(|m| format!("{{\"matrix\": [{m}]}}"))
        .collect();
    nodes.push(r#"{"matrix": [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 4, 5, 6, 1], "translation": [9, 9, 9], "scale": [3, 3, 3]}"#.into());
    let joints: Vec<String> = (0..nodes.len()).map(|i| i.to_string()).collect();
    let json = format!(
        "{{\"asset\": {{\"version\": \"2.0\"}}, \"nodes\": [{}], \"skins\": [{{\"joints\": [{}]}}]}}",
        nodes.join(", "),
        joints.join(", ")
    );
    glb(&json, &[])
}

/// One animated skin over `nodes`, with `extra` spliced into the root: the reader's rules for channels.
fn anim_doc(
    bin: &[u8],
    views: &str,
    accessors: &str,
    nodes: &str,
    skins: &str,
    samplers: &str,
    channels: &str,
) -> Vec<u8> {
    let json = format!(
        r#"{{
  "asset": {{"version": "2.0"}},
  "buffers": [{{"byteLength": {len}}}],
  "bufferViews": [{views}],
  "accessors": [{accessors}],
  "nodes": [{nodes}],
  "skins": [{skins}],
  "animations": [{{"name": "a", "samplers": [{samplers}], "channels": [{channels}]}}]
}}"#,
        len = bin.len()
    );
    glb(&json, bin)
}

fn anim_fixtures(v: &mut Vec<(String, Vec<u8>)>) {
    let mut bin = f32_bytes(&[0.0, 1.0]); // @0 times (8)
    bin.extend(f32_bytes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0])); // @8 VEC3 × 2 (24)
    for x in [0u16, 0, 0, 65535, 0, 65535, 0, 0] {
        bin.extend(x.to_le_bytes()); // @32 u16 rotations (16)
    }
    bin.extend([0u8, 0, 0, 127, 0, 0x81, 0, 0x80]); // @48 i8 rotations (8)
    bin.extend(f32_bytes(&[0.0, f32::NAN])); // @56 times with a NaN last (8)
    bin.extend(f32_bytes(&[0.0, 0.5, 1.0, 1.0, 2.0])); // @64 five times (20)
    bin.extend(f32_bytes(&(0..15).map(|k| k as f32).collect::<Vec<_>>())); // @84 five VEC3 (60)
    for x in [0u32, 0, 0, 1, 0, 0, 1, 0] {
        bin.extend(x.to_le_bytes()); // @144 u32 rotations (32)
    }
    let views = r#"{"buffer": 0, "byteOffset": 0, "byteLength": 8}, {"buffer": 0, "byteOffset": 8, "byteLength": 24},
    {"buffer": 0, "byteOffset": 32, "byteLength": 16}, {"buffer": 0, "byteOffset": 48, "byteLength": 8},
    {"buffer": 0, "byteOffset": 56, "byteLength": 8}, {"buffer": 0, "byteOffset": 64, "byteLength": 20},
    {"buffer": 0, "byteOffset": 84, "byteLength": 60}, {"buffer": 0, "byteOffset": 144, "byteLength": 32}"#;
    let accs = r#"{"bufferView": 0, "componentType": 5126, "count": 2, "type": "SCALAR"},
    {"bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC3"},
    {"bufferView": 2, "componentType": 5123, "normalized": true, "count": 2, "type": "VEC4"},
    {"bufferView": 3, "componentType": 5120, "normalized": true, "count": 2, "type": "VEC4"},
    {"bufferView": 4, "componentType": 5126, "count": 2, "type": "SCALAR"},
    {"bufferView": 5, "componentType": 5126, "count": 5, "type": "SCALAR"},
    {"bufferView": 6, "componentType": 5126, "count": 5, "type": "VEC3"},
    {"bufferView": 7, "componentType": 5125, "count": 2, "type": "VEC4"},
    {"bufferView": 1, "componentType": 5121, "count": 2, "type": "VEC3"},
    {"bufferView": 6, "componentType": 5126, "count": 4, "type": "VEC3"},
    {"bufferView": 2, "componentType": 5121, "count": 2, "type": "SCALAR"}"#;
    let n3 = r#"{"children": [1]}, {}, {}"#;
    let sk = r#"{"joints": [0, 1]}"#;
    let ch = |s: usize, n: usize, p: &str| {
        format!(r#"{{"sampler": {s}, "target": {{"node": {n}, "path": "{p}"}}}}"#)
    };
    let cases: Vec<(&str, String, String, String, String)> = vec![
        (
            "anim_rot_u16.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 0, "output": 2}"#.into(),
            ch(0, 1, "rotation"),
        ),
        (
            "anim_rot_i8.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 0, "output": 3}"#.into(),
            ch(0, 1, "rotation"),
        ),
        (
            "anim_rot_u32.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 0, "output": 7}"#.into(),
            ch(0, 1, "rotation"),
        ),
        (
            "anim_translation_u8.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 0, "output": 8}"#.into(),
            ch(0, 0, "translation"),
        ),
        (
            "anim_times_u8.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 10, "output": 1}"#.into(),
            ch(0, 0, "scale"),
        ),
        (
            "anim_times_nan_last.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 4, "output": 1}"#.into(),
            ch(0, 0, "translation"),
        ),
        (
            "anim_step_five.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 5, "output": 6, "interpolation": "STEP"}"#.into(),
            ch(0, 1, "scale"),
        ),
        (
            "anim_cubic_short.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 5, "output": 9, "interpolation": "CUBICSPLINE"}"#.into(),
            ch(0, 1, "translation"),
        ),
        (
            "anim_linear_more_times.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 5, "output": 1}"#.into(),
            ch(0, 0, "translation"),
        ),
        (
            "anim_two_skins.glb",
            n3.into(),
            r#"{"joints": [0]}, {"joints": [1, 2]}"#.into(),
            r#"{"input": 0, "output": 1}"#.into(),
            format!("{}, {}", ch(0, 2, "translation"), ch(0, 0, "scale")),
        ),
        (
            "anim_joint_twice.glb",
            n3.into(),
            r#"{"joints": [0, 1, 0]}"#.into(),
            r#"{"input": 0, "output": 1}"#.into(),
            ch(0, 0, "translation"),
        ),
        (
            "anim_target_node_oob.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 0, "output": 1}"#.into(),
            ch(0, 9, "translation"),
        ),
        (
            "anim_bad_path_non_joint.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 0, "output": 1}"#.into(),
            ch(0, 2, "position"),
        ),
        (
            "anim_no_channels.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 0, "output": 1}"#.into(),
            String::new(),
        ),
        (
            "anim_no_skins.glb",
            n3.into(),
            String::new(),
            r#"{"input": 0, "output": 1}"#.into(),
            ch(0, 0, "translation"),
        ),
        (
            "anim_interp_null.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 0, "output": 1, "interpolation": null}"#.into(),
            ch(0, 0, "translation"),
        ),
        (
            "anim_input_oob.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 40, "output": 1}"#.into(),
            ch(0, 0, "translation"),
        ),
        (
            "anim_channel_sampler_float.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 0, "output": 1}"#.into(),
            r#"{"sampler": 0.0, "target": {"node": 0, "path": "translation"}}"#.into(),
        ),
        (
            "anim_path_number.glb",
            n3.into(),
            sk.into(),
            r#"{"input": 0, "output": 1}"#.into(),
            r#"{"sampler": 0, "target": {"node": 0, "path": 3}}"#.into(),
        ),
        (
            "anim_parent_cycle.glb",
            r#"{"children": [1]}, {"children": [0]}, {}"#.into(),
            sk.into(),
            r#"{"input": 0, "output": 1}"#.into(),
            ch(0, 1, "scale"),
        ),
    ];
    for (name, nodes, skins, samplers, channels) in cases {
        v.push((
            name.into(),
            anim_doc(&bin, views, accs, &nodes, &skins, &samplers, &channels),
        ));
    }
    // 130 joints in a chain: more than MAX_JOINTS (all loaded, with a warning).
    let nodes: Vec<String> = (0..130)
        .map(|i| {
            if i + 1 < 130 {
                format!(
                    "{{\"children\": [{}], \"translation\": [{i}, 0, 0]}}",
                    i + 1
                )
            } else {
                "{}".into()
            }
        })
        .collect();
    let joints: Vec<String> = (0..130).map(|i| i.to_string()).collect();
    v.push((
        "anim_skin_130.glb".into(),
        anim_doc(
            &bin,
            views,
            accs,
            &nodes.join(", "),
            &format!("{{\"joints\": [{}]}}", joints.join(", ")),
            r#"{"input": 0, "output": 1}"#,
            &ch(0, 129, "translation"),
        ),
    ));
    // Mesh reader rules: short normals, u8 normalized UVs, i16 UVs (Rust: unreachable!), no indices, u8 indices.
    let mut mb = f32_bytes(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]); // @0 (36)
    mb.extend(f32_bytes(&[0.0, 0.0, 1.0, 0.0, 1.0, 0.0])); // @36 two normals (24)
    mb.extend([0u8, 0, 255, 0, 128, 255, 0, 0]); // @60 u8 UVs (6, +2)
    mb.extend([0u8, 2, 1, 0]); // @68 u8 indices (3, +1)
    let mesh_doc = |prims: &str| -> Vec<u8> {
        let json = format!(
            r#"{{
  "asset": {{"version": "2.0"}},
  "buffers": [{{"byteLength": 72}}],
  "bufferViews": [{{"buffer": 0, "byteLength": 36}}, {{"buffer": 0, "byteOffset": 36, "byteLength": 24}},
    {{"buffer": 0, "byteOffset": 60, "byteLength": 6}}, {{"buffer": 0, "byteOffset": 68, "byteLength": 3}}],
  "accessors": [
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0, 0, 0], "max": [1, 0, 1]}},
    {{"bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC3"}},
    {{"bufferView": 2, "componentType": 5121, "normalized": true, "count": 3, "type": "VEC2"}},
    {{"bufferView": 3, "componentType": 5121, "count": 3, "type": "SCALAR"}},
    {{"bufferView": 2, "componentType": 5122, "count": 1, "type": "VEC2"}},
    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"}}
  ],
  "meshes": [{{"primitives": [{prims}]}}]
}}"#
        );
        glb(&json, &mb)
    };
    v.push(("mesh_reader_rules.glb".into(), mesh_doc(r#"{"attributes": {"POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2}, "indices": 3, "material": null}, {"attributes": {"POSITION": 0, "NORMAL": 5}}, {"attributes": {"POSITION": 0, "TEXCOORD_1": 4}, "mode": 4}"#)));
    v.push((
        "mesh_uv_i16.glb".into(),
        mesh_doc(r#"{"attributes": {"POSITION": 0, "TEXCOORD_0": 4}}"#),
    ));
    v.push((
        "mesh_uv_none_vec3.glb".into(),
        mesh_doc(r#"{"attributes": {"POSITION": 0, "TEXCOORD_0": 1}}"#),
    ));
}

fn fixtures() -> Vec<(String, Vec<u8>)> {
    let mut v = probe_fixtures();
    schema_fixtures(&mut v);
    v.push(("sparse.glb".into(), sparse_glb(0)));
    v.push(("sparse_index_past_view.glb".into(), sparse_glb(1)));
    v.push(("matrix_joints.glb".into(), matrix_joints_glb()));
    anim_fixtures(&mut v);
    // The Rust tests' own inputs.
    v.push(("empty.glb".into(), Vec::new()));
    let edge = mesh_edge_cases_glb();
    v.push(("trunc_8.glb".into(), edge[..8].to_vec()));
    let (rj, _) = rich_parts();
    v.push((
        "plain_trailing_ws.gltf".into(),
        format!("{rj}\n\t \r\n").into_bytes(),
    ));
    v.push((
        "plain_trailing_text.gltf".into(),
        format!("{rj} x").into_bytes(),
    ));
    // Strings serde_json refuses and bayan 1.11.1 takes (its open issue of 2026-11-11): a raw control byte, and bytes
    // that aren't UTF-8.
    let pos = r#""attributes": {"POSITION": 0}"#;
    v.push((
        "raw_control.glb".into(),
        tri_glb(
            &format!("{{{pos}}}"),
            "",
            "",
            ",\n  \"extensionsUsed\": [\"a\u{1}b\"]",
        ),
    ));
    let mut b = tri_glb(
        &format!("{{{pos}}}"),
        "",
        "",
        ",\n  \"extensionsUsed\": [\"a@@b\"]",
    );
    if let Some(at) = b.windows(2).position(|w| w == b"@@") {
        b[at] = 0xFF;
        b[at + 1] = 0xFE;
    }
    v.push(("bad_utf8.glb".into(), b));
    v
}

// ---------------------------------------------------------------------------
// The rows
// ---------------------------------------------------------------------------

fn panic_or<T>(
    r: std::thread::Result<soorat::error::Result<T>>,
    ok: impl FnOnce(T) -> Toks,
) -> (Toks, String) {
    match r {
        Ok(Ok(v)) => (ok(v), String::new()),
        Ok(Err(e)) => (Toks::new().err("Model"), format!(": {e}")),
        Err(p) => {
            let msg = if let Some(s) = p.downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = p.downcast_ref::<String>() {
                s.clone()
            } else {
                "<non-string panic>".into()
            };
            (Toks::new().s("panic"), format!(": Rust panics ({msg})"))
        }
    }
}

pub fn module() -> Module {
    let mut m = Module::new("gltf");
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    for (name, bytes) in fixtures() {
        let ins = Toks::new().s(&name);
        let (out, why) = panic_or(
            catch_unwind(AssertUnwindSafe(|| load_gltf_meshes(&bytes))),
            |(meshes, images)| {
                let mut t = Toks::new().i(meshes.len() as i128);
                for md in &meshes {
                    let f: Vec<f32> = md
                        .vertices
                        .iter()
                        .flat_map(|v| {
                            let mut a = v.position.to_vec();
                            a.extend(v.normal);
                            a.extend(v.tex_coords);
                            a.extend(v.color);
                            a
                        })
                        .collect();
                    let ix: Vec<i128> = md.indices.iter().map(|&x| x as i128).collect();
                    t = t.gs(&f).is(&ix);
                    t = match md.material_index {
                        Some(k) => t.i(k as i128),
                        None => t.none(),
                    };
                }
                t = t.i(images.len() as i128);
                for im in &images {
                    let b: Vec<i128> = im.iter().map(|&x| x as i128).collect();
                    t = t.is(&b);
                }
                t
            },
        );
        m.row("load_gltf_meshes", 'A', &ins, &out, &format!("{name}{why}"));
        let (out, why) = panic_or(
            catch_unwind(AssertUnwindSafe(|| load_gltf_animations(&bytes))),
            |(skels, clips)| {
                let mut t = Toks::new().i(skels.len() as i128);
                for sk in &skels {
                    t = t.i(sk.joints.len() as i128);
                    for j in &sk.joints {
                        t = t
                            .i(j.parent as i128)
                            .gs(&j.inverse_bind)
                            .gs(&j.translation)
                            .gs(&j.rotation)
                            .gs(&j.scale);
                    }
                }
                t = t.i(clips.len() as i128);
                for c in &clips {
                    t = t.s(&c.name).g(c.duration).i(c.channels.len() as i128);
                    for ch in &c.channels {
                        let p = match ch.property {
                            AnimationProperty::Translation => 0,
                            AnimationProperty::Rotation => 1,
                            AnimationProperty::Scale => 2,
                            _ => 9,
                        };
                        t = t
                            .i(ch.joint_index as i128)
                            .i(p)
                            .i(ch.keyframes.len() as i128);
                        for k in &ch.keyframes {
                            t = t.g(k.time).gs(&k.value);
                        }
                    }
                }
                t
            },
        );
        m.row(
            "load_gltf_animations",
            'A',
            &ins,
            &out,
            &format!("{name}{why}"),
        );
        m.files.push((name, bytes));
    }
    std::panic::set_hook(prev);
    let mut list = String::new();
    for (n, _) in &m.files {
        let _ = writeln!(list, "{n}");
    }
    m.files.push(("LIST".into(), list.into_bytes()));
    m
}
