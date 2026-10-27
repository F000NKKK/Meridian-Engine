//! Tests of this module.

use meridian_gac_core::{Motor3, Rotor, Vec3};

use super::*;
use crate::TextureHandle;
use crate::scene::{Camera2d, Scene2d, Sprite};
use crate::submission::TextureRegistry;

/// The `index`th handle a fresh pool hands out: same index, same handle.
pub(super) fn texture(index: usize) -> TextureHandle {
    let mut pool = meridian_memory_core::ResourcePool::<()>::default();
    let handles: Vec<_> = (0..=index).map(|_| pool.insert(())).collect();
    TextureHandle::new(handles[index])
}

pub(super) fn sprite(texture: TextureHandle, x: f32, y: f32, layer: i32) -> Sprite {
    Sprite {
        texture,
        source_rect: None,
        size: [10.0, 10.0],
        tint: [1.0; 4],
        frame: Motor3::translation(Vec3::new(x, y, 0.0)),
        layer,
    }
}

pub(super) fn assert_close(actual: [f32; 2], expected: [f32; 2]) {
    let close = actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (a - e).abs() < 1e-3);
    assert!(close, "got {actual:?}, expected {expected:?}");
}

pub(super) fn scene(sprites: Vec<Sprite>) -> Scene2d {
    Scene2d {
        camera: Camera2d::from_surface_size(100.0, 100.0),
        sprites,
    }
}

#[test]
fn empty_scene_has_no_batches() {
    assert!(batch_scene2d(&scene(vec![])).is_empty());
}

#[test]
fn quad_corners_are_centre_plus_half_size_with_y_down() {
    let batches = batch_scene2d(&scene(vec![sprite(texture(0), 50.0, 40.0, 0)]));
    let positions: Vec<[f32; 2]> = batches[0].vertices.iter().map(|v| v.position).collect();
    assert_close(positions[0], [45.0, 35.0]); // "top-left"
    assert_close(positions[1], [55.0, 35.0]); // "top-right"
    assert_close(positions[2], [55.0, 45.0]); // "bottom-right"
    assert_eq!(
        positions[5],
        [45.0, 45.0],
        "bottom-left closes the second triangle"
    );
}

#[test]
fn full_texture_uvs_run_from_top_left() {
    let batches = batch_scene2d(&scene(vec![sprite(texture(0), 50.0, 50.0, 0)]));
    assert_close(batches[0].vertices[0].uv, [0.0, 0.0]);
    assert_close(batches[0].vertices[2].uv, [1.0, 1.0]);
}

#[test]
fn source_rect_selects_a_normalized_sub_rectangle() {
    let mut s = sprite(texture(0), 50.0, 50.0, 0);
    s.source_rect = Some([0.25, 0.5, 0.25, 0.5]);
    let batches = batch_scene2d(&scene(vec![s]));
    assert_close(batches[0].vertices[0].uv, [0.25, 0.5]);
    assert_close(batches[0].vertices[2].uv, [0.5, 1.0]);
}

#[test]
fn lower_layers_draw_first_and_equal_layers_keep_submission_order() {
    let t = texture(0);
    let batches = batch_scene2d(&scene(vec![
        sprite(t, 30.0, 50.0, 2),
        sprite(t, 10.0, 50.0, 1),
        sprite(t, 20.0, 50.0, 1),
    ]));
    let centres: Vec<f32> = batches[0]
        .vertices
        .chunks(6)
        .map(|quad| quad[0].position[0] + 5.0)
        .collect();
    assert_close([centres[0], centres[1]], [10.0, 20.0]);
    assert_close([centres[2], 0.0], [30.0, 0.0]);
}

