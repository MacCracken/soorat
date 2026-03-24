//! Sprite rendering types.

use crate::color::Color;
use serde::{Deserialize, Serialize};

/// UV rectangle defining a sub-region of a texture (for sprite atlases).
/// Values are in normalized texture coordinates (0.0–1.0).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UvRect {
    pub u_min: f32,
    pub v_min: f32,
    pub u_max: f32,
    pub v_max: f32,
}

impl Default for UvRect {
    fn default() -> Self {
        Self::FULL
    }
}

impl UvRect {
    /// Full texture (0,0 to 1,1).
    pub const FULL: Self = Self {
        u_min: 0.0,
        v_min: 0.0,
        u_max: 1.0,
        v_max: 1.0,
    };

    /// Create a UV rect from pixel coordinates and atlas dimensions.
    pub fn from_pixel_rect(x: u32, y: u32, w: u32, h: u32, atlas_w: u32, atlas_h: u32) -> Self {
        Self {
            u_min: x as f32 / atlas_w as f32,
            v_min: y as f32 / atlas_h as f32,
            u_max: (x + w) as f32 / atlas_w as f32,
            v_max: (y + h) as f32 / atlas_h as f32,
        }
    }

    /// UV coordinates for quad corners: [top-left, top-right, bottom-right, bottom-left].
    pub fn corners(&self) -> [[f32; 2]; 4] {
        [
            [self.u_min, self.v_min],
            [self.u_max, self.v_min],
            [self.u_max, self.v_max],
            [self.u_min, self.v_max],
        ]
    }
}

/// A 2D sprite instance to render.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sprite {
    /// Position in screen/world coordinates.
    pub x: f32,
    pub y: f32,
    /// Size in pixels.
    pub width: f32,
    pub height: f32,
    /// Rotation in radians.
    pub rotation: f32,
    /// Tint color (multiplied with texture).
    pub color: Color,
    /// Texture ID (0 = white pixel / no texture).
    pub texture_id: u64,
    /// Z-order for sorting (higher = in front).
    pub z_order: i32,
    /// UV region within the texture (for sprite atlases).
    pub uv: UvRect,
}

impl Default for Sprite {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 64.0,
            height: 64.0,
            rotation: 0.0,
            color: Color::WHITE,
            texture_id: 0,
            z_order: 0,
            uv: UvRect::FULL,
        }
    }
}

impl Sprite {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            ..Default::default()
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_rotation(mut self, radians: f32) -> Self {
        self.rotation = radians;
        self
    }

    pub fn with_texture(mut self, id: u64) -> Self {
        self.texture_id = id;
        self
    }

    pub fn with_z_order(mut self, z: i32) -> Self {
        self.z_order = z;
        self
    }

    pub fn with_uv(mut self, uv: UvRect) -> Self {
        self.uv = uv;
        self
    }

    /// Center position.
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Bounding box: (min_x, min_y, max_x, max_y).
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.x + self.width, self.y + self.height)
    }
}

/// A batch of sprites to render together.
/// Supports mixed textures via `SpritePipeline::draw_batched()`.
#[derive(Debug, Clone, Default)]
pub struct SpriteBatch {
    pub sprites: Vec<Sprite>,
}

