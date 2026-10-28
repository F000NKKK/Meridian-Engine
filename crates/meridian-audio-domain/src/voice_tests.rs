//! Tests of voice playback.

use super::*;

fn voice(samples: &[f32], repeat: Repeat) -> Voice {
    Voice::new(
        MemorySource::new(samples.to_vec(), repeat),
        Emitter::default(),
    )
}

#[test]
fn a_looping_voice_wraps_and_never_finishes() {
    let mut voice = voice(&[1.0, 2.0, 3.0], Repeat::Loop);

    assert_eq!(voice.render(7), &[1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0]);
    assert_eq!(voice.state(), PlaybackState::Playing);
}

#[test]
fn a_one_shot_voice_pads_with_silence_and_finishes() {
    let mut voice = voice(&[1.0, 2.0], Repeat::Once);

    assert_eq!(voice.render(4), &[1.0, 2.0, 0.0, 0.0]);
    assert_eq!(voice.state(), PlaybackState::Finished);
    assert_eq!(voice.render(2), &[0.0, 0.0]);
}

#[test]
fn a_one_shot_ending_exactly_on_a_block_edge_finishes_on_the_next_block() {
    let mut voice = voice(&[1.0, 2.0], Repeat::Once);

    assert_eq!(voice.render(2), &[1.0, 2.0]);
    assert_eq!(voice.state(), PlaybackState::Playing);
    assert_eq!(voice.render(2), &[0.0, 0.0]);
    assert_eq!(voice.state(), PlaybackState::Finished);
}

#[test]
fn pause_is_silent_and_keeps_the_position() {
    let mut voice = voice(&[1.0, 2.0, 3.0, 4.0], Repeat::Once);
    assert_eq!(voice.render(2), &[1.0, 2.0]);

    voice.pause();
    assert_eq!(voice.render(2), &[0.0, 0.0]);

    voice.play();
    assert_eq!(voice.render(2), &[3.0, 4.0]);
}

#[test]
fn stop_is_final() {
    let mut voice = voice(&[1.0], Repeat::Loop);
    voice.stop();
    voice.play();

    assert_eq!(voice.state(), PlaybackState::Finished);
    assert_eq!(voice.render(1), &[0.0]);
}

#[test]
fn gain_scales_the_samples() {
    let mut voice = voice(&[1.0, -1.0], Repeat::Loop);
    voice.gain = 0.5;

    assert_eq!(voice.render(2), &[0.5, -0.5]);
}

#[test]
fn an_empty_source_finishes_at_once() {
    let mut voice = voice(&[], Repeat::Loop);

    assert_eq!(voice.render(2), &[0.0, 0.0]);
    assert_eq!(voice.state(), PlaybackState::Finished);
}
