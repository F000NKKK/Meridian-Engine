//! [`MaterialDecoder`]: `.mmat` bytes to a [`MaterialAsset`].

use super::{MaterialAsset, MaterialError};
use crate::Decoder;

/// Reads a `.mmat` file (text, or binary when it starts with `SDAT`).
#[derive(Debug, Clone, Copy, Default)]
pub struct MaterialDecoder;

impl Decoder<MaterialAsset> for MaterialDecoder {
    type Error = MaterialError;

    fn decode(&self, bytes: &[u8]) -> Result<MaterialAsset, MaterialError> {
        MaterialAsset::read(bytes)
    }
}
