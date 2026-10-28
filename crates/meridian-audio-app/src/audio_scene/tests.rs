//! Tests of the scene.

use meridian_audio_domain::{Declicker, Gain, MemorySource, Mixer, Repeat, SpeakerLayout};
use meridian_gac::{Motor3, Vec3};

use super::*;

fn right_of_listener() -> Emitter {
    Emitter {
        frame: Motor3::translation(Vec3::new(0.0, 0.0, 0.5)),
    }
}

fn stereo_scene() -> AudioScene {
    AudioScene::new(Mixer::new(SpeakerLayout::stereo_headphones()))
}

#[test]
fn a_voice_right_of_the_listener_plays_in_the_right_channel() {
    let mut scene = stereo_scene();
    scene.add_voice(Voice::new(
        MemorySource::new(vec![1.0; 8], Repeat::Loop),
        right_of_listener(),
    ));

    let mut out = [0.0f32; 8];
    scene.render_block(&mut out);

    assert!(out.chunks_exact(2).all(|f| f[1] > 0.5 && f[0].abs() < 1e-3));
}

#[test]
fn moving_the_emitter_moves_the_sound() {
    let mut scene = stereo_scene();
    let id = scene.add_voice(Voice::new(
        MemorySource::new(vec![1.0; 4], Repeat::Loop),
        right_of_listener(),
    ));
    let left = Emitter {
        frame: Motor3::translation(Vec3::new(0.0, 0.0, -0.5)),
    };
    assert!(scene.set_emitter(id, left));

    let mut out = [0.0f32; 4];
    scene.render_block(&mut out);

    assert!(out[0] > 0.5 && out[1].abs() < 1e-3);
}

#[test]
fn finished_voices_are_reaped_and_their_ids_go_stale() {
    let mut scene = stereo_scene();
    let id = scene.add_voice(Voice::new(
        MemorySource::new(vec![1.0], Repeat::Once),
        right_of_listener(),
    ));

    scene.render_block(&mut [0.0; 4]);

    assert_eq!(scene.reap_finished(), 1);
    assert_eq!(scene.voice_count(), 0);
    assert!(!scene.set_emitter(id, right_of_listener()));
}

fn looping_voice() -> Voice {
    Voice::new(
        MemorySource::new(vec![1.0; 4], Repeat::Loop),
        right_of_listener(),
    )
}

#[test]
fn a_full_scene_steals_the_oldest_voice() {
    let mut scene = stereo_scene().with_max_voices(2);
    let oldest = scene.add_voice(looping_voice());
    let middle = scene.add_voice(looping_voice());

    let newest = scene.add_voice(looping_voice());

    assert_eq!(scene.voice_count(), 2);
    assert!(scene.voice_mut(oldest).is_none());
    assert!(scene.voice_mut(middle).is_some());
    assert!(scene.voice_mut(newest).is_some());
}

#[test]
fn a_full_scene_steals_the_lowest_priority_voice() {
    let mut scene = stereo_scene().with_max_voices(2);
    let mut important = looping_voice();
    important.priority = 5;
    let important = scene.add_voice(important);
    let minor = scene.add_voice(looping_voice());

    let newest = scene.add_voice(looping_voice());

    assert!(
        scene.voice_mut(important).is_some(),
        "the old but important voice stays"
    );
    assert!(scene.voice_mut(minor).is_none());
    assert!(scene.voice_mut(newest).is_some());
}

#[test]
fn a_new_voice_that_every_voice_outranks_is_dropped() {
    let mut scene = stereo_scene().with_max_voices(1);
    let mut loud = looping_voice();
    loud.priority = 3;
    let loud = scene.add_voice(loud);

    let weak = scene.add_voice(looping_voice());

    assert!(scene.voice_mut(loud).is_some());
    assert!(scene.voice_mut(weak).is_none());
    assert_eq!(scene.voice_count(), 1);
}

