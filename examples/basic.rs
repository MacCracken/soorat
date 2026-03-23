//! Basic soorat usage — create sprites and a window config.

use soorat::WindowConfig;
use soorat::color::Color;
use soorat::sprite::{Sprite, SpriteBatch};

fn main() {
    let config = WindowConfig::new("Soorat Example", 1280, 720);
    println!(
        "Window: {}x{} ({})",
        config.width, config.height, config.title
    );
    println!("Aspect ratio: {:.2}", config.aspect_ratio());
    println!("VSync: {}", config.vsync);

    let mut batch = SpriteBatch::new();

    // Create a grid of colored sprites
    for row in 0..5 {
        for col in 0..5 {
            let t = (row * 5 + col) as f32 / 25.0;
            let color = Color::RED.lerp(Color::BLUE, t);
            let sprite = Sprite::new(col as f32 * 70.0, row as f32 * 70.0, 64.0, 64.0)
                .with_color(color)
                .with_z_order(row);
            batch.push(sprite);
        }
    }

    println!("Batch: {} sprites", batch.len());
    batch.sort_by_z();
    println!("Sorted by z-order.");
}
