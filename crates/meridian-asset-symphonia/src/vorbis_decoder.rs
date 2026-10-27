//! OGG/Vorbis decoding.

use meridian_asset_core::{AudioClip, AudioFormat, DecodeError, Decoder};

use crate::decode_compressed::{decode_compressed, expect_format};

/// Decodes OGG/Vorbis (via `symphonia`, pure Rust).
#[derive(Debug, Default)]
pub struct VorbisDecoder;

impl Decoder<AudioClip> for VorbisDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<AudioClip, DecodeError> {
        expect_format(bytes, AudioFormat::OggVorbis)?;
        decode_compressed(bytes)
    }
}
