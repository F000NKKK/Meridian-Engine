//! OGG/Opus decoding.

use meridian_asset_domain::{AudioClip, AudioFormat, DecodeError, Decoder};

use crate::decode_compressed::{decode_compressed, expect_format};

/// Decodes OGG/Opus — symphonia's OGG demuxer plus the reference
/// `libopus` registered in the shared codec registry (see ADR 013 — no mature
/// pure-Rust Opus decoder exists). Output is always 48 kHz, Opus's
/// native decode rate.
#[derive(Debug, Default)]
pub struct OpusDecoder;

impl Decoder<AudioClip> for OpusDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioClip, DecodeError> {
        expect_format(bytes, AudioFormat::OggOpus)?;
        decode_compressed(bytes)
    }
}
