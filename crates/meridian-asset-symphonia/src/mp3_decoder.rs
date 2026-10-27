//! MPEG Layer III decoding.

use meridian_asset_core::{AudioClip, AudioFormat, DecodeError, Decoder};

use crate::decode_compressed::{decode_compressed, expect_format};

/// Decodes MPEG Layer III (via `symphonia`, pure Rust).
#[derive(Debug, Default)]
pub struct Mp3Decoder;

impl Decoder<AudioClip> for Mp3Decoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioClip, DecodeError> {
        expect_format(bytes, AudioFormat::Mp3)?;
        decode_compressed(bytes)
    }
}
