//! Incremental ("streaming") audio decoding — see
//! docs/adr/014-streaming-audio-decode.md.
//!
//! Eager decoding ([`AnyAudioDecoder`](crate::AnyAudioDecoder)) holds
//! the *entire* decoded PCM in memory — roughly 10× the compressed
//! size; a few minutes of music is tens of megabytes. Streaming keeps
//! the compressed bytes and decodes one bounded block at a time, so
//! memory stays at compressed-size + one block regardless of track
//! length. This is incremental *decode*, not incremental I/O: the
//! compressed bytes still arrive complete, per this crate's
//! bytes-in/data-out contract (no filesystem, no lifetime policy).
//!
//! [`DecodeStrategy`] is the tunable front door: force either mode, or
//! let `Auto` pick by comparing the *estimated decoded size* (from
//! container metadata) against a configurable threshold — with
//! unknown-length tracks streaming by definition (an unbounded decode
//! can't be safely eager). The block size is the "buffer" knob.

use std::sync::Arc;

use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

use crate::compressed_audio::{AudioFormat, codec_registry};
use crate::{AudioData, DecodeError, Decoder, WavDecoder};

/// Which decode path [`open_audio`] takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DecodeMode {
    /// Decide from the estimated decoded size vs.
    /// [`DecodeStrategy::auto_threshold_bytes`]; unknown-length tracks
    /// stream.
    #[default]
    Auto,
    /// Always decode the whole track into one [`AudioData`].
    ForceEager,
    /// Always stream, no matter how small the track.
    ForceStreaming,
}

/// Tunable decode strategy for [`open_audio`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeStrategy {
    pub mode: DecodeMode,
    /// `Auto` streams once the estimated decoded size (PCM `i16`)
    /// exceeds this. Default 16 MiB ≈ 1.5 min of 48 kHz stereo.
    pub auto_threshold_bytes: u64,
    /// PCM frames per [`StreamingAudioDecoder::next_block`] — the
    /// streaming buffer size. Default 4096 (~85 ms at 48 kHz).
    pub block_frames: usize,
}

impl Default for DecodeStrategy {
    fn default() -> Self {
        Self {
            mode: DecodeMode::Auto,
            auto_threshold_bytes: 16 * 1024 * 1024,
            block_frames: 4096,
        }
    }
}

/// What [`open_audio`] produced: the whole track, or an incremental
/// decoder to pull it from block by block.
pub enum AudioAsset {
    Decoded(AudioData),
    Streaming(StreamingAudioDecoder),
}

impl AudioAsset {
    pub fn sample_rate(&self) -> u32 {
        match self {
            AudioAsset::Decoded(audio) => audio.sample_rate,
            AudioAsset::Streaming(stream) => stream.sample_rate(),
        }
    }

    pub fn channels(&self) -> u16 {
        match self {
            AudioAsset::Decoded(audio) => audio.channels,
            AudioAsset::Streaming(stream) => stream.channels(),
        }
    }
}

enum StreamBackend {
    /// Compressed formats: a live symphonia demux/decode pipeline.
    Symphonia {
        format: Box<dyn FormatReader>,
        decoder: Box<dyn AudioDecoder>,
        track_id: u32,
        /// Decoded samples beyond the last served block boundary.
        pending: Vec<i16>,
        finished: bool,
    },
    /// WAV is already PCM — "streaming" serves views of the parsed data
    /// (there is nothing to incrementally decode, but the caller gets
    /// one uniform block API for every format).
    Wav { audio: AudioData, cursor: usize },
}

/// Incremental audio decoder: identifies the format by signature (never
/// by extension), then yields interleaved-`i16` blocks of at most the
/// configured size via [`next_block`](Self::next_block). [`rewind`]
/// (Self::rewind) restarts from the beginning without re-copying the
/// compressed bytes (they're shared, not cloned) — the loop-playback
/// primitive.
pub struct StreamingAudioDecoder {
    bytes: Arc<[u8]>,
    backend: StreamBackend,
    sample_rate: u32,
    channels: u16,
    /// Total frames per container metadata, if the container knows.
    total_frames: Option<u64>,
    block_frames: usize,
}

impl StreamingAudioDecoder {
    pub fn new(bytes: &[u8], block_frames: usize) -> Result<Self, DecodeError> {
        Self::from_shared(Arc::from(bytes), block_frames)
    }

