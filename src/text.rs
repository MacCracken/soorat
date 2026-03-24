//! Text rendering — bitmap font glyph atlas + text batching.

use crate::color::Color;
use crate::sprite::{Sprite, SpriteBatch, UvRect};

/// A bitmap font glyph atlas — fixed-width monospace.
#[derive(Debug, Clone)]
pub struct BitmapFont {
    /// Texture ID for the glyph atlas in TextureCache.
    pub texture_id: u64,
    /// Width of each glyph cell in the atlas (pixels).
    pub glyph_width: u32,
    /// Height of each glyph cell in the atlas (pixels).
    pub glyph_height: u32,
    /// Number of columns in the atlas grid.
    pub columns: u32,
    /// Atlas texture width.
    pub atlas_width: u32,
    /// Atlas texture height.
    pub atlas_height: u32,
    /// First ASCII code in the atlas (typically 32 = space).
    pub first_char: u8,
    /// Number of glyphs in the atlas.
    pub glyph_count: u32,
}

impl BitmapFont {
    /// Get the UV rect for a character.
    pub fn glyph_uv(&self, ch: char) -> UvRect {
        let code = ch as u32;
        let index =
            if code >= self.first_char as u32 && code < self.first_char as u32 + self.glyph_count {
                code - self.first_char as u32
            } else {
                0 // fallback to first glyph
            };

        let col = index % self.columns;
        let row = index / self.columns;
        let x = col * self.glyph_width;
        let y = row * self.glyph_height;

        UvRect::from_pixel_rect(
            x,
            y,
            self.glyph_width,
            self.glyph_height,
            self.atlas_width,
            self.atlas_height,
        )
    }
}

/// A batch of text to render as textured sprite quads.
pub struct TextBatch {
    pub batch: SpriteBatch,
}

impl TextBatch {
    pub fn new() -> Self {
        Self {
            batch: SpriteBatch::new(),
        }
    }

    pub fn with_capacity(chars: usize) -> Self {
        Self {
            batch: SpriteBatch::with_capacity(chars),
        }
    }

    /// Add a string of text at a screen position.
    /// `scale`: multiplier on glyph size (1.0 = native atlas size).
    pub fn draw_text(
        &mut self,
        font: &BitmapFont,
        text: &str,
        x: f32,
        y: f32,
        scale: f32,
        color: Color,
    ) {
        let char_w = font.glyph_width as f32 * scale;
        let char_h = font.glyph_height as f32 * scale;

        for (i, ch) in text.chars().enumerate() {
            if ch == ' ' {
                continue; // skip spaces (transparent anyway)
            }

            let uv = font.glyph_uv(ch);
            let sprite = Sprite::new(x + i as f32 * char_w, y, char_w, char_h)
                .with_color(color)
                .with_texture(font.texture_id)
                .with_uv(uv);
            self.batch.push(sprite);
        }
    }

    /// Add text with a z-order (for layering with other sprites).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text_z(
        &mut self,
        font: &BitmapFont,
        text: &str,
        x: f32,
        y: f32,
        scale: f32,
        color: Color,
        z_order: i32,
    ) {
        let char_w = font.glyph_width as f32 * scale;
        let char_h = font.glyph_height as f32 * scale;

        for (i, ch) in text.chars().enumerate() {
            if ch == ' ' {
                continue;
            }

            let uv = font.glyph_uv(ch);
            let sprite = Sprite::new(x + i as f32 * char_w, y, char_w, char_h)
                .with_color(color)
                .with_texture(font.texture_id)
                .with_uv(uv)
                .with_z_order(z_order);
            self.batch.push(sprite);
        }
    }

    pub fn clear(&mut self) {
        self.batch.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.batch.is_empty()
    }

    pub fn len(&self) -> usize {
        self.batch.len()
    }
}

impl Default for TextBatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_font() -> BitmapFont {
        BitmapFont {
            texture_id: 1,
            glyph_width: 8,
            glyph_height: 16,
            columns: 16,
            atlas_width: 128,
            atlas_height: 128,
            first_char: 32,
            glyph_count: 96,
        }
    }

    #[test]
    fn glyph_uv_space() {
        let font = test_font();
        let uv = font.glyph_uv(' '); // char 32 = index 0
        assert_eq!(uv.u_min, 0.0);
        assert_eq!(uv.v_min, 0.0);
    }

    #[test]
    fn glyph_uv_a() {
        let font = test_font();
        let uv = font.glyph_uv('A'); // char 65, index 33
        // col = 33 % 16 = 1, row = 33 / 16 = 2
        let expected_u = 1.0 * 8.0 / 128.0;
        let expected_v = 2.0 * 16.0 / 128.0;
        assert!((uv.u_min - expected_u).abs() < 0.001);
        assert!((uv.v_min - expected_v).abs() < 0.001);
    }

    #[test]
    fn glyph_uv_out_of_range_falls_back() {
        let font = test_font();
        let uv = font.glyph_uv('\u{200}'); // way outside ASCII
        assert_eq!(uv.u_min, 0.0); // fallback to first glyph
        assert_eq!(uv.v_min, 0.0);
    }

    #[test]
    fn text_batch_draw() {
        let font = test_font();
        let mut tb = TextBatch::new();
        tb.draw_text(&font, "Hello", 10.0, 20.0, 1.0, Color::WHITE);
        // "Hello" = 5 chars, no spaces
        assert_eq!(tb.len(), 5);
    }

    #[test]
    fn text_batch_skips_spaces() {
        let font = test_font();
        let mut tb = TextBatch::new();
        tb.draw_text(&font, "A B", 0.0, 0.0, 1.0, Color::WHITE);
        assert_eq!(tb.len(), 2); // 'A' and 'B', space skipped
    }

    #[test]
    fn text_batch_empty() {
        let tb = TextBatch::new();
        assert!(tb.is_empty());
        assert_eq!(tb.len(), 0);
    }

    #[test]
    fn text_batch_clear() {
        let font = test_font();
        let mut tb = TextBatch::new();
        tb.draw_text(&font, "test", 0.0, 0.0, 1.0, Color::WHITE);
        tb.clear();
        assert!(tb.is_empty());
    }

    #[test]
    fn text_batch_scale() {
        let font = test_font();
        let mut tb = TextBatch::new();
        tb.draw_text(&font, "X", 0.0, 0.0, 2.0, Color::WHITE);
        let sprite = &tb.batch.sprites[0];
        assert_eq!(sprite.width, 16.0); // 8 * 2.0
        assert_eq!(sprite.height, 32.0); // 16 * 2.0
    }

    #[test]
    fn text_batch_positioning() {
        let font = test_font();
        let mut tb = TextBatch::new();
        tb.draw_text(&font, "AB", 100.0, 50.0, 1.0, Color::WHITE);
        assert_eq!(tb.batch.sprites[0].x, 100.0);
        assert_eq!(tb.batch.sprites[1].x, 108.0); // 100 + 8
        assert_eq!(tb.batch.sprites[0].y, 50.0);
    }
}
