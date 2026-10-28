//! [`RawBytes`]: the decoder that decodes nothing.

use std::convert::Infallible;

use meridian_asset_domain::Decoder;

/// Hands the bytes a read produced back unchanged, so the asset loader can
/// run a plain file read on its background threads (ADR 027); the data
/// layer does the decoding itself, on the caller's thread, into a `World`.
#[derive(Debug, Clone, Copy)]
pub(super) struct RawBytes;

impl Decoder<Vec<u8>> for RawBytes {
    type Error = Infallible;

    fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, Self::Error> {
        Ok(bytes.to_vec())
    }
}
