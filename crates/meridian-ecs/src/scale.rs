//! The scale component: [`Scale3`] on an entity.

use meridian_gac::Scale3;

use crate::Component;

/// An entity's own scale along its local axes, applied before its
/// `Transform`. An entity without it has the unit scale. The component is
/// local; `propagate_transforms` multiplies it down the hierarchy into
/// `GlobalTransform::scale`, so a child is scaled and displaced with its
/// parent.
impl Component for Scale3 {}
