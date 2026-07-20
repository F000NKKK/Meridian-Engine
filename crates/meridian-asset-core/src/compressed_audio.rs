//! Compressed-audio decoding: MP3, OGG/Vorbis and FLAC via `symphonia`
//! (pure Rust), Opus via the reference `libopus` plugged into the same
//! symphonia pipeline (`symphonia-adapter-libopus`) — see
//! docs/adr/013-compressed-audio-codecs.md for the dependency decision.
//!
//! Formats are identified by leading magic bytes ([`AudioFormat::detect`]),
//! never by file extension — the file name is not part of any decoder's
//! input. [`AnyAudioDecoder`] is the sniffing front door: detect, then
//! dispatch to the per-format decoder (including the hand-rolled
//! [`WavDecoder`](crate::WavDecoder) for uncompressed PCM).
//!
//! Everything here decodes complete in-memory bytes to a complete
//! [`AudioData`] — the same bytes-in/CPU-data-out contract as every other
//! decoder in this crate (no streaming, no I/O, no lifetime policy).

use crate::{AudioData, DecodeError, Decoder, WavDecoder};

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::codecs::registry::CodecRegistry;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// One codec registry for every compressed format this module decodes:
/// symphonia's own enabled codecs (MP3/Vorbis/FLAC) plus libopus via the
/// adapter — so Opus rides the exact same probe/demux/decode pipeline
/// instead of a hand-written OGG parse.
fn codec_registry() -> &'static CodecRegistry {
    static REGISTRY: std::sync::OnceLock<CodecRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = CodecRegistry::new();
        symphonia::default::register_enabled_codecs(&mut registry);
        registry.register_audio_decoder::<symphonia_adapter_libopus::OpusDecoder>();
        registry
    })
}

/// An audio container/codec identified from leading magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// RIFF/WAVE (`RIFF....WAVE`) — decoded by the hand-rolled
    /// [`WavDecoder`](crate::WavDecoder).
    Wav,
    /// MPEG Layer III: an `ID3` tag or an MPEG frame-sync header.
    Mp3,
    /// OGG container whose first packet is `\x01vorbis`.
    OggVorbis,
    /// OGG container whose first packet is `OpusHead`.
    OggOpus,
    /// Native FLAC (`fLaC`).
    Flac,
}

impl AudioFormat {
    /// Identifies the format from the buffer's leading bytes. Returns
    /// `None` when no known signature matches — extension-based guessing
    /// is deliberately not a fallback.
    pub fn detect(bytes: &[u8]) -> Option<AudioFormat> {
        if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
            return Some(AudioFormat::Wav);
        }
        if bytes.len() >= 4 && &bytes[0..4] == b"fLaC" {
            return Some(AudioFormat::Flac);
        }
        if bytes.len() >= 4 && &bytes[0..4] == b"OggS" {
            // The first packet's payload starts right after the 27-byte
            // page header plus the segment table; its first bytes name
            // the codec ("OpusHead" / "\x01vorbis").
            let payload = bytes
                .get(26)
                .map(|&segments| 27 + segments as usize)
                .and_then(|start| bytes.get(start..));
            if let Some(payload) = payload {
                if payload.starts_with(b"OpusHead") {
                    return Some(AudioFormat::OggOpus);
                }
                if payload.starts_with(b"\x01vorbis") {
                    return Some(AudioFormat::OggVorbis);
                }
            }
            return None;
        }
        if bytes.len() >= 3 && &bytes[0..3] == b"ID3" {
            return Some(AudioFormat::Mp3);
        }
        // Raw MPEG audio frame sync: 11 set bits, then a valid
        // version/layer combination (layer bits `01` = Layer III).
        if bytes.len() >= 2
            && bytes[0] == 0xFF
            && bytes[1] & 0xE0 == 0xE0
            && bytes[1] & 0x06 == 0x02
        {
            return Some(AudioFormat::Mp3);
        }
        None
    }
}

