//! Roadmap milestone: real, audible audio output — `engine-core`'s frame
//! loop feeding `audio-core`'s mixer into `audio-driver`'s device stream
//! (via `audio-core::AudioOutput`, the one core→own-driver bridge). A
//! 440 Hz tone orbits the listener for a few seconds: you should hear it
//! pan front → right → behind (folded to front on stereo — see
//! `fold_to_front_hemisphere`) → left, getting slightly quieter at the
//! orbit's far side.
//!
//! The loop's real-time pacing comes from the stream itself:
//! `AudioOutput::push_interleaved` blocks when the ring buffer is full,
//! so rendering one block per iteration runs at exactly the hardware's
//! playback rate — no sleep() needed. This is the composition pattern the
//! workspace uses for drivers generally (see `spinning_cube` for the
//! graphics equivalent): the app owns the driver-facing output,
//! `Runtime` stays driver-free.
//!
//! Run with:
//!   ./build.sh run audible_scene

use meridian_audio_core::{AudioOutput, Emitter, Mixer, SpeakerLayout};
use meridian_engine_core::{Runtime, SubsystemManager};
use meridian_gac_core::{Motor3, Vec3};

const SAMPLE_RATE: u32 = 48_000;
const BLOCK_FRAMES: usize = 800; // 1/60 s — one render block per tick
const ORBIT_SECONDS: f32 = 8.0;
const ORBIT_RADIUS: f32 = 3.0;
const TONE_HZ: f32 = 440.0;
const AMPLITUDE: f32 = 0.3;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mixer = Mixer::new(SpeakerLayout::stereo_speakers());
    let output = match AudioOutput::open(&mixer.layout, SAMPLE_RATE, None).await {
        Ok(output) => output,
        Err(err) => {
            println!("skipping: no audio output device available ({err})");
            return;
        }
    };

    let mut runtime = Runtime::new(SubsystemManager::new(mixer));
    runtime
        .subsystems
        .emitters
        .push((Emitter::default(), AMPLITUDE));

    println!("playing: 440 Hz tone orbiting the listener for {ORBIT_SECONDS} s ...");

    // The orbit and oscillator advance on the *audio* clock (samples
    // pushed so far), not Runtime's wall clock — the sound stays smooth
    // even if a tick is late. Runtime::tick still runs once per block, as
    // it would in a real game frame.
    let mut phase = 0.0f32;
    let mut block = vec![0.0f32; BLOCK_FRAMES];
    let total_blocks = (ORBIT_SECONDS * SAMPLE_RATE as f32) as usize / BLOCK_FRAMES;

    for block_index in 0..total_blocks {
        runtime.tick();

        let seconds = (block_index * BLOCK_FRAMES) as f32 / SAMPLE_RATE as f32;
        // One full orbit over the run: front (+X) → right (+Z) → back → left.
        let angle = seconds / ORBIT_SECONDS * std::f32::consts::TAU;
        let position = Vec3::new(ORBIT_RADIUS * angle.cos(), 0.0, ORBIT_RADIUS * angle.sin());
        runtime.subsystems.emitters[0].0 = Emitter {
            frame: Motor3::translation(position),
        };

        for sample in block.iter_mut() {
            phase = (phase + TONE_HZ / SAMPLE_RATE as f32).fract();
            *sample = AMPLITUDE * (phase * std::f32::consts::TAU).sin();
        }

        let emitter = runtime.subsystems.emitters[0].0;
        let interleaved = runtime.subsystems.mixer.render_interleaved(
            &runtime.subsystems.listener,
            &[(emitter, &block)],
            BLOCK_FRAMES,
        );
        // Blocks when the ring buffer is full — this is what paces the loop.
        output.push_interleaved(&interleaved);

        if block_index % 60 == 0 {
            println!(
                "  t={seconds:>4.1}s  azimuth={:>6.1}°  emitter at ({:+.2}, 0, {:+.2})",
                angle.to_degrees(),
                position.x,
                position.z
            );
        }
    }

    println!("done — one full orbit played.");
}
