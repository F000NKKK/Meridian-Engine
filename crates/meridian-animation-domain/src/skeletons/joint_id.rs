//! The identity of a joint within its skeleton.

/// A joint of a [`Skeleton`](super::Skeleton), named by its position in
/// [`Skeleton::joints`](super::Skeleton::joints). Not interchangeable between
/// skeletons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JointId(u32);

impl JointId {
    /// The joint at `index`, `None` when `index` does not fit in a `u32`.
    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        u32::try_from(index).ok().map(Self)
    }

    /// The joint's position in its skeleton.
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}
