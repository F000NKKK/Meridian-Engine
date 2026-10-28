//! The handle of a bus in a scene.

/// Identifies a bus (a gain group of voices) in an [`AudioScene`](crate::AudioScene).
/// [`BusId::MASTER`] exists in every scene and scales the whole mix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BusId(usize);

impl BusId {
    /// The bus every voice is mixed through; it scales the final output.
    pub const MASTER: Self = Self(0);

    pub(crate) fn new(index: usize) -> Self {
        Self(index)
    }

    pub(crate) fn index(self) -> usize {
        self.0
    }
}
