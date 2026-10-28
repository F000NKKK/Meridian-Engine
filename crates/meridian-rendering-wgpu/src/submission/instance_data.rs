//! One drawn instance, packed for the instance buffer.

use meridian_gac::Motor3;
use meridian_rendering_domain::scene::Material;

/// Appends the instance record [`instance_vertex_layout`](super::vertex_layout)
/// describes: the model matrix (column-major), the material's colour,
/// emissive, roughness and metallic.
pub(crate) fn push_instance(bytes: &mut Vec<u8>, frame: &Motor3, material: &Material) {
    let matrix = frame.to_mat4();
    let floats = matrix
        .iter()
        .flatten()
        .chain(&material.base_color_factor)
        .chain(&material.emissive)
        .chain([&material.roughness, &material.metallic]);
    bytes.extend(floats.flat_map(|value| value.to_le_bytes()));
}
