//! Block-by-block audio decoding.

use std::sync::Arc;

use meridian_asset_domain::{AudioClip, AudioFormat, DecodeError, Decoder, WavDecoder};

use super::{StreamBackend, open_symphonia};

/// Incremental audio decoder: identifies the format by signature (never
/// by extension), then yields interleaved-`i16` blocks of at most the
/// configured size via [`next_block`](Self::next_block);
/// [`rewind`](Self::rewind) restarts from the beginning without re-copying the
/// compressed bytes (they're shared, not cloned) — the loop-playback
/// primitive.
#[derive(Debug)]
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
    /// Opens an audio stream, decoding at most `block_frames` per block.
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
                let track = open_symphonia(&bytes)?;
                let mut this = Self {
                    bytes,
                    backend: StreamBackend::Symphonia {
                        format: track.format,
                        decoder: track.decoder,
                        track_id: track.track_id,
                        pending: Vec::new(),
                        finished: false,
                    },
                    sample_rate: track.sample_rate.unwrap_or(0),
                    channels: track.channels.unwrap_or(0),
                    total_frames: track.total_frames,
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

    /// Drains the remaining blocks into one [`AudioClip`] — how `Auto`/
    /// `ForceEager` reuse the already-open pipeline instead of probing
    /// twice.
    pub fn decode_all(mut self) -> Result<AudioClip, DecodeError> {
        let mut samples = Vec::new();
        while let Some(block) = self.next_block()? {
            samples.extend_from_slice(&block);
        }
        if samples.is_empty() {
            return Err(DecodeError::Malformed("no decodable audio packets"));
        }
        Ok(AudioClip {
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
