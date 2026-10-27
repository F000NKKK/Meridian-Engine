//! Tests of this module.

use meridian_gac::{Motor3, Vec3};

use super::*;

fn listener_at_origin_facing_x() -> Listener {
    Listener {
        frame: Motor3::identity(),
    }
}

fn emitter_at(position: Vec3) -> Emitter {
    Emitter {
        frame: Motor3::translation(position),
    }
}

/// A model with `gain == 1.0` for any distance used in these tests
/// (`reference_distance` far beyond them) — isolates *direction* tests
/// from *distance* attenuation, which has its own dedicated tests.
fn no_attenuation() -> AttenuationModel {
    AttenuationModel {
        reference_distance: 1000.0,
        rolloff: 1.0,
        max_distance: 1000.0,
    }
}

fn gain_of(gains: &[(Channel, f32)], channel: Channel) -> f32 {
    gains
        .iter()
        .find(|(c, _)| *c == channel)
        .map(|(_, g)| *g)
        .unwrap_or(0.0)
}

// ---- direction, stereo (headphones: full ±90° hemisphere) ----

#[test]
fn stereo_headphones_front_source_is_centered() {
    let listener = listener_at_origin_facing_x();
    let emitter = emitter_at(Vec3::new(5.0, 0.0, 0.0));
    let gains = spatial_gains(
        &listener,
        &emitter,
        &SpeakerLayout::stereo_headphones(),
        &no_attenuation(),
    );
    let (l, r) = (
        gain_of(&gains, Channel::Left),
        gain_of(&gains, Channel::Right),
    );
    assert!(
        (l - r).abs() < 1e-4,
        "front source must be centered, got L={l} R={r}"
    );
    assert!(l > 0.5, "centered gain should be ~0.707, got {l}");
}

#[test]
fn stereo_headphones_behind_source_is_also_centered() {
    // The well-known limitation this module doc explains: without
    // HRTF, amplitude-only stereo can't distinguish front from back —
    // both must produce the same centered pan.
    let listener = listener_at_origin_facing_x();
    let front = emitter_at(Vec3::new(5.0, 0.0, 0.0));
    let behind = emitter_at(Vec3::new(-5.0, 0.0, 0.0));
    let attenuation = AttenuationModel::default();
    let layout = SpeakerLayout::stereo_headphones();
    let front_gains = spatial_gains(&listener, &front, &layout, &attenuation);
    let behind_gains = spatial_gains(&listener, &behind, &layout, &attenuation);
    assert!(
        (gain_of(&front_gains, Channel::Left) - gain_of(&behind_gains, Channel::Left)).abs() < 1e-4
    );
    assert!(
        (gain_of(&front_gains, Channel::Right) - gain_of(&behind_gains, Channel::Right)).abs()
            < 1e-4
    );
}

#[test]
fn stereo_headphones_left_source_is_full_left() {
    let listener = listener_at_origin_facing_x();
    let emitter = emitter_at(Vec3::new(0.0, 0.0, -5.0));
    let gains = spatial_gains(
        &listener,
        &emitter,
        &SpeakerLayout::stereo_headphones(),
        &no_attenuation(),
    );
    assert!((gain_of(&gains, Channel::Left) - 1.0).abs() < 1e-4);
    assert!(gain_of(&gains, Channel::Right) < 1e-4);
}

#[test]
fn stereo_headphones_right_source_is_full_right() {
    let listener = listener_at_origin_facing_x();
    let emitter = emitter_at(Vec3::new(0.0, 0.0, 5.0));
    let gains = spatial_gains(
        &listener,
        &emitter,
        &SpeakerLayout::stereo_headphones(),
        &no_attenuation(),
    );
    assert!((gain_of(&gains, Channel::Right) - 1.0).abs() < 1e-4);
    assert!(gain_of(&gains, Channel::Left) < 1e-4);
}

// ---- headphones vs speakers: narrower stereo clamps sooner ----

