//! Tests of clip rectangles and clipped batching.

use super::tests::{assert_close, scene, sprite, texture};
use super::*;
use crate::scene::Camera2d;

fn rect(x: f32, y: f32, w: f32, h: f32) -> ClipRect {
    ClipRect {
        origin: [x, y],
        size: [w, h],
    }
}

fn span(layers: core::ops::RangeInclusive<i32>, rect: ClipRect) -> ClipSpan {
    ClipSpan { layers, rect }
}

#[test]
fn intersection_is_the_shared_part() {
    let shared = rect(0.0, 0.0, 10.0, 10.0).intersect(rect(5.0, 4.0, 10.0, 10.0));
    let shared = shared.expect("overlapping rectangles intersect");
    assert_close(shared.origin, [5.0, 4.0]);
    assert_close(shared.size, [5.0, 6.0]);
}

#[test]
fn disjoint_rectangles_do_not_intersect() {
    assert!(
        rect(0.0, 0.0, 10.0, 10.0)
            .intersect(rect(20.0, 0.0, 5.0, 5.0))
            .is_none()
    );
}

#[test]
fn scissor_scales_camera_units_to_pixels_y_down() {
    let camera = Camera2d::from_surface_size(100.0, 100.0);
    let scissor = rect(10.0, 20.0, 30.0, 40.0).to_scissor(camera, [200, 200]);
    let expected = ScissorRect {
        x: 20,
        y: 40,
        width: 60,
        height: 80,
    };
    assert_eq!(scissor, Some(expected));
}

#[test]
fn scissor_is_clamped_to_the_target() {
    let camera = Camera2d::from_surface_size(100.0, 100.0);
    let scissor = rect(-50.0, 90.0, 80.0, 50.0).to_scissor(camera, [100, 100]);
    let expected = ScissorRect {
        x: 0,
        y: 90,
        width: 30,
        height: 10,
    };
    assert_eq!(scissor, Some(expected));
}

#[test]
fn scissor_outside_the_target_is_none() {
    let camera = Camera2d::from_surface_size(100.0, 100.0);
    assert!(
        rect(150.0, 0.0, 10.0, 10.0)
            .to_scissor(camera, [100, 100])
            .is_none()
    );
}

#[test]
fn scissor_follows_a_y_up_world_camera() {
    let camera = Camera2d::from_world_extent(100.0, 100.0);
    // The top-left quarter of the world is the top-left of the target.
    let scissor = rect(-50.0, 0.0, 50.0, 50.0).to_scissor(camera, [100, 100]);
    let expected = ScissorRect {
        x: 0,
        y: 0,
        width: 50,
        height: 50,
    };
    assert_eq!(scissor, Some(expected));
}

#[test]
fn sprites_under_a_span_carry_its_clip() {
    let t = texture(0);
    let view = rect(0.0, 0.0, 50.0, 100.0);
    let batches = batch_scene2d_clipped(
        &scene(vec![sprite(t, 20.0, 50.0, 0), sprite(t, 30.0, 50.0, 5)]),
        &[span(5..=5, view)],
    );
    let clips: Vec<Option<ClipRect>> = batches.iter().map(|batch| batch.clip).collect();
    assert_eq!(clips, [None, Some(view)]);
}

#[test]
fn sprites_entirely_outside_their_clip_are_dropped() {
    let t = texture(0);
    let batches = batch_scene2d_clipped(
        &scene(vec![sprite(t, 80.0, 50.0, 0)]),
        &[span(0..=0, rect(0.0, 0.0, 40.0, 100.0))],
    );
    assert!(batches.is_empty());
}

#[test]
fn same_texture_with_a_different_clip_does_not_merge() {
    let t = texture(0);
    let batches = batch_scene2d_clipped(
        &scene(vec![sprite(t, 10.0, 10.0, 0), sprite(t, 20.0, 10.0, 1)]),
        &[span(1..=1, rect(0.0, 0.0, 100.0, 100.0))],
    );
    assert_eq!(batches.len(), 2);
}

#[test]
fn nested_spans_clip_to_their_intersection() {
    let t = texture(0);
    let batches = batch_scene2d_clipped(
        &scene(vec![sprite(t, 50.0, 50.0, 3)]),
        &[
            span(0..=9, rect(0.0, 0.0, 60.0, 100.0)),
            span(3..=3, rect(40.0, 0.0, 60.0, 100.0)),
        ],
    );
    let clip = batches[0].clip.expect("clipped");
    assert_close(clip.origin, [40.0, 0.0]);
    assert_close(clip.size, [20.0, 100.0]);
}

#[test]
fn spans_that_do_not_overlap_hide_the_layer() {
    let t = texture(0);
    let batches = batch_scene2d_clipped(
        &scene(vec![sprite(t, 50.0, 50.0, 3)]),
        &[
            span(3..=3, rect(0.0, 0.0, 10.0, 10.0)),
            span(3..=3, rect(50.0, 50.0, 10.0, 10.0)),
        ],
    );
    assert!(batches.is_empty());
}
