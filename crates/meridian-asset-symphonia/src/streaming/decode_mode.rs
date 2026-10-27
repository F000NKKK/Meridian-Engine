//! Whether audio is decoded up front or streamed.

/// Which decode path [`open_audio`](super::open_audio) takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DecodeMode {
    /// Decide from the estimated decoded size vs.
    /// [`DecodeStrategy::auto_threshold_bytes`](super::DecodeStrategy::auto_threshold_bytes); unknown-length tracks
    /// stream.
    #[default]
    Auto,
    /// Always decode the whole track into one [`AudioClip`](meridian_asset_domain::AudioClip).
    ForceEager,
    /// Always stream, no matter how small the track.
    ForceStreaming,
}
