//! WGSL for the normal matrix of a scaled instance, spliced into the lit shaders.

/// `axis_scale_squared`: the squared scale of an instance along each axis.
pub(super) const AXIS_SCALE_WGSL: &str = r"
// The squared length of each axis column of `model`: the squares of the
// instance's scale along its axes. A normal must move by the inverse
// transpose of the model matrix, which for a rotation times a scale is the
// model matrix applied to the normal divided by these squares; with a unit
// scale it is the identity.
fn axis_scale_squared(model: mat4x4<f32>) -> vec3<f32> {
    return vec3<f32>(
        dot(model[0].xyz, model[0].xyz),
        dot(model[1].xyz, model[1].xyz),
        dot(model[2].xyz, model[2].xyz),
    );
}
";