/// Decodes any compressed format `symphonia` handles here (MP3,
/// OGG/Vorbis, FLAC): probe the container, decode the default audio
/// track to completion, convert to interleaved `i16`.
fn decode_with_symphonia(bytes: &[u8]) -> Result<AudioData, DecodeError> {
    let source = Box::new(std::io::Cursor::new(bytes.to_vec()));
    let stream = MediaSourceStream::new(source, Default::default());

    let mut format = symphonia::default::get_probe()
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
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or(DecodeError::Unsupported("no audio codec parameters"))?;

    let mut decoder = codec_registry()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|e| DecodeError::Codec(e.to_string()))?;

    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let mut samples: Vec<i16> = Vec::new();
    let mut block: Vec<i16> = Vec::new();

    while let Some(packet) = format
        .next_packet()
        .map_err(|e| DecodeError::Codec(e.to_string()))?
    {
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = decoder
            .decode(&packet)
            .map_err(|e| DecodeError::Codec(e.to_string()))?;
        let spec = decoded.spec();
        sample_rate = spec.rate();
        channels = spec.channels().count() as u16;
        decoded.copy_to_vec_interleaved::<i16>(&mut block);
        samples.extend_from_slice(&block);
    }

    if samples.is_empty() {
        return Err(DecodeError::Malformed("no decodable audio packets"));
    }
    Ok(AudioData {
        sample_rate,
        channels,
        samples,
    })
}

/// Guards that `bytes` really carries `expected`'s signature before
/// handing them to the codec — so e.g. [`Mp3Decoder`] on OGG bytes fails
/// with `BadMagic`, not a codec-internal error.
fn expect_format(bytes: &[u8], expected: AudioFormat) -> Result<(), DecodeError> {
    match AudioFormat::detect(bytes) {
        Some(found) if found == expected => Ok(()),
        _ => Err(DecodeError::BadMagic {
            expected: match expected {
                AudioFormat::Wav => "RIFF/WAVE",
                AudioFormat::Mp3 => "ID3 or MPEG frame sync",
                AudioFormat::OggVorbis => "OggS + \\x01vorbis",
                AudioFormat::OggOpus => "OggS + OpusHead",
                AudioFormat::Flac => "fLaC",
            },
        }),
    }
}

/// Decodes MPEG Layer III (via `symphonia`, pure Rust).
#[derive(Debug, Default)]
pub struct Mp3Decoder;

impl Decoder<AudioData> for Mp3Decoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioData, DecodeError> {
        expect_format(bytes, AudioFormat::Mp3)?;
        decode_with_symphonia(bytes)
    }
}

/// Decodes OGG/Vorbis (via `symphonia`, pure Rust).
#[derive(Debug, Default)]
pub struct VorbisDecoder;

impl Decoder<AudioData> for VorbisDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioData, DecodeError> {
        expect_format(bytes, AudioFormat::OggVorbis)?;
        decode_with_symphonia(bytes)
    }
}

/// Decodes native FLAC (via `symphonia`, pure Rust).
#[derive(Debug, Default)]
pub struct FlacDecoder;

impl Decoder<AudioData> for FlacDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioData, DecodeError> {
        expect_format(bytes, AudioFormat::Flac)?;
        decode_with_symphonia(bytes)
    }
}

/// Decodes OGG/Opus via the reference `libopus` (see ADR 013 — no mature
/// pure-Rust Opus decoder exists). Mapping family 0 only (mono/stereo,
/// which is what `opusenc`/`ffmpeg` produce for plain music files);
/// output is always 48 kHz, Opus's native decode rate. The `OpusHead`
/// pre-skip is trimmed from the front; end-trimming from the final
/// page's granule position is not applied (a few ms of decoder padding
/// may remain at the tail).
#[derive(Debug, Default)]
pub struct OpusDecoder;

impl Decoder<AudioData> for OpusDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioData, DecodeError> {
        expect_format(bytes, AudioFormat::OggOpus)?;