    fn from_shared(bytes: Arc<[u8]>, block_frames: usize) -> Result<Self, DecodeError> {
        let block_frames = block_frames.max(1);
        match AudioFormat::detect(&bytes) {
            None => Err(DecodeError::Unsupported(
                "unrecognized audio signature (not WAV/MP3/OGG-Vorbis/OGG-Opus/FLAC)",
            )),
            Some(AudioFormat::Wav) => {
                let audio = WavDecoder.decode(&bytes)?;
                Ok(Self {
                    sample_rate: audio.sample_rate,
                    channels: audio.channels,
                    total_frames: Some(audio.samples.len() as u64 / audio.channels.max(1) as u64),
                    backend: StreamBackend::Wav { audio, cursor: 0 },
                    bytes,
                    block_frames,
                })
            }
            Some(_) => {
                let (format, decoder, track_id, total_frames, params_rate, params_channels) =
                    open_symphonia(&bytes)?;
                let mut this = Self {
                    bytes,
                    backend: StreamBackend::Symphonia {
                        format,
                        decoder,
                        track_id,
                        pending: Vec::new(),
                        finished: false,
                    },
                    sample_rate: params_rate.unwrap_or(0),
                    channels: params_channels.unwrap_or(0),
                    total_frames,
                    block_frames,
                };
                // Codec params may omit rate/channels (e.g. some OGG
                // mappings) — prime one packet so both are authoritative
                // before the caller opens an output stream.
                if this.sample_rate == 0 || this.channels == 0 {
                    this.fill_pending_to(1)?;
                }
                if this.sample_rate == 0 || this.channels == 0 {
                    return Err(DecodeError::Malformed(
                        "stream carries no sample rate/channel information",
                    ));
                }
                Ok(this)
            }
        }
    }

    /// Output sample rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Output (interleaved) channel count.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Total track length in frames, when the container declares it.
    pub fn total_frames(&self) -> Option<u64> {
        self.total_frames
    }

    /// Estimated size of the fully decoded track in bytes (PCM `i16`),
    /// when the container declares its length — what `Auto` mode
    /// compares against its threshold.
    pub fn estimated_decoded_bytes(&self) -> Option<u64> {
        self.total_frames
            .map(|frames| frames * self.channels as u64 * 2)
    }

    /// Decodes packets until `pending` holds at least `samples` samples
    /// or the track ends.
    fn fill_pending_to(&mut self, samples: usize) -> Result<(), DecodeError> {
        let StreamBackend::Symphonia {
            format,
            decoder,
            track_id,
            pending,
            finished,
        } = &mut self.backend
        else {
            return Ok(());
        };
        let mut block = Vec::new();
        while !*finished && pending.len() < samples {
            let Some(packet) = format
                .next_packet()
                .map_err(|e| DecodeError::Codec(e.to_string()))?
            else {
                *finished = true;
                break;
            };
            if packet.track_id != *track_id {
                continue;
            }
            let decoded = decoder
                .decode(&packet)
                .map_err(|e| DecodeError::Codec(e.to_string()))?;
            let spec = decoded.spec();
            self.sample_rate = spec.rate();
            self.channels = spec.channels().count() as u16;
            decoded.copy_to_vec_interleaved::<i16>(&mut block);
            pending.extend_from_slice(&block);
        }
        Ok(())
    }

    /// The next block of interleaved samples — at most `block_frames`
    /// frames; shorter only at the end of the track. `Ok(None)` means
    /// the track is finished (use [`rewind`](Self::rewind) to loop).
    pub fn next_block(&mut self) -> Result<Option<Vec<i16>>, DecodeError> {
        let want = self.block_frames * self.channels.max(1) as usize;
        match &mut self.backend {
            StreamBackend::Wav { audio, cursor } => {
                if *cursor >= audio.samples.len() {
                    return Ok(None);
                }
                let end = (*cursor + want).min(audio.samples.len());
                let block = audio.samples[*cursor..end].to_vec();
                *cursor = end;
                Ok(Some(block))
            }
            StreamBackend::Symphonia { .. } => {
                self.fill_pending_to(want)?;
                let StreamBackend::Symphonia {
                    pending, finished, ..
                } = &mut self.backend
                else {
                    unreachable!()
                };
                if pending.is_empty() {
                    return Ok(if *finished { None } else { Some(Vec::new()) });
                }
                let take = want.min(pending.len());
                let block = pending.drain(..take).collect();
                Ok(Some(block))
            }
        }
    }

