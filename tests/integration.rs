//! Integration tests for soorat.

use soorat::WindowConfig;
use soorat::color::Color;
use soorat::pipeline::batch_to_vertices;
use soorat::sprite::{Sprite, SpriteBatch};
use soorat::texture::TextureCache;
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

#[test]
fn vertex_color_interop() {
    let color = Color::from_hex(0xFF8040FF);
    let v = Vertex2D {
        position: [10.0, 20.0],
        tex_coords: [0.0, 0.0],
        color: color.to_array(),
    };
    assert_eq!(v.color[0], color.r);
    assert_eq!(v.color[1], color.g);
    assert_eq!(v.color[2], color.b);
    assert_eq!(v.color[3], color.a);
}

#[test]
fn sprite_batch_serde_roundtrip() {
    let sprite = Sprite::new(10.0, 20.0, 64.0, 64.0)
        .with_color(Color::from_hex(0x336699FF))
        .with_rotation(1.57)
        .with_texture(42)
        .with_z_order(3);
    let json = serde_json::to_string(&sprite).unwrap();
    let decoded: Sprite = serde_json::from_str(&json).unwrap();
    assert_eq!(sprite, decoded);
}

#[test]
fn sprite_to_vertex_quad() {
    let sprite = Sprite::new(100.0, 200.0, 50.0, 30.0).with_color(Color::RED);
    let color_arr = sprite.color.to_array();

    // Generate quad vertices from sprite
    let verts = [
        Vertex2D {
            position: [sprite.x, sprite.y],
            tex_coords: [0.0, 0.0],
            color: color_arr,
        },
        Vertex2D {
            position: [sprite.x + sprite.width, sprite.y],
            tex_coords: [1.0, 0.0],
            color: color_arr,
        },
        Vertex2D {
            position: [sprite.x + sprite.width, sprite.y + sprite.height],
            tex_coords: [1.0, 1.0],
            color: color_arr,
        },
        Vertex2D {
            position: [sprite.x, sprite.y + sprite.height],
            tex_coords: [0.0, 1.0],
            color: color_arr,
        },
    ];

    // Verify quad covers sprite bounds
    let (min_x, min_y, max_x, max_y) = sprite.bounds();
    assert_eq!(verts[0].position, [min_x, min_y]);
    assert_eq!(verts[2].position, [max_x, max_y]);

    // Verify bytemuck cast works for GPU upload
    let bytes: &[u8] = bytemuck::cast_slice(&verts);
    assert_eq!(bytes.len(), 32 * 4);
}

#[test]
fn batch_sort_preserves_all_sprites() {
    let mut batch = SpriteBatch::new();
    for i in 0..50 {
        batch.push(
            Sprite::new(i as f32, 0.0, 10.0, 10.0)
                .with_z_order(50 - i)
                .with_texture(i as u64),
        );
    }
    batch.sort_by_z();
    assert_eq!(batch.len(), 50);
    // All texture IDs should still be present
    let mut ids: Vec<u64> = batch.sprites.iter().map(|s| s.texture_id).collect();
    ids.sort();
    assert_eq!(ids, (0..50).collect::<Vec<u64>>());
}

#[test]
fn pipeline_batch_to_vertices_integration() {
    let mut batch = SpriteBatch::with_capacity(10);
    for i in 0..10 {
        batch.push(
            Sprite::new(i as f32 * 50.0, 100.0, 32.0, 32.0)
                .with_color(Color::from_rgba8(255, i * 25, 0, 255))
                .with_z_order(10 - i as i32),
        );
    }
    batch.sort_by_z();

    let (verts, indices) = batch_to_vertices(&batch);
    assert_eq!(verts.len(), 40); // 10 sprites * 4 verts
    assert_eq!(indices.len(), 60); // 10 sprites * 6 indices

    // Verify all indices are in range
    let max_vert = verts.len() as u16;
    for &idx in &indices {
        assert!(idx < max_vert, "Index {idx} out of range (max {max_vert})");
    }

    // Verify bytemuck cast for GPU upload
    let bytes: &[u8] = bytemuck::cast_slice(&verts);
    assert_eq!(bytes.len(), 32 * 40);
}

#[test]
fn texture_cache_workflow() {
    let cache = TextureCache::new();
    assert!(cache.is_empty());
    assert!(!cache.contains(0));
    assert!(!cache.contains(42));
}

#[test]
fn pipeline_index_pattern() {
    let mut batch = SpriteBatch::new();
    batch.push(Sprite::new(0.0, 0.0, 10.0, 10.0));
    batch.push(Sprite::new(20.0, 0.0, 10.0, 10.0));
    batch.push(Sprite::new(40.0, 0.0, 10.0, 10.0));

    let (_, indices) = batch_to_vertices(&batch);
    // Each quad: base+0, base+1, base+2, base+2, base+3, base+0
    assert_eq!(&indices[0..6], &[0, 1, 2, 2, 3, 0]);
    assert_eq!(&indices[6..12], &[4, 5, 6, 6, 7, 4]);
    assert_eq!(&indices[12..18], &[8, 9, 10, 10, 11, 8]);
}
