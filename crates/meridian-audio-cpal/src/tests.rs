//! Tests of this crate.

use super::*;

#[test]
fn anti_click_never_steps_on_starvation_or_recovery() {
    let mut anti = AntiClick::new(1);
    let mut previous = 0.0f32;
    let mut max_step = 0.0f32;
    let mut track = |out: f32, max_step: &mut f32| {
        *max_step = max_step.max((out - previous).abs());
        previous = out;
    };

    // Steady loud signal, then sudden starvation, then recovery —
    // the sequence that clicks without the declicker.
    for _ in 0..64 {
        track(anti.process(Some(0.9), 0), &mut max_step);
    }
    for _ in 0..128 {
        track(anti.process(None, 0), &mut max_step);
    }
    for _ in 0..512 {
        track(anti.process(Some(0.9), 0), &mut max_step);
    }
    assert!(
        max_step < 0.1,
        "output stepped by {max_step}; a raw gap would step by 0.9"
    );
}

#[test]
fn anti_click_fades_to_silence_while_starved() {
    let mut anti = AntiClick::new(1);
    anti.process(Some(1.0), 0);
    let mut out = 1.0;
    for _ in 0..96 {
        out = anti.process(None, 0);
    }
    assert!(out.abs() < 1e-3, "still audible after ~2 ms: {out}");
}

#[test]
fn anti_click_is_transparent_in_steady_state() {
    let mut anti = AntiClick::new(2);
    // After the ramp, samples must pass through untouched.
    for _ in 0..600 {
        anti.process(Some(0.5), 0);
        anti.process(Some(-0.5), 1);
    }
    assert_eq!(anti.process(Some(0.123), 0), 0.123);
    assert_eq!(anti.process(Some(-0.321), 1), -0.321);
}

#[cfg(feature = "hw-tests")]
#[test]
fn stream_passes_the_sink_contract() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a current-thread runtime");
    let layout = SpeakerLayout::stereo_headphones();
    let open = || runtime.block_on(open_output(&layout, 48000, None));
    if open().is_err() {
        eprintln!("skipping: no audio device available");
        return;
    }
    meridian_audio_app::testing::run_contract(|| open().expect("the device opened a moment ago"));
}

#[cfg(feature = "hw-tests")]
async fn device_or_skip() -> Option<AudioDevice> {
    match AudioDevice::new().await {
        Ok(device) => Some(device),
        Err(err) => {
            eprintln!("skipping: no audio device available ({err})");
            None
        }
    }
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn device_reports_a_name() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    let name = device.name().unwrap();
    assert!(!name.is_empty(), "device name should not be empty");
    eprintln!("audio device: {name}");
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn device_reports_supported_configs() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    let config = AudioStreamConfig::default();
    let configs = device.supported_output_configs(config).unwrap();
    assert!(
        !configs.is_empty(),
        "device should support at least one config near 48kHz stereo"
    );
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn open_and_close_stream() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    let config = AudioStreamConfig::default();
    let stream = device.open_stream(config).unwrap();
    assert_eq!(stream.config().sample_rate, 48000);
    assert_eq!(stream.config().channels, 2);
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn push_samples_does_not_panic() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    let config = AudioStreamConfig::default();
    let stream = device.open_stream(config).unwrap();
    let samples = vec![0.0f32; 256];
    stream.push_samples(&samples);
    std::thread::sleep(std::time::Duration::from_millis(50));
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn available_capacity_reflects_buffer_size() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    let config = AudioStreamConfig {
        channels: 1,
        buffer_frames: Some(256),
        ..Default::default()
    };
    let stream = device.open_stream(config).unwrap();
    assert!(
        stream.available_capacity() > 0,
        "buffer should have capacity"
    );
    assert!(stream.can_push(128));
}