        let mut reader = ogg::PacketReader::new(std::io::Cursor::new(bytes));
        let head = reader
            .read_packet()
            .map_err(|e| DecodeError::Codec(e.to_string()))?
            .ok_or(DecodeError::Malformed("empty OGG stream"))?;
        if head.data.len() < 19 || !head.data.starts_with(b"OpusHead") {
            return Err(DecodeError::Malformed("first OGG packet is not OpusHead"));
        }
        let channel_count = head.data[9];
        let pre_skip = u16::from_le_bytes([head.data[10], head.data[11]]) as usize;
        let mapping_family = head.data[18];
        if mapping_family != 0 {
            return Err(DecodeError::Unsupported(
                "Opus mapping family other than 0 (mono/stereo)",
            ));
        }
        let channels = match channel_count {
            1 => opus::Channels::Mono,
            2 => opus::Channels::Stereo,
            _ => {
                return Err(DecodeError::Malformed(
                    "OpusHead channel count invalid for mapping family 0",
                ));
            }
        };

        // OpusTags is mandatory as the second packet; tolerate it missing.
        let mut pending = reader
            .read_packet()
            .map_err(|e| DecodeError::Codec(e.to_string()))?;
        if let Some(packet) = &pending {
            if packet.data.starts_with(b"OpusTags") {
                pending = None;
            }
        }

        let mut decoder = opus::Decoder::new(48_000, channels)
            .map_err(|e| DecodeError::Codec(e.to_string()))?;
        // 120 ms at 48 kHz — the maximum Opus frame duration.
        const MAX_FRAME_SAMPLES: usize = 5760;
        let mut frame = vec![0i16; MAX_FRAME_SAMPLES * channel_count as usize];
        let mut samples: Vec<i16> = Vec::new();

        loop {
            let packet = match pending.take() {
                Some(packet) => packet,
                None => match reader
                    .read_packet()
                    .map_err(|e| DecodeError::Codec(e.to_string()))?
                {
                    Some(packet) => packet,
                    None => break,
                },
            };
            let decoded_per_channel = decoder
                .decode(&packet.data, &mut frame, false)
                .map_err(|e| DecodeError::Codec(e.to_string()))?;
            samples.extend_from_slice(&frame[..decoded_per_channel * channel_count as usize]);
        }

        let skip = (pre_skip * channel_count as usize).min(samples.len());
        samples.drain(..skip);
        if samples.is_empty() {
            return Err(DecodeError::Malformed("no decodable Opus packets"));
        }
        Ok(AudioData {
            sample_rate: 48_000,
            channels: channel_count as u16,
            samples,
        })
    }
}

/// The sniffing front door: [`AudioFormat::detect`] on the bytes, then
/// dispatch to the matching decoder — WAV included. This is still a pure
/// bytes-to-data function object, not a manager: no caching, no
/// registry, no lifetime policy.
#[derive(Debug, Default)]
pub struct AnyAudioDecoder;

impl AnyAudioDecoder {
    /// The detected format for `bytes`, if any — exposed so callers can
    /// report what they're about to decode.
    pub fn sniff(&self, bytes: &[u8]) -> Option<AudioFormat> {
        AudioFormat::detect(bytes)
    }
}

impl Decoder<AudioData> for AnyAudioDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioData, DecodeError> {
        match AudioFormat::detect(bytes) {
            Some(AudioFormat::Wav) => WavDecoder.decode(bytes),
            Some(AudioFormat::Mp3) => Mp3Decoder.decode(bytes),
            Some(AudioFormat::OggVorbis) => VorbisDecoder.decode(bytes),
            Some(AudioFormat::OggOpus) => OpusDecoder.decode(bytes),
            Some(AudioFormat::Flac) => FlacDecoder.decode(bytes),
            None => Err(DecodeError::Unsupported(
                "unrecognized audio signature (not WAV/MP3/OGG-Vorbis/OGG-Opus/FLAC)",
            )),
        }
    }
}
