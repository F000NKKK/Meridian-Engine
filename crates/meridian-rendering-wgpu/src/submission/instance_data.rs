//! One drawn instance, packed for the instance buffer.

use meridian_gac::{Motor3, Vec3};
use meridian_rendering_domain::scene::Material;

/// Bytes of one instance record: thirty `f32`s (see [`push_instance`]).
pub(crate) const INSTANCE_STRIDE: usize = 120;

/// Appends the instance record [`instance_vertex_layout`](super::vertex_layout)
/// describes: the model matrix (column-major), the material's colour,
/// emissive, roughness, metallic, normal scale, occlusion strength and the
/// emissive factor that scales an emissive map.
///
/// The matrix is `frame * scale`: each of its three axis columns is stretched
/// by the matching component of `scale`, the translation column is not.
///
/// A material with an emissive map puts its factor in the last slot and a
/// zero in the plain emissive one. Bloom's plain pass reads the plain slot;
/// its mapped pass reads the last slot and samples the map (ADR 042).
pub(crate) fn push_instance(bytes: &mut Vec<u8>, frame: &Motor3, scale: Vec3, material: &Material) {
    let mut matrix = frame.to_mat4();
    for (column, factor) in matrix.iter_mut().zip([scale.x, scale.y, scale.z]) {
        column[..3].iter_mut().for_each(|value| *value *= factor);
    }
    let (plain, mapped) = if material.emissive_map.is_some() {
        ([0.0; 3], material.emissive)
    } else {
        (material.emissive, [0.0; 3])
    };
    let floats = matrix
        .iter()
        .flatten()
        .chain(&material.base_color_factor)
        .chain(&plain)
        .chain([&material.roughness, &material.metallic])
        .chain([&material.normal_scale, &material.occlusion_strength])
        .chain(&mapped);
    bytes.extend(floats.flat_map(|value| value.to_le_bytes()));
}
