//! Basic soorat usage — open a window and render colored sprites.

use soorat::color::Color;
use soorat::pipeline::SpritePipeline;
use soorat::sprite::{Sprite, SpriteBatch};
use soorat::texture::Texture;
use soorat::window::{WindowConfig, run};

fn main() {
    let config = WindowConfig::new("Soorat — Sprite Demo", 1280, 720);

    let mut state: Option<(SpritePipeline, wgpu::BindGroup)> = None;

    run(
        config,
        |_window| {},
        move |window| {
            // Lazy init on first frame
            let (pipeline, bind_group) = state.get_or_insert_with(|| {
                let p = SpritePipeline::new(&window.gpu.device, window.format())
                    .expect("Failed to create sprite pipeline");
                let white = Texture::white_pixel(&window.gpu.device, &window.gpu.queue);
                let bg = white.bind_group(&window.gpu.device, p.texture_bind_group_layout());
                (p, bg)
            });

            let (w, h) = window.size();
            pipeline.update_projection(&window.gpu.queue, w as f32, h as f32);

            // Build a grid of colored sprites
            let mut batch = SpriteBatch::with_capacity(25);
            for row in 0..5 {
                for col in 0..5 {
                    let t = (row * 5 + col) as f32 / 24.0;
                    let color = Color::RED.lerp(Color::BLUE, t);
                    let sprite = Sprite::new(
                        100.0 + col as f32 * 80.0,
                        100.0 + row as f32 * 80.0,
                        64.0,
                        64.0,
                    )
                    .with_color(color)
                    .with_z_order(row);
                    batch.push(sprite);
                }
            }
            batch.sort_by_z();

            let output = match window.current_texture() {
                Ok(t) => t,
                Err(_) => return true,
            };
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            pipeline.draw(
                &window.gpu.device,
                &window.gpu.queue,
                &view,
                &batch,
                bind_group,
                Some(Color::CORNFLOWER_BLUE),
            );

            output.present();
            true
        },
    )
    .expect("Event loop failed");
}
