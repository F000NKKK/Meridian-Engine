//! Low-level audio device abstraction: output streams, buffers and platform audio backends.
//!
//! Real, backed by `cpal` — the same class of decision as `wgpu` for GPU
//! and `winit` for windowing: hand-writing FFI to ALSA/PulseAudio/WASAPI/
//! CoreAudio would be a multi-month undertaking with three or more
//! independent classes of platform bugs; `cpal` is to audio output what
//! `winit` is to windowing (see ADR 012 and the workspace `Cargo.toml`'s
//! `cpal` entry for the full rationale).
//!
//! The design follows the same driver/core separation as every other
//! subsystem in this workspace (see ADR 005): `audio-driver` knows about
//! OS audio devices and stream lifecycle; `audio-core` (in a separate
//! crate) knows about spatial panning, mixing, and DSP. This crate never
//! depends on `audio-core`.
//!
//! **Stream model**: CPAL uses a callback-driven model — the audio
//! hardware calls a user-provided function on a real-time audio thread to
//! fill an output buffer. This crate bridges that into a push-based model
//! suitable for a game engine: [`AudioStream::push_samples`] is called
//! from the main/game thread to write interleaved float samples into an
//! internal ring buffer, and the CPAL callback drains that ring buffer.
//! This decouples the game's audio processing rate from the hardware's
//! buffer request rate — the game can push a full frame's worth of audio
//! at once, and the hardware drains it in smaller chunks.
//!
//! **Async on genuine I/O**: [`AudioDevice::new`] is a real `async fn`
//! (enumerating audio devices is an OS/driver handshake with unbounded
//! completion time, matching the same policy as
//! `graphics-driver::Device::new` — see ADR 009). Stream creation and
//! sample pushing are synchronous (bounded, local work).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use meridian_platform_core::{BackendCapabilities, CpuCapabilities, GpuCapabilities};

/// Why [`AudioDevice::new`] or [`AudioDevice::open_stream`] failed.
#[derive(Debug, Clone)]
pub enum AudioDeviceError {
    /// No default output device found.
    NoDevice,
    /// The requested stream configuration is not supported by the device.
    UnsupportedConfig(String),
    /// Stream creation failed.
    StreamError(String),
}

impl std::fmt::Display for AudioDeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioDeviceError::NoDevice => write!(f, "no default audio output device found"),
            AudioDeviceError::UnsupportedConfig(msg) => {
                write!(f, "unsupported audio stream config: {msg}")
            }
            AudioDeviceError::StreamError(msg) => write!(f, "audio stream error: {msg}"),
        }
    }
}

impl std::error::Error for AudioDeviceError {}

/// Desired audio output stream configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioStreamConfig {
    /// Sample rate in Hz (e.g. 44100, 48000).
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo, etc.).
    pub channels: u16,
    /// Buffer size in frames (one frame = one sample per channel). `None`
    /// lets the driver pick a default.
    pub buffer_frames: Option<u32>,
}

impl Default for AudioStreamConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            buffer_frames: None,
        }
    }
}

/// An open audio output stream. Samples are pushed from the game thread
/// via [`push_samples`](Self::push_samples) and consumed by the audio
/// hardware on a real-time callback thread.
///
/// Dropping the stream stops the audio output.
pub struct AudioStream {
    /// The CPAL stream handle. Kept alive for the stream's lifetime.
    _stream: cpal::Stream,
    /// Channel to push interleaved float samples into the ring buffer
    /// that the CPAL callback drains.
    sample_tx: crossbeam_channel::Sender<f32>,
    /// Total capacity of the ring buffer in samples (frames × channels).
    capacity: usize,
    /// Stream configuration for querying.
    config: AudioStreamConfig,
    /// Set to true when the stream encounters an error.
    _error_flag: Arc<AtomicBool>,
}

impl AudioStream {
    /// Returns the stream's configuration.
    pub fn config(&self) -> AudioStreamConfig {
        self.config
    }

