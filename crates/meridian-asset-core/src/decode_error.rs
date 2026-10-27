//! Why bytes failed to decode.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    TooShort {
        needed: usize,
        got: usize,
    },
    BadMagic {
        expected: &'static str,
    },
    Unsupported(&'static str),
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

impl meridian_foundation::EngineError for DecodeError {}
