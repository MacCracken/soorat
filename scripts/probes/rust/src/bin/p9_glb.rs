//! P9: GLB fixtures for the bayan probe, and what soorat 1.2.0's public loaders make of them.
//!
//! Writes `../assets/fixtures/p9/<name>.glb` (and one `.gltf`), then dumps
//! `soorat::gltf_loader::load_gltf_meshes` and `soorat::animation::load_gltf_animations`
//! for each into `../assets/out/p9/rust/<name>.txt`. The Cyrius side
//! (`assets/tests/p9_glb.tcyr`) writes the same format into `out/p9/cyr/`; lines starting
//! with `#` are commentary and are not compared.
//!
//! The two builders are copied verbatim from soorat's `src/test_support.rs` (`#[cfg(test)]`,
//! unreachable from outside the crate).
use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use soorat::animation::{AnimationProperty, load_gltf_animations};
use soorat::gltf_loader::load_gltf_meshes;

// ---------------------------------------------------------------------------
// Copied from soorat 1.2.0 src/test_support.rs
// ---------------------------------------------------------------------------

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

fn fixtures() -> Vec<(String, Vec<u8>)> {
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
// The dump
// ---------------------------------------------------------------------------

fn hx(v: f32) -> String {
    format!("{:08x}", v.to_bits())
}

fn hxs(v: &[f32]) -> String {
    v.iter().map(|x| hx(*x)).collect::<Vec<_>>().join(" ")
}

fn panic_msg(e: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = e.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = e.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic>".into()
    }
}

fn dump(bytes: &[u8]) -> String {
    let mut o = String::new();
    match catch_unwind(AssertUnwindSafe(|| load_gltf_meshes(bytes))) {
        Ok(Ok((meshes, images))) => {
            let _ = writeln!(o, "meshes ok");
            let _ = writeln!(o, "mesh_count {}", meshes.len());
            for (i, m) in meshes.iter().enumerate() {
                let mat = m
                    .material_index
                    .map_or_else(|| "none".to_string(), |x| x.to_string());
                let _ = writeln!(
                    o,
                    "mesh {i} verts {} indices {} material {mat}",
                    m.vertices.len(),
                    m.indices.len()
                );
                for (j, v) in m.vertices.iter().enumerate() {
                    let _ = writeln!(
                        o,
                        "v {j} {} {} {}",
                        hxs(&v.position),
                        hxs(&v.normal),
                        hxs(&v.tex_coords)
                    );
                }
                let idx: Vec<String> = m.indices.iter().map(u32::to_string).collect();
                let _ = writeln!(o, "i {}", idx.join(" "));
            }
            let _ = writeln!(o, "image_count {}", images.len());
            for (i, im) in images.iter().enumerate() {
                let _ = writeln!(o, "image {i} len {}", im.len());
            }
        }
        Ok(Err(e)) => {
            let _ = writeln!(o, "meshes err");
            let _ = writeln!(o, "# {e}");
        }
        Err(p) => {
            let _ = writeln!(o, "meshes panic");
            let _ = writeln!(o, "# {}", panic_msg(&*p));
        }
    }
    match catch_unwind(AssertUnwindSafe(|| load_gltf_animations(bytes))) {
        Ok(Ok((skels, clips))) => {
            let _ = writeln!(o, "anim ok");
            let _ = writeln!(o, "skel_count {}", skels.len());
            for (s, sk) in skels.iter().enumerate() {
                let _ = writeln!(o, "skel {s} joints {}", sk.joints.len());
                for (j, jt) in sk.joints.iter().enumerate() {
                    let _ = writeln!(
                        o,
                        "joint {j} parent {} t {} r {} s {}",
                        jt.parent,
                        hxs(&jt.translation),
                        hxs(&jt.rotation),
                        hxs(&jt.scale)
                    );
                    let _ = writeln!(o, "ib {j} {}", hxs(&jt.inverse_bind));
                }
            }
            let _ = writeln!(o, "clip_count {}", clips.len());
            for (c, cl) in clips.iter().enumerate() {
                let _ = writeln!(
                    o,
                    "clip {c} name {} duration {} channels {}",
                    cl.name,
                    hx(cl.duration),
                    cl.channels.len()
                );
                for (k, ch) in cl.channels.iter().enumerate() {
                    let p = match ch.property {
                        AnimationProperty::Translation => "T",
                        AnimationProperty::Rotation => "R",
                        AnimationProperty::Scale => "S",
                        _ => "?",
                    };
                    let _ = writeln!(
                        o,
                        "chan {k} joint {} prop {p} keys {}",
                        ch.joint_index,
                        ch.keyframes.len()
                    );
                    for kf in &ch.keyframes {
                        let _ = writeln!(o, "key {} {}", hx(kf.time), hxs(&kf.value));
                    }
                }
            }
        }
        Ok(Err(e)) => {
            let _ = writeln!(o, "anim err");
            let _ = writeln!(o, "# {e}");
        }
        Err(p) => {
            let _ = writeln!(o, "anim panic");
            let _ = writeln!(o, "# {}", panic_msg(&*p));
        }
    }
    o
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets");
    let fx = root.join("fixtures/p9");
    // P9_OUT redirects the dumps (used for the overflow-checks-off build); fixtures are unchanged.
    let out = std::env::var_os("P9_OUT").map_or_else(|| root.join("out/p9/rust"), PathBuf::from);
    std::fs::create_dir_all(&fx).expect("mkdir fixtures");
    std::fs::create_dir_all(&out).expect("mkdir out");
    let mut list = String::new();
    for (name, bytes) in fixtures() {
        std::fs::write(fx.join(&name), &bytes).expect("write fixture");
        let d = dump(&bytes);
        let stem = name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s);
        std::fs::write(out.join(format!("{stem}.txt")), &d).expect("write dump");
        let verdict: Vec<&str> = d
            .lines()
            .filter(|l| l.starts_with("meshes ") || l.starts_with("anim "))
            .collect();
        println!("{name:28} {:6} bytes  {}", bytes.len(), verdict.join(" | "));
        let _ = writeln!(list, "{name}");
    }
    std::fs::write(fx.join("LIST"), list).expect("write list");
}