#[test]
fn consecutive_same_texture_sprites_merge_but_never_reorder() {
    let (a, b) = (texture(0), texture(1));
    let batches = batch_scene2d(&scene(vec![
        sprite(a, 10.0, 10.0, 0),
        sprite(b, 20.0, 10.0, 1),
        sprite(a, 30.0, 10.0, 2),
        sprite(a, 40.0, 10.0, 3),
    ]));
    let shape: Vec<(TextureHandle, usize)> = batches
        .iter()
        .map(|batch| (batch.texture, batch.quad_count()))
        .collect();
    assert_eq!(shape, [(a, 1), (b, 1), (a, 2)]);
}

#[test]
fn sprites_outside_the_camera_are_culled() {
    let t = texture(0);
    let batches = batch_scene2d(&scene(vec![
        sprite(t, 50.0, 50.0, 0),
        sprite(t, 200.0, 50.0, 1),
        sprite(t, 50.0, -20.0, 2),
    ]));
    assert_eq!(batches[0].quad_count(), 1);
}

#[test]
fn a_sprite_straddling_the_edge_is_kept() {
    let batches = batch_scene2d(&scene(vec![sprite(texture(0), 0.0, 50.0, 0)]));
    assert_eq!(batches[0].quad_count(), 1);
}

#[test]
fn fully_transparent_sprites_are_dropped() {
    let mut s = sprite(texture(0), 50.0, 50.0, 0);
    s.tint[3] = 0.0;
    assert!(batch_scene2d(&scene(vec![s])).is_empty());
}

#[test]
fn rotation_about_the_plane_normal_turns_the_quad() {
    let mut s = sprite(texture(0), 50.0, 50.0, 0);
    let turn = Rotor::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), core::f32::consts::FRAC_PI_2);
    s.frame = Motor3::from_rotation_translation(turn, Vec3::new(50.0, 50.0, 0.0));
    let batches = batch_scene2d(&scene(vec![s]));
    let [x, y] = batches[0].vertices[0].position;
    // (-5, -5) turned a quarter turn lands on (5, -5) or (-5, 5).
    let near = |target: [f32; 2]| (x - target[0]).abs() < 1e-3 && (y - target[1]).abs() < 1e-3;
    assert!(near([55.0, 45.0]) || near([45.0, 55.0]), "got ({x}, {y})");
}

#[test]
fn vertices_encode_to_the_declared_stride() {
    let batches = batch_scene2d(&scene(vec![sprite(texture(0), 50.0, 50.0, 0)]));
    let mut bytes = Vec::new();
    batches[0].vertices[0].write_to(&mut bytes);
    assert_eq!(bytes.len(), SPRITE_VERTEX_BYTES);
    let x = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    assert_close([x, 0.0], [45.0, 0.0]);
}

#[test]
fn hand_built_batches_need_no_sprite() {
    let mut batch = SpriteBatch::new(texture(0));
    batch.push_quad(
        [[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]],
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        [1.0, 0.0, 0.0, 1.0],
    );
    assert_eq!(batch.vertices.len(), 6);
}

#[test]
fn registry_is_the_only_gate_for_unknown_textures() {
    // No GPU needed: an empty registry resolves nothing.
    assert!(TextureRegistry::new().get(texture(0)).is_none());
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn prepared_scene_draws_into_an_offscreen_target_without_validation_errors() {
    use meridian_graphics_driver::{ColorFormat, Device};

    let Ok(device) = Device::new().await else {
        eprintln!("skipping: no GPU device available");
        return;
    };
    let mut textures = TextureRegistry::new();
    let white = textures.upload_solid(&device, [255; 4]);
    let target = device.create_offscreen_color_texture(64, 64, ColorFormat::SrgbRgba8);
    let renderer = SpriteRenderer::new_offscreen(&device, ColorFormat::SrgbRgba8);

    let mut tinted = sprite(white, 32.0, 32.0, 0);
    tinted.tint = [1.0, 0.0, 0.0, 0.5];
    let scene = Scene2d {
        camera: Camera2d::from_surface_size(64.0, 64.0),
        sprites: vec![tinted, sprite(white, 20.0, 20.0, 1)],
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
