//! Per-particle spring adjacency — the data-prep step shared by
//! [`crate::float`]/[`crate::fixed`]'s GPU kernels, generic over
//! `F: GaFlavor` since it's pure topology extraction (no arithmetic of
//! its own, so no GPU-dispatch constraint forces it to be flavor-split —
//! see [`crate`]'s own doc comment).
//!
//! `SoftBodyIntegrator::step` (`physics-core::soft_body::generic_softbody`)
//! computes forces by iterating `springs` once and scattering the result
//! into both endpoints (`forces[a] += total; forces[b] -= total`) — fine
//! sequentially on the CPU, but a data race on the GPU if parallelized
//! directly (two particle-threads could both be another spring's
//! endpoint and write concurrently). [`build_adjacency`] restructures the
//! same springs into a per-particle neighbor list (standard CSR: a flat
//! `offsets`/`neighbor` pair, `neighbor[offsets[i]..offsets[i+1]]` is
//! particle `i`'s incident springs) so each GPU thread only ever *reads*
//! its neighbors' old state and *writes* its own new state — see
//! [`crate::float`]'s module doc for the proof this reformulation
//! produces the identical per-particle force `SoftBodyIntegrator::step`
//! does.

use meridian_gac_core::generic::{GaFlavor, ScalarLike};
use meridian_physics_core::soft_body::generic_softbody::SoftBody;

/// The CSR-encoded adjacency `crate::float`/`crate::fixed` upload as GPU
/// storage buffers. `rest_length`/`stiffness`/`damping` are duplicated
/// per directed half-edge (once from each endpoint's perspective) so a
/// GPU thread never needs to know which side of the original [`Spring`](
/// meridian_physics_core::soft_body::generic_softbody::Spring) it's on.
#[derive(Debug, Clone)]
pub struct Adjacency<F: GaFlavor> {
    /// Length `particle_count + 1`; particle `i`'s half-edges are
    /// `offsets[i]..offsets[i + 1]`.
    pub offsets: Vec<u32>,
    pub neighbor: Vec<u32>,
    pub rest_length: Vec<F::Scalar>,
    pub stiffness: Vec<F::Scalar>,
    pub damping: Vec<F::Scalar>,
}

pub fn build_adjacency<F: GaFlavor>(body: &SoftBody<F>) -> Adjacency<F> {
    let particle_count = body.particle_count();
    let mut degree = vec![0u32; particle_count];
    for spring in &body.springs {
        degree[spring.a] += 1;
        degree[spring.b] += 1;
    }

    let mut offsets = vec![0u32; particle_count + 1];
    for i in 0..particle_count {
        offsets[i + 1] = offsets[i] + degree[i];
    }

    let total_half_edges = offsets[particle_count] as usize;
    let mut neighbor = vec![0u32; total_half_edges];
    let mut rest_length = vec![F::Scalar::ZERO; total_half_edges];
    let mut stiffness = vec![F::Scalar::ZERO; total_half_edges];
    let mut damping = vec![F::Scalar::ZERO; total_half_edges];

    let mut cursor = offsets.clone();
    for spring in &body.springs {
        let a_slot = cursor[spring.a] as usize;
        neighbor[a_slot] = spring.b as u32;
        rest_length[a_slot] = spring.rest_length;
        stiffness[a_slot] = spring.stiffness;
        damping[a_slot] = spring.damping;
        cursor[spring.a] += 1;

        let b_slot = cursor[spring.b] as usize;
        neighbor[b_slot] = spring.a as u32;
        rest_length[b_slot] = spring.rest_length;
        stiffness[b_slot] = spring.stiffness;
        damping[b_slot] = spring.damping;
        cursor[spring.b] += 1;
    }

    Adjacency {
        offsets,
        neighbor,
        rest_length,
        stiffness,
        damping,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_gac_core::float_ga::FloatFlavor;
    use meridian_physics_core::soft_body::float_softbody::icosphere_soft_body;

    #[test]
    fn every_spring_produces_exactly_two_half_edges() {
        let body = icosphere_soft_body(
            meridian_gac_core::Vec3::ZERO,
            1.0,
            1,
            0.05,
            400.0,
            2.0,
            150.0,
            1.0,
        );
        let adjacency = build_adjacency::<FloatFlavor>(&body);
        assert_eq!(adjacency.offsets.len(), body.particle_count() + 1);
        assert_eq!(adjacency.neighbor.len(), body.springs.len() * 2);

        // Every half-edge must point back to a spring that actually
        // connects its owning particle to that neighbor.
        for particle in 0..body.particle_count() {
            let start = adjacency.offsets[particle] as usize;
            let end = adjacency.offsets[particle + 1] as usize;
            for slot in start..end {
                let neighbor = adjacency.neighbor[slot] as usize;
                let connected = body.springs.iter().any(|s| {
                    (s.a == particle && s.b == neighbor) || (s.b == particle && s.a == neighbor)
                });
                assert!(
                    connected,
                    "particle {particle} claims neighbor {neighbor} with no matching spring"
                );
            }
        }
    }
}
