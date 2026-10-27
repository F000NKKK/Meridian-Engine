//! The sniffing front door over every audio format.

use meridian_asset_core::{AudioClip, AudioFormat, DecodeError, Decoder, WavDecoder};

use crate::{FlacDecoder, Mp3Decoder, OpusDecoder, VorbisDecoder};

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

impl Decoder<AudioClip> for AnyAudioDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioClip, DecodeError> {
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
