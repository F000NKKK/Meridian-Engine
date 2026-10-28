//! An image embedded in a model, not yet decoded.

/// The bytes of an image file carried inside a model (PNG, JPEG, ...).
/// The format is identified by magic bytes ([`ImageFormat::detect`](crate::ImageFormat::detect)),
/// and decoding is left to whichever image decoders the application wires,
/// so a model decoder needs no image codec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedImage {
    /// The file's bytes, as stored.
    pub bytes: Vec<u8>,
}
