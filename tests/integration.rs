//! Integration tests for soorat.

use soorat::WindowConfig;
use soorat::color::Color;
use soorat::sprite::{Sprite, SpriteBatch};
use soorat::vertex::{Vertex2D, Vertex3D};

#[test]
fn sprite_workflow() {
    let mut batch = SpriteBatch::new();

    // Build sprites with different z-orders
    for i in 0..10 {
        let sprite = Sprite::new(i as f32 * 50.0, 100.0, 32.0, 32.0)
            .with_color(Color::from_rgba8(255, i * 25, 0, 255))
            .with_z_order(10 - i as i32);
        batch.push(sprite);
    }

    assert_eq!(batch.len(), 10);

    // Sort by z-order
    batch.sort_by_z();
    assert_eq!(batch.sprites[0].z_order, 1);
    assert_eq!(batch.sprites[9].z_order, 10);
}

#[test]
fn color_conversions() {
    let hex = Color::from_hex(0x336699FF);
    let arr: [f32; 4] = hex.into();
    let back: Color = arr.into();
    assert!((hex.r - back.r).abs() < 0.01);
    assert!((hex.g - back.g).abs() < 0.01);
    assert!((hex.b - back.b).abs() < 0.01);
}

#[test]
fn vertex_sizes() {
    assert_eq!(std::mem::size_of::<Vertex2D>(), 32);
    assert_eq!(std::mem::size_of::<Vertex3D>(), 48);
}

#[test]
fn window_config_integration() {
    let cfg = WindowConfig::new("Test Window", 800, 600);
    assert!((cfg.aspect_ratio() - 800.0 / 600.0).abs() < 0.01);
    assert_eq!(cfg.present_mode(), wgpu::PresentMode::AutoVsync);
}
