//! Low-level audio device abstraction: output streams, buffers and platform audio backends.
//!
//! Layer: infrastructure (adapter); bounded context: audio
//! (`craftbook.toml`, docs/context-map.md).
//!
//! Real, backed by `cpal` — the same class of decision as `wgpu` for GPU
//! and `winit` for windowing: hand-writing FFI to ALSA/PulseAudio/WASAPI/
//! `CoreAudio` would be a multi-month undertaking with three or more
//! independent classes of platform bugs; `cpal` is to audio output what
//! `winit` is to windowing (see ADR 012 and the workspace `Cargo.toml`'s
//! `cpal` entry for the full rationale).
//!
//! The direction is ADR 019's: `audio-core` owns the [`AudioSink`] port and
//! knows about spatial panning, mixing and DSP; this crate is the adapter
//! that knows about OS audio devices and stream lifecycle, and implements the
//! port with [`AudioStream`].
//!
//! Design notes: docs/notes/meridian-audio-driver.md (`lib`).

#![forbid(unsafe_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use meridian_audio_core::{AudioSink, SpeakerLayout};
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

impl meridian_foundation::EngineError for AudioDeviceError {}

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

impl std::fmt::Debug for AudioStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioStream")
            .field("capacity", &self.capacity)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
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
            // A send error means the stream was dropped and the receiver
            // is disconnected: there is nobody left to play the rest.
            if self.sample_tx.send(sample).is_err() {
                break;
            }
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

impl AudioSink for AudioStream {
    fn push_interleaved(&self, samples: &[f32]) {
        self.push_samples(samples);
    }

    fn can_push(&self, sample_count: usize) -> bool {
        AudioStream::can_push(self, sample_count)
    }
}

/// Opens the default output device with one stream channel per speaker in
/// `layout`, in speaker order: the interleaving
/// [`Mixer::render_interleaved`](meridian_audio_core::Mixer::render_interleaved)
/// produces.
///
/// `buffer_frames` bounds the stream's ring buffer (`None` = the default,
/// several hardware buffers deep). The ring is the audio's latency: an
/// interactive scene that re-mixes against a moving listener should request a
/// small one (the stream still clamps to what the hardware needs to run
/// underrun-free). Async because enumerating the device is genuine I/O
/// (ADR 009).
///
/// # Errors
///
/// [`AudioDeviceError`] when there is no output device or it cannot run the
/// requested configuration.
pub async fn open_output(
    layout: &SpeakerLayout,
    sample_rate: u32,
    buffer_frames: Option<u32>,
) -> Result<AudioStream, AudioDeviceError> {
    let device = AudioDevice::new().await?;
    device.open_stream(AudioStreamConfig {
        sample_rate,
        channels: u16::try_from(layout.speakers.len()).unwrap_or(u16::MAX),
        buffer_frames,
    })
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
        let name = device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|_| "<unknown>".to_string());
        meridian_foundation::log_info!("audio output device opened: {name}");
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
        config: AudioStreamConfig,
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

    /// The supported hardware config whose channel count matches `config`.
    fn resolve_output_config(
        &self,
        config: AudioStreamConfig,
    ) -> Result<cpal::SupportedStreamConfig, AudioDeviceError> {
        self.supported_output_configs(config)?
            .into_iter()
            .find(|c| c.channels() == config.channels)
            .ok_or_else(|| {
                AudioDeviceError::UnsupportedConfig(format!(
                    "no supported config for {} Hz, {} channels",
                    config.sample_rate, config.channels
                ))
            })
    }

    /// Opens an audio output stream with the given configuration. The
    /// returned [`AudioStream`] can receive samples via
    /// [`AudioStream::push_samples`] from any thread.
    ///
    /// This is synchronous (bounded, local work — configuring a stream
    /// doesn't wait on I/O once the device is known).
    pub fn open_stream(&self, config: AudioStreamConfig) -> Result<AudioStream, AudioDeviceError> {
        let resolved = self.resolve_output_config(config)?;

        let channels = config.channels as usize;
        let ring_capacity = ring_capacity(config, &resolved);

        let (sample_tx, sample_rx) = crossbeam_channel::bounded::<f32>(ring_capacity);
        let error_flag = Arc::new(AtomicBool::new(false));
        let err_flag = Arc::clone(&error_flag);

        let stream_config = cpal::StreamConfig {
            channels: config.channels,
            sample_rate: config.sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        if resolved.sample_format() != cpal::SampleFormat::F32 {
            return Err(AudioDeviceError::UnsupportedConfig(
                "only f32 sample format is supported".to_string(),
            ));
        }
        let mut anti_click = AntiClick::new(channels);
        let stream = self
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
                    meridian_foundation::log_error!("audio stream error: {err}");
                    err_flag.store(true, Ordering::SeqCst);
                },
                None,
            )
            .map_err(|e| AudioDeviceError::StreamError(e.to_string()))?;

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

/// Samples the ring buffer holds: at least two hardware buffers, four by
/// default, unless the caller asked for a size.
fn ring_capacity(config: AudioStreamConfig, resolved: &cpal::SupportedStreamConfig) -> usize {
    let hw_buffer_frames = match *resolved.buffer_size() {
        cpal::SupportedBufferSize::Range { min, max } => 1024u32.clamp(min.max(1), max.max(1)),
        cpal::SupportedBufferSize::Unknown => 1024,
    }
    .max(256);
    let ring_frames = config
        .buffer_frames
        .unwrap_or(hw_buffer_frames * 4)
        .max(hw_buffer_frames * 2);
    ring_frames as usize * config.channels as usize
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
mod tests;
