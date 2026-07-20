//! Compressed-audio decoding: MP3, OGG/Vorbis and FLAC via `symphonia`
//! (pure Rust), Opus via the reference `libopus` plugged into the same
//! symphonia pipeline (`symphonia-adapter-libopus`) — see
//! docs/adr/013-compressed-audio-codecs.md for the dependency decision.
//!
//! Formats are identified by leading magic bytes ([`AudioFormat::detect`]),
//! never by file extension — the file name is not part of any decoder's
//! input. [`AnyAudioDecoder`] is the sniffing front door: detect, then
//! dispatch to the per-format decoder (including the hand-rolled
//! [`WavDecoder`] for uncompressed PCM).
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
pub(crate) fn codec_registry() -> &'static CodecRegistry {
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
    /// [`WavDecoder`].
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
fn decode(bytes: &[u8]) -> Result<AudioData, DecodeError> {
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
        if packet.track_id != track_id {
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
        decode(bytes)
    }
}

/// Decodes OGG/Vorbis (via `symphonia`, pure Rust).
#[derive(Debug, Default)]
pub struct VorbisDecoder;

impl Decoder<AudioData> for VorbisDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioData, DecodeError> {
        expect_format(bytes, AudioFormat::OggVorbis)?;
        decode(bytes)
    }
}

/// Decodes native FLAC (via `symphonia`, pure Rust).
#[derive(Debug, Default)]
pub struct FlacDecoder;

impl Decoder<AudioData> for FlacDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioData, DecodeError> {
        expect_format(bytes, AudioFormat::Flac)?;
        decode(bytes)
    }
}

/// Decodes OGG/Opus — symphonia's OGG demuxer plus the reference
/// `libopus` registered in the shared codec registry (see ADR 013 — no mature
/// pure-Rust Opus decoder exists). Output is always 48 kHz, Opus's
/// native decode rate.
#[derive(Debug, Default)]
pub struct OpusDecoder;

impl Decoder<AudioData> for OpusDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioData, DecodeError> {
        expect_format(bytes, AudioFormat::OggOpus)?;
        decode(bytes)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal single-page OGG stream: real page header (magic,
    /// version, flags, granule, serial, sequence, CRC left zero — enough
    /// for signature detection, not for demuxing), one segment carrying
    /// `payload`.
    fn make_ogg_page(payload: &[u8]) -> Vec<u8> {
        assert!(payload.len() < 255);
        let mut b = Vec::new();
        b.extend_from_slice(b"OggS");
        b.extend_from_slice(&[0u8; 22]); // version..CRC
        b.push(1); // one segment
        b.push(payload.len() as u8); // segment table
        b.extend_from_slice(payload);
        b
    }

    fn make_wav_header() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&36u32.to_le_bytes());
        b.extend_from_slice(b"WAVE");
        b
    }

    /// The repo's real MP3 test asset, if present (skip otherwise — the
    /// asset lives in `examples/`, not in this crate).
    fn demo_mp3() -> Option<Vec<u8>> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/assets/audio/demo-music.mp3"
        );
        match std::fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(_) => {
                eprintln!("skipping: {path} not found");
                None
            }
        }
    }

    #[test]
    fn detect_identifies_every_signature() {
        assert_eq!(
            AudioFormat::detect(&make_wav_header()),
            Some(AudioFormat::Wav)
        );
        assert_eq!(AudioFormat::detect(b"fLaC\x00"), Some(AudioFormat::Flac));
        assert_eq!(
            AudioFormat::detect(b"ID3\x04\x00rest"),
            Some(AudioFormat::Mp3)
        );
        assert_eq!(
            AudioFormat::detect(&[0xFF, 0xFB, 0x90, 0x00]), // MPEG-1 Layer III sync
            Some(AudioFormat::Mp3)
        );
        assert_eq!(
            AudioFormat::detect(&make_ogg_page(b"OpusHead\x01\x02")),
            Some(AudioFormat::OggOpus)
        );
        assert_eq!(
            AudioFormat::detect(&make_ogg_page(b"\x01vorbis\x00")),
            Some(AudioFormat::OggVorbis)
        );
        assert_eq!(AudioFormat::detect(&make_ogg_page(b"\x7fFLAC")), None);
        assert_eq!(AudioFormat::detect(b"not audio at all"), None);
        assert_eq!(AudioFormat::detect(&[]), None);
        // Frame-sync-like byte pair with a reserved layer is not MP3.
        assert_eq!(AudioFormat::detect(&[0xFF, 0xE0]), None);
    }

    #[test]
    fn per_format_decoders_reject_wrong_signatures() {
        let wav = make_wav_header();
        assert!(matches!(
            Mp3Decoder.decode(&wav),
            Err(DecodeError::BadMagic { .. })
        ));
        assert!(matches!(
            VorbisDecoder.decode(&wav),
            Err(DecodeError::BadMagic { .. })
        ));
        assert!(matches!(
            OpusDecoder.decode(b"fLaC"),
            Err(DecodeError::BadMagic { .. })
        ));
        assert!(matches!(
            FlacDecoder.decode(b"ID3\x04"),
            Err(DecodeError::BadMagic { .. })
        ));
    }

    #[test]
    fn malformed_bytes_error_instead_of_panicking() {
        // Valid signatures followed by garbage must produce an error,
        // never a panic or an empty success.
        let mut fake_mp3 = b"ID3\x04\x00\x00\x00\x00\x00\x00".to_vec();
        fake_mp3.extend_from_slice(&[0xAB; 64]);
        assert!(Mp3Decoder.decode(&fake_mp3).is_err());

        let mut fake_opus = make_ogg_page(b"OpusHead\xFF\xFF");
        fake_opus.extend_from_slice(&[0xCD; 64]);
        assert!(OpusDecoder.decode(&fake_opus).is_err());

        assert!(FlacDecoder.decode(b"fLaC\xDE\xAD\xBE\xEF").is_err());
    }

    #[test]
    fn any_audio_decoder_rejects_unknown_signatures() {
        assert!(matches!(
            AnyAudioDecoder.decode(b"garbage bytes here"),
            Err(DecodeError::Unsupported(_))
        ));
    }

    #[test]
    fn any_audio_decoder_decodes_real_mp3_by_signature() {
        let Some(bytes) = demo_mp3() else { return };
        assert_eq!(AnyAudioDecoder.sniff(&bytes), Some(AudioFormat::Mp3));
        let audio = AnyAudioDecoder.decode(&bytes).unwrap();
        assert_eq!(audio.sample_rate, 48_000);
        assert_eq!(audio.channels, 2);
        assert!(
            !audio.samples.is_empty(),
            "decoded MP3 must produce samples"
        );
    }
}