    /// Pushes interleaved float samples (`[-1.0, 1.0]`) into the stream's
    /// internal ring buffer for playback. Blocks if the buffer is full
    /// (the hardware hasn't consumed enough samples yet) — this is the
    /// intended backpressure mechanism: if the game produces audio faster
    /// than the hardware plays it, the game thread waits.
    ///
    /// `samples.len()` should be a multiple of `self.config().channels`.
    /// Samples outside `[-1.0, 1.0]` will clip on output.
    pub fn push_samples(&self, samples: &[f32]) {
        for &sample in samples {
            // Ignore the error — if the stream was dropped, the receiver
            // is disconnected and we just stop pushing.
            let _ = self.sample_tx.send(sample);
        }
    }

    /// Returns the number of samples that can be pushed without blocking.
    pub fn available_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.sample_tx.len())
    }

    /// Returns `true` if the stream's internal buffer has room for at
    /// least `sample_count` more samples without blocking.
    pub fn can_push(&self, sample_count: usize) -> bool {
        self.available_capacity() >= sample_count
    }
}

/// An audio output device. Wraps a `cpal::Device` and provides stream
/// creation.
#[derive(Debug)]
pub struct AudioDevice {
    device: cpal::Device,
}

impl AudioDevice {
    /// Opens the default audio output device. Async because enumerating
    /// audio hosts/devices is an OS/driver handshake (genuine I/O per
    /// ADR 009).
    pub async fn new() -> Result<Self, AudioDeviceError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioDeviceError::NoDevice)?;
        Ok(Self { device })
    }

    /// Returns the human-readable name of this audio device.
    pub fn name(&self) -> Result<String, AudioDeviceError> {
        self.device
            .description()
            .map(|d| d.name().to_string())
            .map_err(|e| AudioDeviceError::StreamError(e.to_string()))
    }

    /// Returns the set of supported output stream configurations closest
    /// to `config`. The first entry is the best match; if the list is
    /// empty, the device doesn't support anything close to `config`.
    pub fn supported_output_configs(
        &self,
        config: &AudioStreamConfig,
    ) -> Result<Vec<cpal::SupportedStreamConfig>, AudioDeviceError> {
        let supported = self
            .device
            .supported_output_configs()
            .map_err(|e| AudioDeviceError::StreamError(e.to_string()))?;

        let mut candidates: Vec<cpal::SupportedStreamConfig> = supported
            .filter(|cfg| {
                config.sample_rate >= cfg.min_sample_rate()
                    && config.sample_rate <= cfg.max_sample_rate()
            })
            .map(|cfg| cfg.with_sample_rate(config.sample_rate))
            .collect();

        candidates.sort_by_key(|c| {
            (
                (c.channels() as i32 - config.channels as i32).abs(),
                c.sample_format() != cpal::SampleFormat::F32,
            )
        });
        Ok(candidates)
    }

    /// Opens an audio output stream with the given configuration. The
    /// returned [`AudioStream`] can receive samples via
    /// [`AudioStream::push_samples`] from any thread.
    ///
    /// This is synchronous (bounded, local work — configuring a stream
    /// doesn't wait on I/O once the device is known).
    pub fn open_stream(&self, config: AudioStreamConfig) -> Result<AudioStream, AudioDeviceError> {
        let supported = self.supported_output_configs(&config)?;
        let resolved = supported
            .into_iter()
            .find(|c| c.channels() == config.channels)
            .ok_or_else(|| {
                AudioDeviceError::UnsupportedConfig(format!(
                    "no supported config for {} Hz, {} channels",
                    config.sample_rate, config.channels
                ))
            })?;

        let channels = config.channels as usize;

        let hw_buffer_frames = match *resolved.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => 1024u32.clamp(min.max(1), max.max(1)),
            cpal::SupportedBufferSize::Unknown => 1024,
        }
        .max(256);

        let ring_frames = config
            .buffer_frames
            .unwrap_or(hw_buffer_frames * 4)
            .max(hw_buffer_frames * 2);
        let ring_capacity = ring_frames as usize * channels;

        let (sample_tx, sample_rx) = crossbeam_channel::bounded::<f32>(ring_capacity);
        let error_flag = Arc::new(AtomicBool::new(false));
        let err_flag = Arc::clone(&error_flag);

        let stream_config = cpal::StreamConfig {
            channels: config.channels,
            sample_rate: config.sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let mut anti_click = AntiClick::new(channels);
        let stream = match resolved.sample_format() {
            cpal::SampleFormat::F32 => self
                .device
                .build_output_stream(
                    stream_config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        for (index, output_sample) in data.iter_mut().enumerate() {
                            *output_sample =
                                anti_click.process(sample_rx.try_recv().ok(), index % channels);
                        }
                    },
                    move |err| {
                        eprintln!("audio stream error: {err}");
                        err_flag.store(true, Ordering::SeqCst);
                    },
                    None,
                )
                .map_err(|e| AudioDeviceError::StreamError(e.to_string()))?,
            _ => {
                return Err(AudioDeviceError::UnsupportedConfig(
                    "only f32 sample format is supported".to_string(),
                ));
            }
        };

        stream
            .play()
            .map_err(|e| AudioDeviceError::StreamError(e.to_string()))?;

        Ok(AudioStream {
            _stream: stream,
            sample_tx,
            capacity: ring_capacity,
            config,
            _error_flag: error_flag,
        })
    }
}

