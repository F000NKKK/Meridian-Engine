//! One joint of a model's skeleton.

/// A joint as the source file stores it: where it hangs, how it sits at rest
/// and how the mesh was bound to it. Plain numbers; the animation domain
/// gives them behaviour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelJoint {
    /// Index of the parent in [`ModelSkeleton::joints`](super::ModelSkeleton::joints),
    /// always below this joint's own index; `None` for a root.
    pub parent: Option<usize>,
    /// Rest translation relative to the parent.
    pub translation: [f32; 3],
    /// Rest rotation relative to the parent, a unit quaternion `[x, y, z, w]`.
    pub rotation: [f32; 4],
    /// Rest scale relative to the parent.
    pub scale: [f32; 3],
    /// Column-major matrix that takes a bind-pose vertex into the joint's space.
    pub inverse_bind: [[f32; 4]; 4],
}