impl SpriteBatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            sprites: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, sprite: Sprite) {
        self.sprites.push(sprite);
    }

    pub fn clear(&mut self) {
        self.sprites.clear();
    }

    pub fn len(&self) -> usize {
        self.sprites.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty()
    }

    /// Sort sprites by z-order for correct rendering.
    pub fn sort_by_z(&mut self) {
        self.sprites.sort_by_key(|s| s.z_order);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv_rect_default() {
        let uv = UvRect::default();
        assert_eq!(uv, UvRect::FULL);
        assert_eq!(
            uv.corners(),
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
        );
    }

    #[test]
    fn uv_rect_from_pixel_rect() {
        let uv = UvRect::from_pixel_rect(64, 0, 32, 32, 256, 256);
        assert!((uv.u_min - 0.25).abs() < f32::EPSILON);
        assert_eq!(uv.v_min, 0.0);
        assert!((uv.u_max - 0.375).abs() < f32::EPSILON);
        assert!((uv.v_max - 0.125).abs() < f32::EPSILON);
    }

    #[test]
    fn uv_rect_serde() {
        let uv = UvRect::from_pixel_rect(10, 20, 30, 40, 100, 100);
        let json = serde_json::to_string(&uv).unwrap();
        let decoded: UvRect = serde_json::from_str(&json).unwrap();
        assert_eq!(uv, decoded);
    }

    #[test]
    fn sprite_with_uv() {
        let uv = UvRect::from_pixel_rect(0, 0, 16, 16, 64, 64);
        let s = Sprite::new(0.0, 0.0, 32.0, 32.0).with_uv(uv);
        assert_eq!(s.uv, uv);
    }

    #[test]
    fn sprite_default() {
        let s = Sprite::default();
        assert_eq!(s.x, 0.0);
        assert_eq!(s.width, 64.0);
        assert_eq!(s.color, Color::WHITE);
        assert_eq!(s.z_order, 0);
        assert_eq!(s.uv, UvRect::FULL);
    }

    #[test]
    fn sprite_builder() {
        let s = Sprite::new(10.0, 20.0, 32.0, 32.0)
            .with_color(Color::RED)
            .with_rotation(1.57)
            .with_texture(42)
            .with_z_order(5);
        assert_eq!(s.x, 10.0);
        assert_eq!(s.color, Color::RED);
        assert_eq!(s.rotation, 1.57);
        assert_eq!(s.texture_id, 42);
        assert_eq!(s.z_order, 5);
    }

    #[test]
    fn sprite_center() {
        let s = Sprite::new(100.0, 200.0, 50.0, 30.0);
        assert_eq!(s.center(), (125.0, 215.0));
    }

    #[test]
    fn sprite_bounds() {
        let s = Sprite::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(s.bounds(), (10.0, 20.0, 110.0, 70.0));
    }

    #[test]
    fn sprite_serde() {
        let s = Sprite::new(1.0, 2.0, 3.0, 4.0).with_color(Color::BLUE);
        let json = serde_json::to_string(&s).unwrap();
        let decoded: Sprite = serde_json::from_str(&json).unwrap();
        assert_eq!(s, decoded);
    }

    #[test]
    fn sprite_batch() {
        let mut batch = SpriteBatch::new();
        assert!(batch.is_empty());

        batch.push(Sprite::new(0.0, 0.0, 10.0, 10.0).with_z_order(2));
        batch.push(Sprite::new(0.0, 0.0, 10.0, 10.0).with_z_order(0));
        batch.push(Sprite::new(0.0, 0.0, 10.0, 10.0).with_z_order(1));
        assert_eq!(batch.len(), 3);

        batch.sort_by_z();
        assert_eq!(batch.sprites[0].z_order, 0);
        assert_eq!(batch.sprites[1].z_order, 1);
        assert_eq!(batch.sprites[2].z_order, 2);

        batch.clear();
        assert!(batch.is_empty());
    }

    #[test]
    fn sprite_negative_dimensions() {
        // Negative dimensions are allowed (flip rendering)
        let s = Sprite::new(10.0, 20.0, -32.0, -16.0);
        assert_eq!(s.width, -32.0);
        let (cx, cy) = s.center();
        assert_eq!(cx, 10.0 + (-32.0) / 2.0);
        assert_eq!(cy, 20.0 + (-16.0) / 2.0);
    }

    #[test]
    fn batch_sort_stability() {
        let mut batch = SpriteBatch::new();
        batch.push(Sprite::new(1.0, 0.0, 10.0, 10.0).with_z_order(0));
        batch.push(Sprite::new(2.0, 0.0, 10.0, 10.0).with_z_order(0));
        batch.push(Sprite::new(3.0, 0.0, 10.0, 10.0).with_z_order(0));
        batch.sort_by_z();
        // Stable sort should preserve insertion order for equal z
        assert_eq!(batch.sprites[0].x, 1.0);
        assert_eq!(batch.sprites[1].x, 2.0);
        assert_eq!(batch.sprites[2].x, 3.0);
    }

    #[test]
    fn batch_with_capacity() {
        let batch = SpriteBatch::with_capacity(100);
        assert!(batch.is_empty());
        assert!(batch.sprites.capacity() >= 100);
    }

    #[test]
    fn batch_1000_sprites() {
        let mut batch = SpriteBatch::new();
        for i in 0..1000 {
            batch.push(Sprite::new(i as f32, 0.0, 10.0, 10.0).with_z_order(1000 - i));
        }
        assert_eq!(batch.len(), 1000);
        batch.sort_by_z();
        assert_eq!(batch.sprites[0].z_order, 1);
        assert_eq!(batch.sprites[999].z_order, 1000);
    }
}
