//! [`Parent`]: the entity another entity hangs under.

use crate::{Component, Entity};

/// The entity this one hangs under. The link is plain data: despawning the
/// parent leaves it dangling, and [`propagate_transforms`](crate::propagate_transforms)
/// then treats the child as a root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Parent(pub Entity);

impl Component for Parent {}