    /// Restarts decoding from the first frame. The compressed bytes are
    /// shared with the restarted pipeline, not copied.
    pub fn rewind(&mut self) -> Result<(), DecodeError> {
        match &mut self.backend {
            StreamBackend::Wav { cursor, .. } => {
                *cursor = 0;
                Ok(())
            }
            StreamBackend::Symphonia { .. } => {
                let fresh = Self::from_shared(Arc::clone(&self.bytes), self.block_frames)?;
                *self = fresh;
                Ok(())
            }
        }
    }

    /// Drains the remaining blocks into one [`AudioData`] — how `Auto`/
    /// `ForceEager` reuse the already-open pipeline instead of probing
    /// twice.
    pub fn decode_all(mut self) -> Result<AudioData, DecodeError> {
        let mut samples = Vec::new();
        while let Some(block) = self.next_block()? {
            samples.extend_from_slice(&block);
        }
        if samples.is_empty() {
            return Err(DecodeError::Malformed("no decodable audio packets"));
        }
        Ok(AudioData {
            sample_rate: self.sample_rate,
            channels: self.channels,
            samples,
        })
    }
}

/// The standard-library face of streaming: a [`StreamingAudioDecoder`]
/// *is* an iterator of blocks. Rust's own pull-streaming abstraction is
/// `Iterator` (async `Stream` lives outside std and buys nothing here —
/// decoding in-memory bytes is CPU-bound, not I/O), so this crate
/// deliberately implements it instead of inventing an engine-specific
/// `AssetStream` trait with a single implementor; a shared cross-asset
/// streaming shape is deferred until a second streamed asset type
/// actually exists (see ADR 014).
impl Iterator for StreamingAudioDecoder {
    type Item = Result<Vec<i16>, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_block().transpose()
    }
}

/// Probes `bytes` with the shared codec registry pipeline and returns
/// the open reader/decoder pair plus the track's declared metadata.
#[allow(clippy::type_complexity)]
fn open_symphonia(
    bytes: &Arc<[u8]>,
) -> Result<
    (
        Box<dyn FormatReader>,
        Box<dyn AudioDecoder>,
        u32,
        Option<u64>,
        Option<u32>,
        Option<u16>,
    ),
    DecodeError,
> {
    let source = Box::new(std::io::Cursor::new(Arc::clone(bytes)));
    let stream = MediaSourceStream::new(source, Default::default());
    let format = symphonia::default::get_probe()
        .probe(
            &Hint::new(),
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| DecodeError::Codec(e.to_string()))?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or(DecodeError::Unsupported("no audio track in container"))?;
    let track_id = track.id;
    let total_frames = track.num_frames;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or(DecodeError::Unsupported("no audio codec parameters"))?;
    let params_rate = params.sample_rate;
    let params_channels = params.channels.as_ref().map(|c| c.count() as u16);

    let decoder = codec_registry()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|e| DecodeError::Codec(e.to_string()))?;

    Ok((
        format,
        decoder,
        track_id,
        total_frames,
        params_rate,
        params_channels,
    ))
}

/// The strategy-driven front door: sniff the format by signature, then
/// decode eagerly or hand back a [`StreamingAudioDecoder`] per
/// `strategy` (see [`DecodeMode`]). In `Auto` mode a track whose length
/// the container doesn't declare streams — an unbounded decode can't be
/// safely eager.
pub fn open_audio(bytes: &[u8], strategy: &DecodeStrategy) -> Result<AudioAsset, DecodeError> {
    let stream = StreamingAudioDecoder::new(bytes, strategy.block_frames)?;
    let eager = match strategy.mode {
        DecodeMode::ForceEager => true,
        DecodeMode::ForceStreaming => false,
        DecodeMode::Auto => stream
            .estimated_decoded_bytes()
            .is_some_and(|bytes| bytes <= strategy.auto_threshold_bytes),
    };
    if eager {
        Ok(AudioAsset::Decoded(stream.decode_all()?))
    } else {
        Ok(AudioAsset::Streaming(stream))
    }
}
