//! Sprite batch → vertex/index generation.
//!
//! Pure CPU functions for converting `SpriteBatch` into GPU-ready vertex and index data.
//! Supports u16 (16383 sprite limit) and u32 (unlimited) index paths.

use crate::sprite::SpriteBatch;
use crate::vertex::Vertex2D;

/// Quad indices for a single sprite (two triangles).
const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];
const QUAD_INDICES_U32: [u32; 6] = [0, 1, 2, 2, 3, 0];

/// Maximum sprites per batch with u16 indices (65535 / 4 = 16383).
pub const MAX_SPRITES_PER_BATCH: usize = 16383;

/// Expand a sprite batch into vertex and index data for GPU upload.
///
/// **Limit**: u16 indices support up to 16383 sprites per batch.
/// Batches exceeding this will be truncated. Use multiple draw calls for larger scenes.
#[must_use]
pub fn batch_to_vertices(batch: &SpriteBatch) -> (Vec<Vertex2D>, Vec<u16>) {
    let sprite_count = batch.sprites.len();
    let vert_cap = match sprite_count.checked_mul(4) {
        Some(v) => v,
        None => return (Vec::new(), Vec::new()),
    };
    let idx_cap = match sprite_count.checked_mul(6) {
        Some(v) => v,
        None => return (Vec::new(), Vec::new()),
    };
    let mut vertices = Vec::with_capacity(vert_cap);
    let mut indices = Vec::with_capacity(idx_cap);
    batch_to_vertices_into(batch, &mut vertices, &mut indices);
    (vertices, indices)
}

/// Expand a sprite batch into pre-allocated vertex and index buffers.
/// Clears and fills the provided buffers. Use this to avoid allocations in game loops.
pub fn batch_to_vertices_into(
    batch: &SpriteBatch,
    vertices: &mut Vec<Vertex2D>,
    indices: &mut Vec<u16>,
) {
    let sprite_count = batch.sprites.len().min(MAX_SPRITES_PER_BATCH);
    vertices.clear();
    vertices.reserve(sprite_count.saturating_mul(4));
    indices.clear();
    indices.reserve(sprite_count.saturating_mul(6));

    for (i, sprite) in batch.sprites.iter().take(sprite_count).enumerate() {
        emit_quad_u16(sprite, i, vertices, indices);
    }
}

/// Expand a sprite batch into vertex and u32 index data. No sprite count limit.
#[must_use]
pub fn batch_to_vertices_u32(batch: &SpriteBatch) -> (Vec<Vertex2D>, Vec<u32>) {
    let sprite_count = batch.sprites.len();
    let vert_cap = match sprite_count.checked_mul(4) {
        Some(v) => v,
        None => return (Vec::new(), Vec::new()),
    };
    let idx_cap = match sprite_count.checked_mul(6) {
        Some(v) => v,
        None => return (Vec::new(), Vec::new()),
    };
    let mut vertices = Vec::with_capacity(vert_cap);
    let mut indices = Vec::with_capacity(idx_cap);
    batch_to_vertices_u32_into(batch, &mut vertices, &mut indices);
    (vertices, indices)
}

/// Expand a sprite batch into pre-allocated u32 index buffers. No sprite count limit.
pub fn batch_to_vertices_u32_into(
    batch: &SpriteBatch,
    vertices: &mut Vec<Vertex2D>,
    indices: &mut Vec<u32>,
) {
    let sprite_count = batch.sprites.len();
    vertices.clear();
    vertices.reserve(sprite_count.saturating_mul(4));
    indices.clear();
    indices.reserve(sprite_count.saturating_mul(6));

    for (i, sprite) in batch.sprites.iter().enumerate() {
        emit_quad_u32(sprite, i, vertices, indices);
    }
}

#[inline]
fn emit_quad_u16(
    sprite: &crate::sprite::Sprite,
    i: usize,
    vertices: &mut Vec<Vertex2D>,
    indices: &mut Vec<u16>,
) {
    let c = sprite.color.to_array();
    debug_assert!(i < 16384, "u16 index overflow: sprite index {i} >= 16384");
    let base = (i * 4) as u16;
    let (positions, uvs) = sprite_quad(sprite);

    for j in 0..4 {
        vertices.push(Vertex2D {
            position: positions[j],
            tex_coords: uvs[j],
            color: c,
        });
    }
    for &idx in &QUAD_INDICES {
        indices.push(base + idx);
    }
}

#[inline]
fn emit_quad_u32(
    sprite: &crate::sprite::Sprite,
    i: usize,
    vertices: &mut Vec<Vertex2D>,
    indices: &mut Vec<u32>,
) {
    let c = sprite.color.to_array();
    let Some(base) = i.checked_mul(4).and_then(|v| u32::try_from(v).ok()) else {
        return;
    };
    let (positions, uvs) = sprite_quad(sprite);

    for j in 0..4 {
        vertices.push(Vertex2D {
            position: positions[j],
            tex_coords: uvs[j],
            color: c,
        });
    }
    for &idx in &QUAD_INDICES_U32 {
        indices.push(base + idx);
    }
}

/// Compute rotated quad positions and UV coordinates for a sprite.
#[inline]
fn sprite_quad(sprite: &crate::sprite::Sprite) -> ([[f32; 2]; 4], [[f32; 2]; 4]) {
    let cx = sprite.x + sprite.width * 0.5;
    let cy = sprite.y + sprite.height * 0.5;
    let hw = sprite.width * 0.5;
    let hh = sprite.height * 0.5;

    let corners = [[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]];

    let (sin, cos) = if sprite.rotation != 0.0 {
        (sprite.rotation.sin(), sprite.rotation.cos())
    } else {
        (0.0, 1.0)
    };

    let uvs = sprite.uv.corners();
    let mut positions = [[0.0f32; 2]; 4];
    for (j, corner) in corners.iter().enumerate() {
        positions[j] = [
            corner[0] * cos - corner[1] * sin + cx,
            corner[0] * sin + corner[1] * cos + cy,
        ];
    }

    (positions, uvs)
}
