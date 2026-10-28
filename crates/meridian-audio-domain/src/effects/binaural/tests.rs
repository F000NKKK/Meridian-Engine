//! Tests of this module.

use meridian_gac::{Motor3, Vec3};

use super::*;

const RATE: u32 = 48_000;

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

/// No distance attenuation — directional behavior only.
fn renderer() -> BinauralRenderer {
    BinauralRenderer::new(RATE).with_attenuation(AttenuationModel {
        reference_distance: 1000.0,
        rolloff: 1.0,
        max_distance: 1000.0,
        ..AttenuationModel::default()
    })
}

fn rms(samples: impl Iterator<Item = f32>) -> f32 {
    let (sum, n) = samples.fold((0.0f32, 0usize), |(s, n), x| (s + x * x, n + 1));
    (sum / n.max(1) as f32).sqrt()
}

fn left(out: &[f32]) -> impl Iterator<Item = f32> + '_ {
    out.chunks_exact(2).map(|f| f[0])
}

fn right(out: &[f32]) -> impl Iterator<Item = f32> + '_ {
    out.chunks_exact(2).map(|f| f[1])
}

#[test]
fn source_on_the_right_is_louder_in_the_right_ear() {
    let mut r = renderer();
    let source = vec![0.5f32; 2048];
    let out = r.render(
        &listener(),
        &[(emitter_at(Vec3::new(0.0, 0.0, 3.0)), &source)],
        2048,
    );
    assert!(rms(right(&out)) > 4.0 * rms(left(&out)));
}

#[test]
fn source_on_the_right_reaches_the_left_ear_later() {
    let mut r = renderer();
    // An impulse after warmup silence; find each ear's peak time.
    let mut source = vec![0.0f32; 256];
    source[64] = 1.0;
    // ~70 deg right: strongly lateral but neither ear is muted (a
    // hard-side source zeroes the far ear's gain, making its peak
    // index meaningless).
    let out = r.render(
        &listener(),
        &[(emitter_at(Vec3::new(1.03, 0.0, 2.82)), &source)],
        256,
    );
    let peak_index = |it: &mut dyn Iterator<Item = f32>| {
        it.enumerate()
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
            .map(|(i, _)| i)
            .unwrap()
    };
    let left_peak = peak_index(&mut left(&out));
    let right_peak = peak_index(&mut right(&out));
    // Full-right ITD at 48 kHz is ~29 samples.
    assert!(
        left_peak >= right_peak + 20,
        "left peak {left_peak} should trail right peak {right_peak}"
    );
}

#[test]
fn behind_is_quieter_than_in_front() {
    let source = vec![0.5f32; 2048];
    let mut front_renderer = renderer();
    let front = front_renderer.render(
        &listener(),
        &[(emitter_at(Vec3::new(3.0, 0.0, 0.0)), &source)],
        2048,
    );
    let mut rear_renderer = renderer();
    let rear = rear_renderer.render(
        &listener(),
        &[(emitter_at(Vec3::new(-3.0, 0.0, 0.0)), &source)],
        2048,
    );
    let total = |out: &[f32]| rms(out.iter().copied());
    assert!(
        total(&rear) < 0.85 * total(&front),
        "rear {} vs front {}",
        total(&rear),
        total(&front)
    );
}

#[test]
fn shadowed_ear_loses_high_frequencies_more_than_the_near_ear() {
    // Nyquist-rate alternation is the highest frequency the stream
    // can carry; compare each ear's response to it against its
    // response to a constant, per ear — the far (shadowed) ear must
    // pass proportionally far less of the high frequency.
    let high: Vec<f32> = (0..4096)
        .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
        .collect();
    let low = vec![0.5f32; 4096];
    let position = Vec3::new(1.03, 0.0, 2.82); // ~70° right — left ear shadowed but not silent

    let mut r_high = renderer();
    let out_high = r_high.render(&listener(), &[(emitter_at(position), &high)], 4096);
    let mut r_low = renderer();
    let out_low = r_low.render(&listener(), &[(emitter_at(position), &low)], 4096);

    let left_ratio = rms(left(&out_high)) / rms(left(&out_low));
    let right_ratio = rms(right(&out_high)) / rms(right(&out_low));
    assert!(
        left_ratio < 0.5 * right_ratio,
        "shadowed left ratio {left_ratio} vs near right ratio {right_ratio}"
    );
}

