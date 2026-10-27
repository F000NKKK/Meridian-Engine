//! Native FLAC decoding.

use meridian_asset_core::{AudioClip, AudioFormat, DecodeError, Decoder};

use crate::decode_compressed::{decode_compressed, expect_format};

/// Decodes native FLAC (via `symphonia`, pure Rust).
#[derive(Debug, Default)]
pub struct FlacDecoder;

impl Decoder<AudioClip> for FlacDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioClip, DecodeError> {
        expect_format(bytes, AudioFormat::Flac)?;
        decode_compressed(bytes)
    }
}
