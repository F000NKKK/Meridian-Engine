//! Tests of the sprite adapter.

use meridian_graphics_core::{SpriteVertex, TextureHandle};

use super::sprite_encoding::{sprite_vertex_layout, write_vertex};
use super::*;

#[test]
fn vertices_encode_to_the_declared_stride() {
    let vertex = SpriteVertex {
        position: [45.0, 7.0],
        uv: [0.25, 0.75],
        color: [1.0, 0.5, 0.25, 1.0],
    };
    let mut bytes = Vec::new();
    write_vertex(&vertex, &mut bytes);
    assert_eq!(bytes.len(), SPRITE_VERTEX_BYTES);
    let x = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    assert!((x - 45.0).abs() < 1e-6);
    let alpha = f32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);
    assert!((alpha - 1.0).abs() < 1e-6);
}

#[test]
fn the_layout_stride_is_the_encoded_size() {
    let layout = sprite_vertex_layout();
    assert_eq!(layout.stride as usize, SPRITE_VERTEX_BYTES);
    assert_eq!(layout.attributes.len(), 3);
}

#[test]
fn registry_is_the_only_gate_for_unknown_textures() {
    // No GPU needed: an empty registry resolves nothing.
    let mut pool = meridian_memory_core::ResourcePool::<()>::default();
    let unknown = TextureHandle::new(pool.insert(()));
    assert!(crate::TextureRegistry::new().get(unknown).is_none());
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn prepared_scene_draws_into_an_offscreen_target_without_validation_errors() {
    use meridian_gac_core::{Motor3, Vec3};
    use meridian_graphics_core::scene::{Camera2d, Scene2d, Sprite};
    use meridian_graphics_core::{SpriteView, batch_scene2d};

    use crate::{ColorFormat, Device, TextureRegistry};

    let Ok(device) = Device::new().await else {
        eprintln!("skipping: no GPU device available");
        return;
    };
    let mut textures = TextureRegistry::new();
    let white = textures.upload_solid(&device, [255; 4]);
    let target = device.create_offscreen_color_texture(64, 64, ColorFormat::SrgbRgba8);
    let renderer = SpriteRenderer::new_offscreen(&device, ColorFormat::SrgbRgba8);

    let sprite = |x: f32, y: f32, layer: i32, tint: [f32; 4]| Sprite {
        texture: white,
        source_rect: None,
        size: [10.0, 10.0],
        tint,
        frame: Motor3::translation(Vec3::new(x, y, 0.0)),
        layer,
    };
    let scene = Scene2d {
        camera: Camera2d::from_surface_size(64.0, 64.0),
        sprites: vec![
            sprite(32.0, 32.0, 0, [1.0, 0.0, 0.0, 0.5]),
            sprite(20.0, 20.0, 1, [1.0; 4]),
        ],
    };
    let batches = batch_scene2d(&scene);
    let prepared = renderer.prepare(
        &device,
        SpriteView {
            camera: scene.camera,
            viewport: [64, 64],
        },
        &batches,
        &textures,
    );
    assert_eq!(prepared.draw_calls(), 1);

    let mut commands = device.create_command_buffer();
    {
        let mut pass = commands.begin_render_pass(target.view(), [0.0; 4], None);
        renderer.draw(&mut pass, &prepared);
    }
    commands.submit();
}