#[test]
fn narrow_stereo_speakers_clamp_beyond_their_own_placement_while_headphones_still_blend() {
    // A source at -60°: inside headphones' ±90° span (blends), but
    // beyond speakers' ±30° span (clamps to full left).
    let listener = listener_at_origin_facing_x();
    // azimuth -60° at some radius: x = r*cos(-60°)... easier to place
    // directly via local-space intent: forward=x, right=z, so
    // azimuth = atan2(z, x). -60 degrees => z = -sin(60), x = cos(60).
    let angle = (-60.0_f32).to_radians();
    let emitter = emitter_at(Vec3::new(angle.cos() * 5.0, 0.0, angle.sin() * 5.0));
    let attenuation = no_attenuation();

    let headphone_gains = spatial_gains(
        &listener,
        &emitter,
        &SpeakerLayout::stereo_headphones(),
        &attenuation,
    );
    let speaker_gains = spatial_gains(
        &listener,
        &emitter,
        &SpeakerLayout::stereo_speakers(),
        &attenuation,
    );

    assert!(
        gain_of(&headphone_gains, Channel::Right) > 1e-3,
        "headphones should still blend in some right at -60deg"
    );
    assert!(
        gain_of(&speaker_gains, Channel::Right) < 1e-4,
        "narrow speakers should already be fully clamped left at -60deg"
    );
    assert!((gain_of(&speaker_gains, Channel::Left) - 1.0).abs() < 1e-4);
}

// ---- 5.0/5.1: real rear speakers distinguish front from back ----

#[test]
fn surround_5_0_front_source_goes_mostly_to_center() {
    let listener = listener_at_origin_facing_x();
    let emitter = emitter_at(Vec3::new(5.0, 0.0, 0.0));
    let gains = spatial_gains(
        &listener,
        &emitter,
        &SpeakerLayout::surround_5_0(),
        &no_attenuation(),
    );
    assert!((gain_of(&gains, Channel::Center) - 1.0).abs() < 1e-4);
    assert!(gain_of(&gains, Channel::SurroundLeft) < 1e-4);
    assert!(gain_of(&gains, Channel::SurroundRight) < 1e-4);
}

#[test]
fn surround_5_0_behind_source_splits_between_surrounds_not_center() {
    // Unlike stereo, 5.0 has real rear speakers: front and back are
    // now distinguishable, and a dead-behind source must not leak
    // into the front Center channel at all.
    let listener = listener_at_origin_facing_x();
    let emitter = emitter_at(Vec3::new(-5.0, 0.0, 0.0));
    let gains = spatial_gains(
        &listener,
        &emitter,
        &SpeakerLayout::surround_5_0(),
        &no_attenuation(),
    );
    assert_eq!(
        gain_of(&gains, Channel::Center),
        0.0,
        "a dead-behind source must not leak into the front Center channel"
    );
    let (sl, sr) = (
        gain_of(&gains, Channel::SurroundLeft),
        gain_of(&gains, Channel::SurroundRight),
    );
    assert!(
        (sl - sr).abs() < 1e-4,
        "dead-behind should split evenly between the two surrounds, got SL={sl} SR={sr}"
    );
    assert!(sl > 0.5);
}

#[test]
fn surround_5_0_left_source_goes_to_left_speaker() {
    let listener = listener_at_origin_facing_x();
    let emitter = emitter_at(Vec3::new(0.0, 0.0, -5.0));
    let gains = spatial_gains(
        &listener,
        &emitter,
        &SpeakerLayout::surround_5_0(),
        &no_attenuation(),
    );
    // -90 degrees falls between L(-30) and SL(-110).
    assert!(gain_of(&gains, Channel::Left) > 0.0);
    assert!(gain_of(&gains, Channel::SurroundLeft) > 0.0);
    assert_eq!(gain_of(&gains, Channel::Right), 0.0);
    assert_eq!(gain_of(&gains, Channel::Center), 0.0);
}

