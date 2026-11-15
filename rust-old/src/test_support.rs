//! Test-only helpers: in-memory GLB construction for loader tests.

/// Little-endian bytes of a slice of `f32`.
pub(crate) fn f32_bytes(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// Little-endian bytes of a slice of `u16`.
pub(crate) fn u16_bytes(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// Assemble a binary glTF (GLB v2) from a JSON document and a BIN chunk.
/// Chunks are padded to 4-byte alignment as the spec requires.
pub(crate) fn glb(json: &str, bin: &[u8]) -> Vec<u8> {
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

/// GLB with a single skin + animation exercising the loader edge cases:
///
/// - nodes: 0 = parent joint (children `[1]`), 1 = child joint, 2 = non-joint
/// - skin joints listed child-first (`[1, 0]`) — not topologically sorted
/// - channel 0 targets node 2 (must be skipped)
/// - channel 1: node 0 rotation, `STEP`, keys at t = 0 and 1
/// - channel 2: node 1 translation, `CUBICSPLINE`, values (1,0,0) and (2,0,0)
pub(crate) fn skinned_animation_glb() -> Vec<u8> {
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

/// GLB with one mesh and three primitives over the same 3-vertex triangle:
/// 0 = valid triangle list, 1 = triangle list whose indices reference vertex 5
/// (out of range), 2 = `LINES` mode. Plus one image whose buffer view lies
/// outside the BIN chunk.
pub(crate) fn mesh_edge_cases_glb() -> Vec<u8> {
    let mut bin = f32_bytes(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]); // positions @0 (36)
    bin.extend(u16_bytes(&[0, 2, 1])); // good indices @36 (6)
    bin.extend(u16_bytes(&[0])); // pad to 4-byte alignment @42 (2) — unused
    bin.extend(u16_bytes(&[0, 1, 5])); // bad indices @44 (6)
    let json = format!(
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
}}"#,
        len = bin.len()
    );
    glb(&json, &bin)
}
