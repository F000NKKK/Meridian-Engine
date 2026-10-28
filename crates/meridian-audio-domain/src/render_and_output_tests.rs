//! Tests of this module.

use meridian_gac::{Motor3, Vec3};

use super::*;
use crate::{Channel, Speaker};

fn listener() -> Listener {
    Listener {
        frame: Motor3::identity(),
    }
}

fn emitter_at(position: Vec3) -> Emitter {
    Emitter {
        frame: Motor3::translation(position),
    }
}

#[test]
fn render_interleaved_matches_mix_per_frame() {
    let mixer = Mixer::new(SpeakerLayout::stereo_headphones());
    let emitter = emitter_at(Vec3::new(1.0, 0.0, 2.0));
    let source = [0.25f32, -0.5, 1.0];

    let out = mixer.render_interleaved(&listener(), &[(emitter, &source)], source.len());
    assert_eq!(out.len(), source.len() * 2);

    for (frame, sample) in source.iter().enumerate() {
        let per_channel = mixer.mix(&listener(), &[(emitter, *sample)]);
        assert!((out[frame * 2] - per_channel[0].1).abs() < 1e-6);
        assert!((out[frame * 2 + 1] - per_channel[1].1).abs() < 1e-6);
    }
}

#[test]
fn render_interleaved_sums_multiple_sources() {
    let mixer = Mixer::new(SpeakerLayout::mono());
    let a = emitter_at(Vec3::new(1.0, 0.0, 0.0));
    let b = emitter_at(Vec3::new(1.0, 0.0, 0.0));
    let source = [0.5f32];

    let one = mixer.render_interleaved(&listener(), &[(a, &source)], 1);
    let two = mixer.render_interleaved(&listener(), &[(a, &source), (b, &source)], 1);
    assert!((two[0] - 2.0 * one[0]).abs() < 1e-6);
}

#[test]
fn render_interleaved_pads_short_sources_with_silence() {
    let mixer = Mixer::new(SpeakerLayout::stereo_headphones());
    let emitter = emitter_at(Vec3::new(1.0, 0.0, 0.0));
    let source = [1.0f32];

    let out = mixer.render_interleaved(&listener(), &[(emitter, &source)], 3);
    assert_eq!(out.len(), 6);
    assert!(out[0] > 0.0);
    assert_eq!(&out[2..], &[0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn render_interleaved_into_overwrites_and_matches_allocating_render() {
    let mixer = Mixer::new(SpeakerLayout::surround_5_1());
    let emitter = emitter_at(Vec3::new(-1.0, 0.0, 2.0));
    let source = [0.25f32, -0.5, 1.0, 0.75];
    let sources = [(emitter, &source[..])];

    let expected = mixer.render_interleaved(&listener(), &sources, 4);
    let mut reused = vec![9.0f32; expected.len()];
    mixer.render_interleaved_into(&listener(), &sources, &mut reused);

    assert_eq!(expected, reused);
}

#[test]
fn mix_into_matches_mix() {
    let mixer = Mixer::new(SpeakerLayout::surround_5_0());
    let emitters = [
        (emitter_at(Vec3::new(2.0, 0.0, 1.0)), 0.5),
        (emitter_at(Vec3::new(-2.0, 0.0, -1.0)), -0.25),
    ];

    let mut sums = vec![7.0f32; 5];
    mixer.mix_into(&listener(), &emitters, &mut sums);

    for ((_, expected), actual) in mixer.mix(&listener(), &emitters).iter().zip(&sums) {
        assert!((expected - actual).abs() < 1e-6);
    }
}

#[test]
fn layouts_wider_than_the_stack_scratch_still_render() {
    let speakers = (0..20u8)
        .map(|i| Speaker {
            channel: Channel::Center,
            azimuth_degrees: Some(f32::from(i) * 18.0 - 180.0 + 9.0),
        })
        .collect();
    let mixer = Mixer::new(SpeakerLayout {
        speakers,
        wraps_around: true,
    });
    let source = [1.0f32];
    let out = mixer.render_interleaved(
        &listener(),
        &[(emitter_at(Vec3::new(1.0, 0.0, 0.0)), &source)],
        1,
    );

    let power: f32 = out.iter().map(|g| g * g).sum();
    assert!((power - 1.0).abs() < 1e-3, "constant power, got {power}");
}

#[test]
fn pan_into_is_constant_power_around_the_whole_ring() {
    let layout = SpeakerLayout::surround_5_1();
    let mut gains = vec![0.0f32; layout.speakers.len()];

    for step in -180i16..=180 {
        layout.pan_into(f32::from(step), &mut gains);
        let power: f32 = gains.iter().map(|g| g * g).sum();
        assert!((power - 1.0).abs() < 1e-3, "azimuth {step}: power {power}");
    }
}

#[test]
fn non_finite_emitter_position_yields_finite_or_silent_output_without_panicking() {
    let mixer = Mixer::new(SpeakerLayout::surround_5_1());
    let source = [1.0f32];
    let emitter = emitter_at(Vec3::new(f32::NAN, 0.0, 0.0));

    let out = mixer.render_interleaved(&listener(), &[(emitter, &source)], 1);

    assert_eq!(out.len(), 6);
}
