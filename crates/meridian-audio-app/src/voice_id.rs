//! The handle of a voice in a scene.

/// Identifies a voice in an [`AudioScene`](crate::AudioScene); never reused within a scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceId(u64);

impl VoiceId {
    pub(crate) fn new(raw: u64) -> Self {
        Self(raw)
    }
}
