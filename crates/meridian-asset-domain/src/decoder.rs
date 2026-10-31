//! The decoder port: bytes in, a typed asset out.

/// Decodes raw file bytes into a CPU-side representation of `T`. Does not
/// decide where `T` lives afterward or when it's dropped.
pub trait Decoder<T> {
    /// Error returned when the input cannot be decoded.
    type Error;

    /// Decodes one complete asset from `bytes`.
    fn decode(&self, bytes: &[u8]) -> Result<T, Self::Error>;
}