#[test]
fn a_full_scene_prefers_dropping_done_voices_to_stealing() {
    let mut scene = stereo_scene().with_max_voices(2);
    let oldest = scene.add_voice(looping_voice());
    let stopped = scene.add_voice(looping_voice());
    scene.voice_mut(stopped).unwrap().stop();

    scene.add_voice(looping_voice());

    assert!(scene.voice_mut(oldest).is_some(), "the live voice survives");
    assert!(scene.voice_mut(stopped).is_none());
}

#[test]
fn a_fading_out_voice_is_kept_until_it_is_silent() {
    let mut scene = stereo_scene();
    let id = scene.add_voice(looping_voice().with_fade(2));
    scene.render_block(&mut [0.0; 4]);
    scene.voice_mut(id).unwrap().stop();

    assert_eq!(scene.reap_finished(), 0, "the fade-out is still audible");
    scene.render_block(&mut [0.0; 6]);
    assert_eq!(scene.reap_finished(), 1);
}

#[test]
fn post_effects_run_over_the_rendered_block() {
    let mut scene = stereo_scene().with_post_effect(Gain(0.5));
    scene.add_voice(Voice::new(
        MemorySource::new(vec![1.0; 2], Repeat::Loop),
        right_of_listener(),
    ));
    let mut plain = stereo_scene();
    plain.add_voice(Voice::new(
        MemorySource::new(vec![1.0; 2], Repeat::Loop),
        right_of_listener(),
    ));

    let (mut halved, mut full) = ([0.0f32; 4], [0.0f32; 4]);
    scene.render_block(&mut halved);
    plain.render_block(&mut full);

    assert!((halved[1] - full[1] * 0.5).abs() < 1e-6);
}

#[test]
fn an_empty_scene_renders_silence() {
    let mut scene = stereo_scene().with_post_effect(Declicker::new(2));
    let mut out = [1.0f32; 6];
    scene.render_block(&mut out);

    assert!(out.iter().all(|s| s.abs() < 1e-6));
}

fn loud_voice() -> Voice {
    Voice::new(
        MemorySource::new(vec![1.0; 4], Repeat::Loop),
        right_of_listener(),
    )
}

fn right_level(scene: &mut AudioScene) -> f32 {
    let mut out = [0.0f32; 4];
    scene.render_block(&mut out);
    out[1]
}

#[test]
fn a_bus_gain_scales_only_its_voices() {
    let mut scene = stereo_scene();
    let music = scene.add_bus();
    scene.add_voice_to(music, loud_voice());
    let baseline = right_level(&mut scene);

    assert!(scene.set_bus_gain(music, 0.5));
    let halved = right_level(&mut scene);
    scene.add_voice(loud_voice());
    let with_direct = right_level(&mut scene);

    assert!((halved - baseline * 0.5).abs() < 1e-4);
    assert!((with_direct - (halved + baseline)).abs() < 1e-4);
}

#[test]
fn the_master_bus_scales_the_whole_mix() {
    let mut scene = stereo_scene();
    let sfx = scene.add_bus();
    scene.add_voice_to(sfx, loud_voice());
    scene.add_voice(loud_voice());
    let full = right_level(&mut scene);

    scene.set_bus_gain(BusId::MASTER, 0.25);

    assert!((right_level(&mut scene) - full * 0.25).abs() < 1e-4);
}

#[test]
fn a_silenced_bus_is_silent_and_bad_gains_mean_silence() {
    let mut scene = stereo_scene();
    let bus = scene.add_bus();
    scene.add_voice_to(bus, loud_voice());

    scene.set_bus_gain(bus, f32::NAN);
    assert!(right_level(&mut scene).abs() < 1e-6);
    scene.set_bus_gain(bus, -2.0);
    assert_eq!(
        scene.bus_gain(bus).map(f32::to_bits),
        Some(0.0f32.to_bits())
    );
}

#[test]
fn an_unknown_bus_falls_back_to_master_and_cannot_be_set() {
    let mut scene = stereo_scene();
    let other = stereo_scene().add_bus();

    let id = scene.add_voice_to(other, loud_voice());

    assert!(scene.voice_mut(id).is_some());
    assert!(!scene.set_bus_gain(other, 0.5));
    assert_eq!(scene.bus_gain(other), None);
}
