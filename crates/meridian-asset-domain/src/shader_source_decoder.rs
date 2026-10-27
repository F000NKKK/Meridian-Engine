//! UTF-8 bytes to shader source.

use crate::{DecodeError, Decoder, ShaderSource};

/// Loads shader source as-is (UTF-8 text). Compiling it is
/// `rendering-wgpu`'s job.
#[derive(Debug, Default)]
pub struct ShaderSourceDecoder;

impl Decoder<ShaderSource> for ShaderSourceDecoder {
    type Error = DecodeError;

    fn decode(&self, bytes: &[u8]) -> Result<ShaderSource, DecodeError> {
        let source = core::str::from_utf8(bytes)
            .map_err(|_| DecodeError::Malformed("not valid UTF-8"))?
            .to_string();
        Ok(ShaderSource { source })
    }
}
