//! Spherical soft bodies built from an icosphere mesh.

use meridian_gac::mesh::icosphere;
use meridian_gac::{GaFlavor, ScalarLike, VectorLike};

use super::mass_spring::SoftBody;
use super::spring::Spring;

/// How stiff a family of springs is and how fast it loses energy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringTuning<F: GaFlavor> {
    /// Hooke's-law spring constant.
    pub stiffness: F::Scalar,
    /// Velocity-relative damping coefficient.
    pub damping: F::Scalar,
}

/// The physical parameters of a soft ball: the mass of every surface
/// particle and the tuning of its edge and spoke springs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoftBodyMaterial<F: GaFlavor> {
    /// Mass of each surface particle.
    pub particle_mass: F::Scalar,
    /// Springs along the surface mesh's edges.
    pub edge: SpringTuning<F>,
    /// Springs from the center particle to each surface particle.
    pub spoke: SpringTuning<F>,
}

/// Builds a spherical [`SoftBody`] from `gac-core`'s [`icosphere`] mesh
/// (`subdivisions`: `0` = the base 12-vertex icosahedron, each further
/// level roughly quadruples the triangle count), centered at `center`
/// with `radius`, plus one center particle spoked to every surface
/// vertex. The [`SoftBodyMaterial`] gives every surface particle's mass and the
/// edge springs' tuning uniformly, and the spokes' tuning (typically softer
/// than the edges, so the ball can compress before its surface visibly
/// deforms). The center particle's own mass is the sum
/// of the surface particles' masses (a resting-heavy "core" the surface
/// springs pull against) — pinning it instead (`inverse_mass = 0`) is a
/// reasonable alternative a caller can build directly via [`SoftBody`]'s
/// fields if a stiffer-feeling ball is wanted.
pub fn icosphere_soft_body<F: GaFlavor>(
    center: F::Vector,
    radius: F::Scalar,
    subdivisions: u32,
    material: SoftBodyMaterial<F>,
) -> SoftBody<F> {
    let SoftBodyMaterial {
        particle_mass,
        edge,
        spoke,
    } = material;
    let mesh = icosphere::<F>(subdivisions);

    let mut positions: Vec<F::Vector> =
        mesh.vertices.iter().map(|&d| center + d * radius).collect();
    let surface_count = positions.len();
    positions.push(center);
    let center_index = surface_count;

    let velocities = vec![F::Vector::ZERO; positions.len()];
    let inverse_masses: Vec<F::Scalar> = {
        let surface_inv = F::Scalar::ONE / particle_mass;
        let mut m = vec![surface_inv; surface_count];
        let center_mass = particle_mass * F::Scalar::from_f64(surface_count as f64);
        m.push(F::Scalar::ONE / center_mass);
        m
    };

    let mut springs = Vec::new();
    for (a, b) in mesh.unique_edges() {
        let rest_length = (positions[b] - positions[a]).length();
        springs.push(Spring {
            a,
            b,
            rest_length,
            stiffness: edge.stiffness,
            damping: edge.damping,
        });
    }
    for surface_index in 0..surface_count {
        let rest_length = (positions[surface_index] - positions[center_index]).length();
        springs.push(Spring {
            a: surface_index,
            b: center_index,
            rest_length,
            stiffness: spoke.stiffness,
            damping: spoke.damping,
        });
    }

    SoftBody {
        positions,
        velocities,
        inverse_masses,
        springs,
    }
}
