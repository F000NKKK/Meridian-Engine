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
//! its neighbors' old state and *writes* its own new state.
//!
//! Each half-edge also encodes, in `neighbor`'s top bit, whether *this*
//! particle is the original [`Spring`](
//! meridian_physics_core::soft_body::generic_softbody::Spring)'s `a` or
//! `b` endpoint (see [`decode_neighbor`]) — not a topology detail, a
//! numerical-correctness one. `Fixed::mul`'s `>>16` truncation rounds
//! toward negative infinity, so `fixed_mul(-p, q)` is *not* always
//! exactly `-fixed_mul(p, q)` (they differ by one raw bit whenever the
//! discarded low bits are nonzero, which is the common case) — meaning
//! recomputing a spring's direction independently from each endpoint's
//! own (sign-flipped) `delta` and multiplying is **not** bit-exact with
//! `SoftBodyIntegrator::step`'s "compute `total` once from `a`'s
//! direction, then `forces[b] -= total`" (this was found the hard way: an
//! earlier version of this module recomputed direction/`total` per
//! endpoint and passed every test except a 60-step bit-exact
//! reproducibility check, which diverged by one raw bit at step 48 —
//! `Vector::Sub`/`Neg`, unlike `Fixed::mul`, *are* exact under negation,
//! since they're plain two's-complement add/negate with no shift/
//! truncation involved). The fix: both endpoints compute the *same*
//! canonical `a`-to-`b` direction and `total` (via `is_a`-conditional
//! subtraction order, still exact), and only the non-`a` endpoint
//! negates the finished `total` vector — an exact negation, never a
//! multiply — reproducing `SoftBodyIntegrator::step` bit-for-bit. See
//! [`crate::fixed`]'s kernel for where this canonicalization happens.

use meridian_gac_core::generic::{GaFlavor, ScalarLike};
use meridian_physics_core::soft_body::generic_softbody::SoftBody;

/// `neighbor`'s top bit, set when this half-edge's owning particle is the
/// original spring's `a` endpoint. Particle counts in this workspace are
/// nowhere near `2^31`, so stealing the top bit of an otherwise-small
/// index costs nothing.
const IS_A_BIT: u32 = 1 << 31;

/// Encodes a half-edge's neighbor particle index and `is_a` flag into one
/// `u32` — see [`decode_neighbor`] and the module doc.
pub fn encode_neighbor(neighbor: usize, is_a: bool) -> u32 {
    debug_assert!(
        neighbor < IS_A_BIT as usize,
        "particle index too large to fit alongside the is_a flag"
    );
    (neighbor as u32) | if is_a { IS_A_BIT } else { 0 }
}

/// Decodes a half-edge's `(neighbor particle index, is_a flag)` — the
/// inverse of [`encode_neighbor`]. WGSL callers do the equivalent bit
/// operations directly (`encoded & 0x7FFFFFFFu`, `encoded >> 31u`).
pub fn decode_neighbor(encoded: u32) -> (usize, bool) {
    ((encoded & !IS_A_BIT) as usize, encoded & IS_A_BIT != 0)
}

/// The CSR-encoded adjacency `crate::float`/`crate::fixed` upload as GPU
/// storage buffers. `rest_length`/`stiffness`/`damping` are duplicated
/// per directed half-edge (once from each endpoint's perspective) so a
/// GPU thread never needs to know which side of the original [`Spring`](
/// meridian_physics_core::soft_body::generic_softbody::Spring) it's on,
/// beyond the `is_a` flag encoded in [`Self::neighbor`] (see the module
/// doc for why that flag exists).
#[derive(Debug, Clone)]
pub struct Adjacency<F: GaFlavor> {
    /// Length `particle_count + 1`; particle `i`'s half-edges are
    /// `offsets[i]..offsets[i + 1]`.
    pub offsets: Vec<u32>,
    /// Each entry is [`encode_neighbor`]'s output — decode with
    /// [`decode_neighbor`], not a raw index.
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
        neighbor[a_slot] = encode_neighbor(spring.b, true);
        rest_length[a_slot] = spring.rest_length;
        stiffness[a_slot] = spring.stiffness;
        damping[a_slot] = spring.damping;
        cursor[spring.a] += 1;

        let b_slot = cursor[spring.b] as usize;
        neighbor[b_slot] = encode_neighbor(spring.a, false);
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
                let (neighbor, is_a) = decode_neighbor(adjacency.neighbor[slot]);
                let connected = body.springs.iter().any(|s| {
                    (s.a == particle && s.b == neighbor && is_a)
                        || (s.b == particle && s.a == neighbor && !is_a)
                });
                assert!(
                    connected,
                    "particle {particle} claims neighbor {neighbor} (is_a={is_a}) with no matching spring"
                );
            }
        }
    }
}
