//! Collision geometry attached independently of body motion and rendering.

use meridian_ecs::Component;
use meridian_physics_domain::ColliderShape;

/// Local collision geometry. Without `PhysicsBody` it is a static obstacle.
/// Dimensions are world units; the ECS physics bridge rejects non-unit scale.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Collider(pub ColliderShape);

impl Component for Collider {}
