//! Why bytes failed to decode.

/// Why an asset's bytes could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The input ended before a required structure was complete.
    TooShort {
        /// Minimum byte count required by the decoder.
        needed: usize,
        /// Actual input byte count.
        got: usize,
    },
    /// The file signature does not match the expected format.
    BadMagic {
        /// Expected file signature, shown in diagnostics.
        expected: &'static str,
    },
    /// The input uses a feature this decoder does not implement.
    Unsupported(&'static str),
    /// The input violates the format's structural rules.
    Malformed(&'static str),
    /// An external codec library rejected the data — the message is the
    /// library's own (dynamic) error text.
    Codec(String),
}

impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::TooShort { needed, got } => {
                write!(f, "buffer too short: needed {needed} bytes, got {got}")
            }
            DecodeError::BadMagic { expected } => write!(f, "bad magic bytes, expected {expected}"),
            DecodeError::Unsupported(what) => write!(f, "unsupported: {what}"),
            DecodeError::Malformed(what) => write!(f, "malformed: {what}"),
            DecodeError::Codec(what) => write!(f, "codec error: {what}"),
        }
    }
}

impl std::error::Error for DecodeError {}

impl meridian_diagnostics::EngineError for DecodeError {}
