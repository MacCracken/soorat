//! UI rendering — screen-space panels, labels, and HUD elements.

use crate::color::Color;
use crate::sprite::{Sprite, SpriteBatch};
use crate::text::{BitmapFont, TextBatch};

/// A UI panel — a colored rectangle in screen space.
#[derive(Debug, Clone)]
pub struct UiPanel {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Color,
    pub texture_id: u64,
    pub z_order: i32,
}

impl UiPanel {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            color: Color::WHITE,
            texture_id: 0,
            z_order: 0,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
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
}

/// A UI label — text positioned in screen space.
#[derive(Debug, Clone)]
pub struct UiLabel {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub color: Color,
    pub z_order: i32,
}

impl UiLabel {
    pub fn new(text: impl Into<String>, x: f32, y: f32) -> Self {
        Self {
            text: text.into(),
            x,
            y,
            scale: 1.0,
            color: Color::WHITE,
            z_order: 0,
        }
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_z_order(mut self, z: i32) -> Self {
        self.z_order = z;
        self
    }
}

/// UI batch — accumulates panels and labels for rendering.
/// Renders on top of the scene via SpritePipeline (no depth test).
pub struct UiBatch {
    pub panels: SpriteBatch,
    pub text: TextBatch,
}

impl UiBatch {
    pub fn new() -> Self {
        Self {
            panels: SpriteBatch::new(),
            text: TextBatch::new(),
        }
    }

    /// Add a panel to the UI.
    pub fn add_panel(&mut self, panel: &UiPanel) {
        self.panels.push(
            Sprite::new(panel.x, panel.y, panel.width, panel.height)
                .with_color(panel.color)
                .with_texture(panel.texture_id)
                .with_z_order(panel.z_order),
        );
    }

    /// Add a text label to the UI.
    pub fn add_label(&mut self, font: &BitmapFont, label: &UiLabel) {
        self.text.draw_text_z(
            font,
            &label.text,
            label.x,
            label.y,
            label.scale,
            label.color,
            label.z_order,
        );
    }

    pub fn clear(&mut self) {
        self.panels.clear();
        self.text.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.panels.is_empty() && self.text.is_empty()
    }
}

impl Default for UiBatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_panel_builder() {
        let p = UiPanel::new(10.0, 20.0, 200.0, 50.0)
            .with_color(Color::BLACK)
            .with_z_order(100);
        assert_eq!(p.x, 10.0);
        assert_eq!(p.color, Color::BLACK);
        assert_eq!(p.z_order, 100);
    }

    #[test]
    fn ui_label_builder() {
        let l = UiLabel::new("Score: 42", 10.0, 10.0)
            .with_scale(2.0)
            .with_color(Color::GREEN);
        assert_eq!(l.text, "Score: 42");
        assert_eq!(l.scale, 2.0);
    }

    #[test]
    fn ui_batch_empty() {
        let batch = UiBatch::new();
        assert!(batch.is_empty());
    }

    #[test]
    fn ui_batch_add_panel() {
        let mut batch = UiBatch::new();
        batch.add_panel(&UiPanel::new(0.0, 0.0, 100.0, 50.0));
        assert!(!batch.is_empty());
        assert_eq!(batch.panels.len(), 1);
    }

    #[test]
    fn ui_batch_add_label() {
        let font = BitmapFont {
            texture_id: 1,
            glyph_width: 8,
            glyph_height: 16,
            columns: 16,
            atlas_width: 128,
            atlas_height: 128,
            first_char: 32,
            glyph_count: 96,
        };
        let mut batch = UiBatch::new();
        batch.add_label(&font, &UiLabel::new("Hi", 0.0, 0.0));
        assert_eq!(batch.text.len(), 2);
    }

    #[test]
    fn ui_batch_clear() {
        let mut batch = UiBatch::new();
        batch.add_panel(&UiPanel::new(0.0, 0.0, 100.0, 50.0));
        batch.clear();
        assert!(batch.is_empty());
    }
}
