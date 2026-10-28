//! Why a thumbnail could not be drawn.

/// A thumbnail that could not be made.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ThumbnailError {
    /// The model file could not be read or decoded.
    #[error("cannot load the model: {0}")]
    Load(String),
    /// The material file could not be read.
    #[error("cannot read the material: {0}")]
    Material(#[from] meridian_asset_domain::MaterialError),
    /// The subject draws nothing (no mesh to frame).
    #[error("nothing to draw")]
    Empty,
    /// The frame could not be drawn or read back.
    #[error("the frame could not be drawn")]
    Render,
}
