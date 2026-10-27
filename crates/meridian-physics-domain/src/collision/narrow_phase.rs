//! Narrow phase: exact contacts for candidate pairs.

use meridian_gac::{GaFlavor, ScalarLike};

use super::{ColliderShape, Contact, cuboid_vs_cuboid, sphere_vs_cuboid, sphere_vs_sphere};
use crate::dynamics::RigidBody;

/// Resolves candidate pairs from [`BroadPhase`](crate::collision::BroadPhase) into exact [`Contact`](crate::collision::Contact)s.
#[derive(Debug)]
pub struct NarrowPhase<F: GaFlavor>(core::marker::PhantomData<F>);

impl<F: GaFlavor> Default for NarrowPhase<F> {
    fn default() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<F: GaFlavor> NarrowPhase<F> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Exact test for the current pair of [`ColliderShape`](crate::collision::ColliderShape)s. `None` if
    /// not overlapping.
    pub fn test_pair(&self, bodies: &[RigidBody<F>], a: usize, b: usize) -> Option<Contact<F>> {
        let (point, penetration, normal) = match (bodies[a].shape, bodies[b].shape) {
            (ColliderShape::Sphere { radius: ra }, ColliderShape::Sphere { radius: rb }) => {
                sphere_vs_sphere::<F>(bodies[a].position(), ra, bodies[b].position(), rb)?
            }
            (ColliderShape::Sphere { radius }, ColliderShape::Cuboid { half_extents }) => {
                let obb = bodies[b].as_obb(half_extents);
                let (point, penetration, normal) =
                    sphere_vs_cuboid(bodies[a].position(), radius, &obb)?;
                // sphere_vs_cuboid's normal points cuboid(b) -> sphere(a);
                // Contact.normal must point a -> b, so flip it.
                (point, penetration, -normal)
            }
            (ColliderShape::Cuboid { half_extents }, ColliderShape::Sphere { radius }) => {
                let obb = bodies[a].as_obb(half_extents);
                // Already cuboid(a) -> sphere(b) = a -> b: no flip needed.
                sphere_vs_cuboid(bodies[b].position(), radius, &obb)?
            }
            (
                ColliderShape::Cuboid {
                    half_extents: half_extents_a,
                },
                ColliderShape::Cuboid {
                    half_extents: half_extents_b,
                },
            ) => {
                let obb_a = bodies[a].as_obb(half_extents_a);
                let obb_b = bodies[b].as_obb(half_extents_b);
                // `test_pair` reports the pair's total, undivided
                // penetration and one representative manifold point —
                // see `generate_contacts` for the multi-point expansion
                // this single-`Contact` view intentionally doesn't do.
                let (points, penetration, normal) = cuboid_vs_cuboid(&obb_a, &obb_b)?;
                (points[0], penetration, normal)
            }
        };
        Some(Contact {
            a,
            b,
            normal,
            penetration,
            point,
            suppress_angular_response: false,
        })
    }

    /// Like [`test_pair`](Self::test_pair), but expands a box-box pair's
    /// full contact manifold (see `face_manifold`) into one [`Contact`](crate::collision::Contact)
    /// per manifold point instead of collapsing it to one — this is what
    /// [`ConstraintSolver::resolve`](crate::dynamics::ConstraintSolver::resolve) should iterate over; `test_pair`'s
    /// single-point view exists only for simple exact-overlap queries.
    pub fn generate_contacts(
        &self,
        bodies: &[RigidBody<F>],
        candidate_pairs: &[(usize, usize)],
    ) -> Vec<Contact<F>> {
        let mut contacts = Vec::with_capacity(candidate_pairs.len());
        for &(a, b) in candidate_pairs {
            match (bodies[a].shape, bodies[b].shape) {
                (
                    ColliderShape::Cuboid { half_extents: he_a },
                    ColliderShape::Cuboid { half_extents: he_b },
                ) => {
                    let obb_a = bodies[a].as_obb(he_a);
                    let obb_b = bodies[b].as_obb(he_b);
                    let Some((points, penetration, normal)) = cuboid_vs_cuboid(&obb_a, &obb_b)
                    else {
                        continue;
                    };
                    let point_count = F::Scalar::from_f64(points.len() as f64);
                    for point in points {
                        contacts.push(Contact {
                            a,
                            b,
                            normal,
                            penetration: penetration / point_count,
                            point,
                            suppress_angular_response: true,
                        });
                    }
                }
                _ => {
                    if let Some(contact) = self.test_pair(bodies, a, b) {
                        contacts.push(contact);
                    }
                }
            }
        }
        contacts
    }
}
