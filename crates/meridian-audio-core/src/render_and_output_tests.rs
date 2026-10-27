//! Tests of this module.

use meridian_gac_core::{Motor3, Vec3};

use super::*;

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

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn audio_output_opens_for_every_layout_or_skips() {
    for layout in [SpeakerLayout::mono(), SpeakerLayout::stereo_headphones()] {
        match AudioOutput::open(&layout, 48000, None).await {
            Ok(output) => {
                let silence = vec![0.0f32; 64 * layout.speakers.len()];
                output.push_interleaved(&silence);
                assert!(output.can_push(1));
            }
            Err(err) => {
                eprintln!("skipping: no audio device available ({err})");
                return;
            }
        }
    }
}