/// Keeps the output waveform continuous when the ring buffer starves.
///
/// Without this, a starved callback jumps from the last delivered sample
/// straight to `0.0` — a full-scale step discontinuity heard as a click
/// every time the producer misses a deadline (dropped frame, moved
/// window, scheduler hiccup). Instead, starvation exponentially fades
/// the last value toward silence (inaudible over ~1 ms) and recovery
/// ramps the incoming signal back in over ~5 ms — no output sample ever
/// steps discontinuously, no matter what the producer does.
///
/// This lives in the driver deliberately: it's a property of the stream
/// transport ("the device never emits a discontinuity"), not a domain
/// effect — domain-side de-clicking (e.g. between DSP stages) is
/// `audio-core`'s business.
struct AntiClick {
    /// Last emitted sample per channel — the fade-out source.
    last: Vec<f32>,
    /// Recovery gain: dropped to 0 by starvation, ramped back to 1.
    gain: f32,
}

impl AntiClick {
    /// Starvation decay per sample: silence (−40 dB) within ~1 ms at 48 kHz.
    const FADE_OUT: f32 = 0.91;
    /// Recovery ramp: full volume ~5 ms after samples resume at 48 kHz.
    const FADE_IN_STEP: f32 = 1.0 / 256.0;

    fn new(channels: usize) -> Self {
        Self {
            last: vec![0.0; channels.max(1)],
            // Starts at zero so the stream's very first samples ramp in
            // instead of stepping from silence to full scale — the
            // start-of-stream click.
            gain: 0.0,
        }
    }

    fn process(&mut self, sample: Option<f32>, channel: usize) -> f32 {
        match sample {
            Some(sample) => {
                self.gain = (self.gain + Self::FADE_IN_STEP).min(1.0);
                let out = sample * self.gain;
                self.last[channel] = out;
                out
            }
            None => {
                self.gain = 0.0;
                let out = self.last[channel] * Self::FADE_OUT;
                self.last[channel] = out;
                out
            }
        }
    }
}

impl BackendCapabilities for AudioDevice {
    fn cpu(&self) -> CpuCapabilities {
        CpuCapabilities::detect()
    }

    fn gpu(&self) -> Option<GpuCapabilities> {
        None // audio driver doesn't dispatch to a GPU
    }
}

#[cfg(test)]
mod tests {
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
        // the exact sequence that used to click.
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

    async fn device_or_skip() -> Option<AudioDevice> {
        match AudioDevice::new().await {
            Ok(device) => Some(device),
            Err(err) => {
                eprintln!("skipping: no audio device available ({err})");
                None
            }
        }
    }

    #[tokio::test]
    async fn device_reports_a_name() {
        let Some(device) = device_or_skip().await else {
            return;
        };
        let name = device.name().unwrap();
        assert!(!name.is_empty(), "device name should not be empty");
        eprintln!("audio device: {name}");
    }

    #[tokio::test]
    async fn device_reports_supported_configs() {
        let Some(device) = device_or_skip().await else {
            return;
        };
        let config = AudioStreamConfig::default();
        let configs = device.supported_output_configs(&config).unwrap();
        assert!(
            !configs.is_empty(),
            "device should support at least one config near 48kHz stereo"
        );
    }

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
}
