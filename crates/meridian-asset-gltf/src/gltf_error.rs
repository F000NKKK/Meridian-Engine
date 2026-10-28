//! Why a glTF file could not be decoded.

/// A typed reason a glTF file was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GltfError {
    /// The container or JSON is invalid (the `gltf` crate's message).
    Malformed(String),
    /// A buffer or image names an external file; resolving paths is the
    /// loader's job, not the decoder's.
    ExternalResource(String),
    /// A `data:` URI is not base64 or is cut short.
    BadDataUri,
    /// A buffer view or accessor reaches outside its buffer.
    OutOfBounds(&'static str),
    /// A primitive that is not a triangle list; carries the mode's name.
    UnsupportedPrimitive(String),
    /// An index points past the primitive's vertices, or the index count is
    /// not a multiple of three.
    BadIndices,
    /// The model expands beyond the total vertex bound.
    ModelTooLarge,
    /// Nodes nest deeper than the decoder follows.
    TooDeep,
}

impl core::fmt::Display for GltfError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GltfError::Malformed(what) => write!(f, "malformed glTF: {what}"),
            GltfError::ExternalResource(uri) => {
                write!(f, "external resource `{uri}` is not loaded by the decoder")
            }
            GltfError::BadDataUri => write!(f, "invalid base64 data URI"),
            GltfError::OutOfBounds(what) => write!(f, "{what} reaches outside its buffer"),
            GltfError::UnsupportedPrimitive(mode) => {
                write!(f, "unsupported primitive mode {mode}: only triangle lists")
            }
            GltfError::BadIndices => write!(f, "indices out of range or not whole triangles"),
            GltfError::ModelTooLarge => write!(f, "model expands beyond the vertex bound"),
            GltfError::TooDeep => write!(f, "node hierarchy nests too deeply"),
        }
    }
}

impl std::error::Error for GltfError {}