#[test]
fn consecutive_blocks_with_a_pose_jump_stay_smooth() {
    // Rotate the listener 180° between blocks — the gain change per
    // output sample must stay tiny (ramped), never a step. A step is
    // the crackle/zipper artifact this renderer exists to avoid.
    let source: Vec<f32> = (0..2400)
        .map(|i| (i as f32 * std::f32::consts::TAU * 220.0 / RATE as f32).sin() * 0.5)
        .collect();
    let mut r = renderer();
    let emitter = emitter_at(Vec3::new(0.0, 0.0, 3.0));

    let block_a = r.render(&listener(), &[(emitter, &source)], 2400);
    let turned = Listener {
        frame: Motor3::from_rotation_translation(
            meridian_gac::Rotor::from_axis_angle(Vec3::Y, std::f32::consts::PI),
            Vec3::ZERO,
        ),
    };
    let block_b = r.render(&turned, &[(emitter, &source)], 2400);

    let last_a = &block_a[block_a.len() - 2..];
    let first_b = &block_b[..2];
    for ear in 0..2 {
        let step = (first_b[ear] - last_a[ear]).abs();
        assert!(
            step < 0.05,
            "ear {ear} stepped by {step} across the block boundary"
        );
    }
    // And within the second block no adjacent-sample jump exceeds
    // what a 220 Hz sine with slowly ramping parameters can produce.
    for ear in 0..2 {
        let channel: Vec<f32> = block_b.chunks_exact(2).map(|f| f[ear]).collect();
        for pair in channel.windows(2) {
            assert!(
                (pair[1] - pair[0]).abs() < 0.08,
                "intra-block step {}",
                (pair[1] - pair[0]).abs()
            );
        }
    }
}

#[test]
fn identical_consecutive_blocks_continue_seamlessly() {
    // Same pose, same sine phase-continued across two blocks: the
    // boundary must look like any other pair of adjacent samples.
    let sine = |offset: usize| -> Vec<f32> {
        (0..1200)
            .map(|i| {
                ((offset + i) as f32 * std::f32::consts::TAU * 330.0 / RATE as f32).sin() * 0.4
            })
            .collect()
    };
    let mut r = renderer();
    let emitter = emitter_at(Vec3::new(2.0, 0.0, 1.0));
    let a = r.render(&listener(), &[(emitter, &sine(0))], 1200);
    let b = r.render(&listener(), &[(emitter, &sine(1200))], 1200);
    for ear in 0..2 {
        let step = (b[ear] - a[a.len() - 2 + ear]).abs();
        assert!(step < 0.03, "ear {ear} boundary step {step}");
    }
}

#[test]
fn water_shrinks_the_interaural_delay() {
    // Sound crosses the head ~4.4x faster in water than in air, so
    // the ITD shrinks accordingly — the medium, not a constant,
    // decides it.
    let peak_offset = |renderer: &mut BinauralRenderer| {
        let mut source = vec![0.0f32; 256];
        source[64] = 1.0;
        // ~70 deg right: strongly lateral but neither ear is muted.
        let out = renderer.render(
            &listener(),
            &[(emitter_at(Vec3::new(1.03, 0.0, 2.82)), &source)],
            256,
        );
        let peak = |ear: usize| {
            out.chunks_exact(2)
                .map(|f| f[ear])
                .enumerate()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .map(|(i, _)| i)
                .unwrap()
        };
        peak(0) as i64 - peak(1) as i64
    };

    let mut air = renderer();
    let mut water = renderer();
    water.medium = AcousticMedium::fresh_water();
    let air_offset = peak_offset(&mut air);
    let water_offset = peak_offset(&mut water);
    assert!(
        air_offset >= 20,
        "air ITD should be ~29 samples, got {air_offset}"
    );
    assert!(
        water_offset <= air_offset / 3,
        "water ITD {water_offset} should be far below air {air_offset}"
    );
}