#[test]
fn surround_5_1_lfe_never_receives_directional_gain() {
    let listener = listener_at_origin_facing_x();
    for position in [
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(-5.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(0.0, 0.0, -5.0),
    ] {
        let emitter = emitter_at(position);
        let gains = spatial_gains(
            &listener,
            &emitter,
            &SpeakerLayout::surround_5_1(),
            &no_attenuation(),
        );
        assert_eq!(gain_of(&gains, Channel::LowFrequency), 0.0);
    }
}

#[test]
fn mono_always_gets_full_gain_regardless_of_direction() {
    let listener = listener_at_origin_facing_x();
    for position in [
        Vec3::new(5.0, 0.0, 0.0),
        Vec3::new(-5.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(0.0, 0.0, -5.0),
    ] {
        let emitter = emitter_at(position);
        let gains = spatial_gains(
            &listener,
            &emitter,
            &SpeakerLayout::mono(),
            &no_attenuation(),
        );
        assert_eq!(gain_of(&gains, Channel::Center), 1.0);
    }
}

// ---- distance attenuation ----

#[test]
fn attenuation_gain_is_one_at_or_inside_reference_distance() {
    let model = AttenuationModel::default();
    assert_eq!(model.gain(0.5), 1.0);
    assert_eq!(model.gain(1.0), 1.0);
}

#[test]
fn attenuation_gain_decreases_with_distance() {
    let model = AttenuationModel::default();
    assert!(model.gain(2.0) < model.gain(1.0));
    assert!(model.gain(10.0) < model.gain(2.0));
}

#[test]
fn closer_source_is_louder_than_farther_source_same_direction() {
    let listener = listener_at_origin_facing_x();
    let near = emitter_at(Vec3::new(2.0, 0.0, 0.0));
    let far = emitter_at(Vec3::new(20.0, 0.0, 0.0));
    let attenuation = AttenuationModel::default();
    let layout = SpeakerLayout::mono();
    let near_gain = gain_of(
        &spatial_gains(&listener, &near, &layout, &attenuation),
        Channel::Center,
    );
    let far_gain = gain_of(
        &spatial_gains(&listener, &far, &layout, &attenuation),
        Channel::Center,
    );
    assert!(near_gain > far_gain);
}

// ---- mixer: multiple simultaneous sources ----

#[test]
fn mixer_sums_multiple_sources_linearly() {
    let listener = listener_at_origin_facing_x();
    let left_source = (emitter_at(Vec3::new(0.0, 0.0, -5.0)), 1.0);
    let right_source = (emitter_at(Vec3::new(0.0, 0.0, 5.0)), 1.0);
    let mixer = Mixer::new(SpeakerLayout::stereo_headphones());

    let mixed = mixer.mix(&listener, &[left_source, right_source]);
    let solo_left = mixer.mix(&listener, &[left_source]);
    let solo_right = mixer.mix(&listener, &[right_source]);

    assert!((gain_of(&mixed, Channel::Left) - gain_of(&solo_left, Channel::Left)).abs() < 1e-4);
    assert!((gain_of(&mixed, Channel::Right) - gain_of(&solo_right, Channel::Right)).abs() < 1e-4);
}

#[test]
fn mixer_respects_per_source_sample_amplitude() {
    let listener = listener_at_origin_facing_x();
    let mixer = Mixer::new(SpeakerLayout::mono());
    let quiet = mixer.mix(&listener, &[(emitter_at(Vec3::new(5.0, 0.0, 0.0)), 0.1)]);
    let loud = mixer.mix(&listener, &[(emitter_at(Vec3::new(5.0, 0.0, 0.0)), 0.9)]);
    assert!(gain_of(&loud, Channel::Center) > gain_of(&quiet, Channel::Center) * 5.0);
}

// ---- DSP graph ----

#[test]
fn gain_node_scales_samples() {
    let mut graph = DspGraph::new();
    graph.push(Gain(0.5));
    let mut samples = [1.0, 2.0, -4.0];
    graph.process(&mut samples);
    assert_eq!(samples, [0.5, 1.0, -2.0]);
}

#[test]
fn low_pass_filter_smooths_a_step_input_toward_target() {
    let mut filter = LowPassFilter::new(0.1);
    let mut samples = [1.0; 50];
    filter.process(&mut samples);
    assert!(samples[0] < 0.5, "first sample should barely move from 0");
    assert!(
        samples[49] > 0.98,
        "after many samples the filter should have converged near 1.0"
    );
}

#[test]
fn dsp_graph_chains_nodes_in_order() {
    let mut graph = DspGraph::new();
    graph.push(Gain(2.0));
    graph.push(Gain(3.0));
    let mut samples = [1.0];
    graph.process(&mut samples);
    assert_eq!(samples, [6.0]);
}

// ---- fold_to_front_hemisphere itself ----

#[test]
fn fold_maps_directly_behind_to_directly_ahead() {
    assert!((fold_to_front_hemisphere(180.0) - 0.0).abs() < 1e-4);
}

#[test]
fn fold_is_identity_within_the_front_hemisphere() {
    assert_eq!(fold_to_front_hemisphere(45.0), 45.0);
    assert_eq!(fold_to_front_hemisphere(-45.0), -45.0);
}

#[test]
fn fold_is_continuous_at_the_ninety_degree_boundary() {
    assert!((fold_to_front_hemisphere(90.0) - fold_to_front_hemisphere(90.001)).abs() < 0.01);
    assert!((fold_to_front_hemisphere(-90.0) - fold_to_front_hemisphere(-90.001)).abs() < 0.01);
}
