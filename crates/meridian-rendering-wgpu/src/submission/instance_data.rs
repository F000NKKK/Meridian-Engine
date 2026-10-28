//! One drawn instance, packed for the instance buffer.

use meridian_gac::{Motor3, Scale3};
use meridian_rendering_domain::scene::Material;

/// Bytes of one instance record: thirty `f32`s (see [`push_instance`]).
pub(crate) const INSTANCE_STRIDE: usize = 120;

/// Appends the instance record [`instance_vertex_layout`](super::vertex_layout)
/// describes: the model matrix (column-major), the material's colour,
/// emissive, roughness, metallic, normal scale, occlusion strength and the
/// emissive factor that scales an emissive map.
///
/// A material with an emissive map puts its factor in the last slot and a
/// zero in the plain emissive one. Bloom's plain pass reads the plain slot;
/// its mapped pass reads the last slot and samples the map (ADR 042).
pub(crate) fn push_instance(
    bytes: &mut Vec<u8>,
    frame: &Motor3,
    scale: Scale3,
    material: &Material,
) {
    let matrix = scaled_matrix(frame.to_mat4(), scale);
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

/// `matrix * diag(scale)`: each of the three axis columns times its factor,
/// so the scale applies before the frame.
fn scaled_matrix(mut matrix: [[f32; 4]; 4], scale: Scale3) -> [[f32; 4]; 4] {
    for (column, factor) in matrix.iter_mut().zip(scale.components()) {
        for value in &mut column[..3] {
            *value *= factor;
        }
    }
    matrix
}

#[cfg(test)]
mod tests {
    use meridian_gac::Vec3;

    use super::*;

    #[test]
    fn the_scale_stretches_the_axis_columns_and_not_the_translation() {
        let frame = Motor3::translation(Vec3::new(1.0, 2.0, 3.0));
        let scale = Scale3::new(2.0, 3.0, 4.0).unwrap();
        let matrix = scaled_matrix(frame.to_mat4(), scale);
        assert_eq!([matrix[0][0], matrix[1][1], matrix[2][2]], [2.0, 3.0, 4.0]);
        assert_eq!(matrix[3][..3], [1.0, 2.0, 3.0]);
    }
}
