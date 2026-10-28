//! Tests of voice playback.

use super::*;
use crate::Emitter;

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

#[test]
fn a_fade_in_ramps_up_from_silence() {
    let mut voice = voice(&[1.0; 8], Repeat::Loop).with_fade(4);

    assert_eq!(voice.render(6), &[0.25, 0.5, 0.75, 1.0, 1.0, 1.0]);
}

#[test]
fn pause_fades_out_then_stops_pulling_and_play_fades_back_in() {
    let mut voice = voice(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], Repeat::Once).with_fade(2);
    assert_eq!(voice.render(2), &[0.5, 2.0]);

    voice.pause();
    assert_eq!(voice.state(), PlaybackState::Paused);
    assert_eq!(
        voice.render(3),
        &[1.5, 0.0, 0.0],
        "ramps 1 -> 0.5 -> 0, then silent"
    );

    voice.play();
    // The source advanced over the whole 3-frame block that held the fade-out.
    assert_eq!(voice.render(2), &[3.0, 7.0]);
}

#[test]
fn stop_fades_out_and_is_done_only_when_silent() {
    let mut voice = voice(&[1.0; 8], Repeat::Loop).with_fade(2);
    voice.render(2);

    voice.stop();
    assert_eq!(voice.state(), PlaybackState::Finished);
    assert!(!voice.is_done(), "the fade-out is still audible");

    assert_eq!(voice.render(3), &[0.5, 0.0, 0.0]);
    assert!(voice.is_done());
}

#[test]
fn without_a_fade_stop_is_done_at_once() {
    let mut voice = voice(&[1.0], Repeat::Loop);
    voice.stop();

    assert!(voice.is_done() || voice.render(1) == [0.0]);
    voice.render(1);
    assert!(voice.is_done());
}

fn resampled(samples: &[f32], from: u32, to: u32) -> ResampledSource<MemorySource> {
    ResampledSource::new(MemorySource::new(samples.to_vec(), Repeat::Once), from, to)
}

fn read_all(source: &mut impl SampleSource) -> Vec<f32> {
    let mut out = vec![0.0; 64];
    let read = source.read(&mut out);
    out.truncate(read);
    out
}

#[test]
fn the_same_rate_passes_samples_through() {
    let mut source = resampled(&[1.0, 2.0, 3.0], 48_000, 48_000);

    assert_eq!(read_all(&mut source), vec![1.0, 2.0]);
}

#[test]
fn doubling_the_rate_interpolates_between_samples() {
    let mut source = resampled(&[0.0, 1.0, 2.0], 24_000, 48_000);

    assert_eq!(read_all(&mut source), vec![0.0, 0.5, 1.0, 1.5]);
}

#[test]
fn halving_the_rate_skips_every_other_sample() {
    let mut source = resampled(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0], 48_000, 24_000);

    assert_eq!(read_all(&mut source), vec![0.0, 2.0, 4.0]);
}

#[test]
fn a_source_too_short_to_interpolate_ends_at_once() {
    let mut source = resampled(&[1.0], 24_000, 48_000);

    assert_eq!(read_all(&mut source), Vec::<f32>::new());
}

#[test]
fn a_looping_source_never_ends_and_stays_continuous_across_reads() {
    let looping = MemorySource::new(vec![0.0, 1.0], Repeat::Loop);
    let mut source = ResampledSource::new(looping, 24_000, 48_000);
    let mut out = [0.0f32; 4];
    let mut heard = Vec::new();

    for _ in 0..3 {
        assert_eq!(source.read(&mut out), 4);
        heard.extend_from_slice(&out);
    }

    // 0 -> 1 -> 0 -> 1 ... sampled at half steps.
    assert_eq!(&heard[..6], &[0.0, 0.5, 1.0, 0.5, 0.0, 0.5]);
}

#[test]
fn a_zero_rate_leaves_the_source_unconverted() {
    let mut source = resampled(&[1.0, 2.0, 3.0], 0, 48_000);

    assert_eq!(read_all(&mut source), vec![1.0, 2.0]);
}

fn tone(hz: f32, rate: f32, frames: usize) -> Vec<f32> {
    let step = std::f32::consts::TAU * hz / rate;
    std::iter::successors(Some(0.0f32), |phase| Some(phase + step))
        .take(frames)
        .map(f32::sin)
        .collect()
}

fn rms_after_settling(source: &mut impl SampleSource) -> f32 {
    let mut out = vec![0.0; 1024];
    let read = source.read(&mut out);
    let settled = &out[read / 2..read];
    let mean_square = settled.iter().map(|s| s * s).sum::<f32>()
        / f32::from(u16::try_from(settled.len()).unwrap());
    mean_square.sqrt()
}

#[test]
fn anti_aliasing_removes_content_above_the_new_nyquist() {
    // 7 kHz is above the 4 kHz Nyquist of an 8 kHz stream and would fold to 1 kHz.
    let input = tone(7_000.0, 48_000.0, 8_192);
    let mut plain = resampled(&input, 48_000, 8_000);
    let mut filtered =
        ResampledSource::anti_aliased(MemorySource::new(input, Repeat::Once), 48_000, 8_000);

    assert!(rms_after_settling(&mut plain) > 0.5, "unfiltered aliases");
    assert!(rms_after_settling(&mut filtered) < 0.05);
}

#[test]
fn anti_aliasing_keeps_content_below_the_new_nyquist() {
    let input = tone(500.0, 48_000.0, 8_192);
    let mut filtered =
        ResampledSource::anti_aliased(MemorySource::new(input, Repeat::Once), 48_000, 8_000);

    let rms = rms_after_settling(&mut filtered);

    assert!(
        (rms - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.05,
        "rms {rms}"
    );
}

#[test]
fn anti_aliasing_leaves_upward_conversion_unfiltered() {
    let mut source = ResampledSource::anti_aliased(
        MemorySource::new(vec![0.0, 1.0, 2.0], Repeat::Once),
        24_000,
        48_000,
    );

    assert_eq!(read_all(&mut source), vec![0.0, 0.5, 1.0, 1.5]);
}
