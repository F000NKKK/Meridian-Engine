//! The scale component: [`Scale3`] on an entity.

use meridian_gac::Scale3;

use crate::Component;

/// An entity's own scale along its local axes, applied before its
/// `Transform`. An entity without it has the unit scale. It is local: a
/// child does not inherit its parent's scale, because
/// `propagate_transforms` combines motors only.
impl Component for Scale3 {}
