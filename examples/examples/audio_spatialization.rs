//! Roadmap milestone: validate `meridian-audio-core`'s spatial panning
//! against the exact scenario it was designed against — a listener
//! (character/camera) at the world origin facing world `+X`, with sound
//! sources placed directly in front of, behind, to the left of, and to
//! the right of it, checked against several output speaker layouts
//! (headphones, narrow stereo speakers, 5.0, 5.1) to show they don't all
//! behave the same way. The exhaustive numeric checks live in
//! `meridian-audio-core`'s own test suite (`cargo test -p
//! meridian-audio-core`); this is the human-readable version.
//!
//! Run with:
//!   ./build.sh run audio_spatialization

use meridian_audio_core::{AttenuationModel, Channel, Emitter, Listener, Mixer, SpeakerLayout};
use meridian_gac_core::{Motor3, Vec3};

fn gain_of(gains: &[(Channel, f32)], channel: Channel) -> f32 {
    gains.iter().find(|(c, _)| *c == channel).map(|(_, g)| *g).unwrap_or(0.0)
}

fn print_gains(label: &str, gains: &[(Channel, f32)]) {
    let parts: Vec<String> = gains.iter().map(|(c, g)| format!("{c:?}={g:.3}")).collect();
    println!("  {label:28} {}", parts.join("  "));
}

fn check(label: &str, condition: bool) {
    println!("    [{}] {label}", if condition { "OK" } else { "FAIL" });
    assert!(condition, "{label} failed");
}

fn main() {
    // No attenuation for the direction-focused sections below — distance
    // is checked separately at the end. Character/listener at the
    // origin, facing +X (see the crate's module doc for the
    // forward=+X/up=+Y/right=+Z convention).
    let listener = Listener { frame: Motor3::identity() };
    let no_attenuation = AttenuationModel { reference_distance: 1000.0, rolloff: 1.0, max_distance: 1000.0 };

    let front = Emitter { frame: Motor3::translation(Vec3::new(5.0, 0.0, 0.0)) };
    let behind = Emitter { frame: Motor3::translation(Vec3::new(-5.0, 0.0, 0.0)) };
    let left = Emitter { frame: Motor3::translation(Vec3::new(0.0, 0.0, -5.0)) };
    let right = Emitter { frame: Motor3::translation(Vec3::new(0.0, 0.0, 5.0)) };

    println!("== Headphones (full ±90° hemisphere) ==");
    let layout = SpeakerLayout::stereo_headphones();
    for (label, emitter) in [("front (5,0,0)", &front), ("behind (-5,0,0)", &behind), ("left (0,0,-5)", &left), ("right (0,0,5)", &right)] {
        let gains = meridian_audio_core::spatial_gains(&listener, emitter, &layout, &no_attenuation);
        print_gains(label, &gains);
    }
    println!("  -> front and behind must produce the SAME centered pan: no HRTF,");
    println!("     amplitude-only stereo genuinely cannot tell front from back.");
    {
        let f = meridian_audio_core::spatial_gains(&listener, &front, &layout, &no_attenuation);
        let b = meridian_audio_core::spatial_gains(&listener, &behind, &layout, &no_attenuation);
        check("front/behind centered gain matches", (gain_of(&f, Channel::Left) - gain_of(&b, Channel::Left)).abs() < 1e-4);
        let l = meridian_audio_core::spatial_gains(&listener, &left, &layout, &no_attenuation);
        check("hard left is (near-)full Left, ~zero Right", gain_of(&l, Channel::Left) > 0.99 && gain_of(&l, Channel::Right) < 1e-3);
    }

    println!("\n== Stereo speakers (narrower ±30° near-field placement) ==");
    let narrow = SpeakerLayout::stereo_speakers();
    for (label, emitter) in [("front (5,0,0)", &front), ("left (0,0,-5)", &left)] {
        let gains = meridian_audio_core::spatial_gains(&listener, emitter, &narrow, &no_attenuation);
        print_gains(label, &gains);
    }
    println!("  -> a source beyond ±30° clamps to fully one channel — no speaker");
    println!("     out there to keep blending toward, unlike headphones' ±90°.");
    {
        let l = meridian_audio_core::spatial_gains(&listener, &left, &narrow, &no_attenuation);
        check("hard left clamps to exactly full Left on narrow speakers too", (gain_of(&l, Channel::Left) - 1.0).abs() < 1e-4);
    }

    println!("\n== 5.0 surround (real rear speakers: L -30° C 0° R 30° SL -110° SR 110°) ==");
    let surround = SpeakerLayout::surround_5_0();
    for (label, emitter) in [("front (5,0,0)", &front), ("behind (-5,0,0)", &behind), ("left (0,0,-5)", &left), ("right (0,0,5)", &right)] {
        let gains = meridian_audio_core::spatial_gains(&listener, emitter, &surround, &no_attenuation);
        print_gains(label, &gains);
    }
    println!("  -> unlike stereo, front and behind are now genuinely distinguishable:");
    println!("     front routes to Center, behind splits across the two Surrounds.");
    {
        let f = meridian_audio_core::spatial_gains(&listener, &front, &surround, &no_attenuation);
        let b = meridian_audio_core::spatial_gains(&listener, &behind, &surround, &no_attenuation);
        check("front -> full Center", (gain_of(&f, Channel::Center) - 1.0).abs() < 1e-4);
        check("behind has zero Center leakage", gain_of(&b, Channel::Center) == 0.0);
        check("behind splits evenly across Surround L/R", (gain_of(&b, Channel::SurroundLeft) - gain_of(&b, Channel::SurroundRight)).abs() < 1e-4);
    }

    println!("\n== 5.1 (5.0 + LFE) — LFE never receives directional content ==");
    let surround51 = SpeakerLayout::surround_5_1();
    for (label, emitter) in [("front (5,0,0)", &front), ("left (0,0,-5)", &left)] {
        let gains = meridian_audio_core::spatial_gains(&listener, emitter, &surround51, &no_attenuation);
        print_gains(label, &gains);
        check("LFE gain is exactly zero", gain_of(&gains, Channel::LowFrequency) == 0.0);
    }

    println!("\n== Multiple simultaneous sources (mixer sums linearly) ==");
    let mixer = Mixer::new(SpeakerLayout::stereo_headphones()).with_attenuation(no_attenuation);
    let mixed = mixer.mix(&listener, &[(left, 1.0), (right, 1.0)]);
    print_gains("left + right sources", &mixed);
    check("simultaneous hard-left + hard-right sums to ~equal L/R", (gain_of(&mixed, Channel::Left) - gain_of(&mixed, Channel::Right)).abs() < 1e-4);

    println!("\n== Distance attenuation ==");
    let real_attenuation = AttenuationModel::default();
    let near = Emitter { frame: Motor3::translation(Vec3::new(2.0, 0.0, 0.0)) };
    let far = Emitter { frame: Motor3::translation(Vec3::new(20.0, 0.0, 0.0)) };
    let mono = SpeakerLayout::mono();
    let near_gain = gain_of(&meridian_audio_core::spatial_gains(&listener, &near, &mono, &real_attenuation), Channel::Center);
    let far_gain = gain_of(&meridian_audio_core::spatial_gains(&listener, &far, &mono, &real_attenuation), Channel::Center);
    println!("  near (d=2): gain={near_gain:.3}   far (d=20): gain={far_gain:.3}");
    check("closer source is louder than farther source", near_gain > far_gain);

    println!("\nAll checks passed — spatial panning and distance attenuation behave correctly across every layout.");
}
