//! Sequential-impulse contact solver.

use super::RigidBody;
use crate::collision::Contact;
use meridian_gac_core::{BivectorLike, GaFlavor, MotorLike, ScalarLike, VectorLike};

/// Resolves contacts into corrective linear *and* angular impulses
/// (conserving momentum) plus a small positional correction so resting
/// contacts don't sink into each other frame over frame.
///
/// The angular part is a simplified, not fully coupled, solver — see
/// docs/physics-design.md for why (it's provably inert for sphere-sphere
/// contacts specifically, since the contact point always lies on the
/// line between the two centers).
#[derive(Debug, Clone, Copy)]
pub struct ConstraintSolver<F: GaFlavor> {
    /// 0 = perfectly inelastic (bodies stick, no bounce), 1 = perfectly
    /// elastic (no kinetic energy lost).
    pub restitution: F::Scalar,
    /// Coulomb friction coefficient, applied as a tangential impulse
    /// clamped to `friction * normal_impulse` (see [`resolve`](Self::resolve)) —
    /// `0` disables friction entirely (the old behavior every existing
    /// [`ConstraintSolver::new`] call site got, since this field didn't
    /// exist before). Not a true static/dynamic-friction split (that
    /// needs per-contact tangential *position* tracking to detect
    /// "still stuck" vs "sliding", which this solver's single-point,
    /// no-warm-starting design doesn't carry) — one coefficient used as
    /// a dynamic-friction cone for both regimes, the same simplification
    /// most simple/game physics engines make.
    pub friction: F::Scalar,
    /// Below this incoming closing speed (in world units/second), a
    /// contact resolves as if `restitution` were `0` regardless of the
    /// configured value — see [`resolve_velocity`](Self::resolve_velocity).
    /// Every real tick, gravity re-adds a small downward closing speed to
    /// a body already resting on a surface (`gravity * dt`, a few
    /// hundredths of a unit/second at 60 Hz); without this threshold,
    /// *any* nonzero `restitution` turns that into a small bounce-back
    /// impulse every single tick forever — a resting contact that never
    /// actually rests, just bounces at a shrinking-but-never-zero
    /// amplitude. A single point (a resting sphere) barely shows it; a
    /// box/pyramid's multi-point manifold (see
    /// [`NarrowPhase::generate_contacts`](crate::collision::NarrowPhase::generate_contacts)) re-triggers it at up to 4
    /// points per tick, which is what made it visible as continuous
    /// up/down jitter — the concrete bug this threshold exists to kill
    /// outright, not just shrink.
    pub restitution_velocity_threshold: F::Scalar,
}

impl<F: GaFlavor> Default for ConstraintSolver<F> {
    fn default() -> Self {
        Self {
            restitution: F::Scalar::ZERO,
            friction: F::Scalar::ZERO,
            restitution_velocity_threshold: F::Scalar::from_f64(0.5),
        }
    }
}

impl<F: GaFlavor> ConstraintSolver<F> {
    /// `friction` defaults to `0` (no friction) and
    /// `restitution_velocity_threshold` to `0.5` — existing call sites
    /// that only ever passed `restitution` keep essentially the same
    /// behavior for real impacts (well above the threshold) while a
    /// resting contact no longer bounces forever off residual
    /// per-tick gravity velocity. Opt into friction via
    /// [`with_friction`](Self::with_friction), or override the
    /// threshold via [`with_restitution_velocity_threshold`](Self::with_restitution_velocity_threshold).
    pub fn new(restitution: F::Scalar) -> Self {
        Self {
            restitution,
            friction: F::Scalar::ZERO,
            restitution_velocity_threshold: F::Scalar::from_f64(0.5),
        }
    }

    pub fn with_friction(mut self, friction: F::Scalar) -> Self {
        self.friction = friction;
        self
    }

    pub fn with_restitution_velocity_threshold(mut self, threshold: F::Scalar) -> Self {
        self.restitution_velocity_threshold = threshold;
        self
    }

    /// Applies an impulse-based resolution for `contact`, mutating only
    /// `bodies[contact.a]` and `bodies[contact.b]`. A no-op if both bodies
    /// are static (infinite mass on both sides — nothing to resolve).
    ///
    /// Combines [`resolve_velocity`](Self::resolve_velocity) with a
    /// single positional-correction push — fine for a single-contact
    /// pair resolved once per tick, but calling `resolve` several times
    /// per tick against the *same* resting contact (e.g. multiple
    /// relaxation passes over a multi-point box manifold) applies that
    /// positional push every time, over-correcting the body upward each
    /// tick and producing a visible up/down bounce — the concrete
    /// "cube/pyramid jitter down-up and clip through the floor" bug this
    /// split exists to let a caller avoid: run
    /// [`resolve_velocity`](Self::resolve_velocity) for every relaxation
    /// pass, then [`apply_positional_correction`](Self::apply_positional_correction)
    /// exactly once per tick (see `examples/examples/physic_figures.rs`).
    pub fn resolve(&self, bodies: &mut [RigidBody<F>], contact: &Contact<F>) {
        self.resolve_velocity(bodies, contact);
        self.apply_positional_correction(bodies, contact);
    }

    /// The velocity-only half of [`resolve`](Self::resolve): normal
    /// impulse, angular response (unless
    /// [`Contact::suppress_angular_response`](crate::collision::Contact::suppress_angular_response)) and friction. Safe to call
    /// repeatedly per tick (e.g. once per relaxation pass over a
    /// multi-point manifold) without the positional over-correction
    /// [`resolve`](Self::resolve) would apply each time.
    pub fn resolve_velocity(&self, bodies: &mut [RigidBody<F>], contact: &Contact<F>) {
        let inv_mass_a = bodies[contact.a].inverse_mass();
        let inv_mass_b = bodies[contact.b].inverse_mass();
        let total_inv_mass = inv_mass_a + inv_mass_b;
        if total_inv_mass <= F::Scalar::ZERO {
            return;
        }

        let relative_velocity = bodies[contact.b].velocity - bodies[contact.a].velocity;
        let velocity_along_normal = relative_velocity.dot(contact.normal);

        if velocity_along_normal < F::Scalar::ZERO {
            // Below the threshold, treat this as a resting contact (no
            // bounce) regardless of `restitution` — see
            // `restitution_velocity_threshold`'s doc comment for why.
            let effective_restitution =
                if -velocity_along_normal > self.restitution_velocity_threshold {
                    self.restitution
                } else {
                    F::Scalar::ZERO
                };
            let j =
                -(F::Scalar::ONE + effective_restitution) * velocity_along_normal / total_inv_mass;
            Self::apply_impulse(bodies, contact, contact.normal * j);
            self.apply_friction(bodies, contact, j, total_inv_mass);
        }
    }

    /// Applies `impulse` to the pair — opposite linear velocity changes
    /// and, unless [`Contact::suppress_angular_response`](crate::collision::Contact::suppress_angular_response), the angular
    /// response (see the type's doc comment on why this is a simplified,
    /// decoupled solve, and that field's doc comment for why a box-box
    /// manifold point skips it entirely).
    fn apply_impulse(bodies: &mut [RigidBody<F>], contact: &Contact<F>, impulse: F::Vector) {
        let inv_mass_a = bodies[contact.a].inverse_mass();
        let inv_mass_b = bodies[contact.b].inverse_mass();
        bodies[contact.a].velocity = bodies[contact.a].velocity - impulse * inv_mass_a;
        bodies[contact.b].velocity = bodies[contact.b].velocity + impulse * inv_mass_b;
        if contact.suppress_angular_response {
            return;
        }
        let offset_a = contact.point - bodies[contact.a].position();
        let offset_b = contact.point - bodies[contact.b].position();
        let inv_inertia_a = bodies[contact.a].inverse_inertia();
        let inv_inertia_b = bodies[contact.b].inverse_inertia();
        let torque_a = F::Bivector::wedge(offset_a, impulse);
        let torque_b = F::Bivector::wedge(offset_b, impulse);
        bodies[contact.a].angular_velocity =
            bodies[contact.a].angular_velocity - torque_a * inv_inertia_a;
        bodies[contact.b].angular_velocity =
            bodies[contact.b].angular_velocity + torque_b * inv_inertia_b;
    }

    /// Coulomb friction: the tangential component of relative
    /// velocity (whatever's left of `relative_velocity` once the
    /// along-normal part is removed) gets its own impulse,
    /// opposing sliding, clamped to `friction * j` — a cone
    /// around the normal impulse, not a fixed force, so light
    /// contacts can't get an unphysically strong sideways
    /// impulse. Recomputed from the *post-normal-impulse*
    /// velocity so it reacts to what the normal impulse just
    /// did, same sequential-impulse spirit as running each
    /// manifold point's `resolve` call in sequence.
    fn apply_friction(
        &self,
        bodies: &mut [RigidBody<F>],
        contact: &Contact<F>,
        j: F::Scalar,
        total_inv_mass: F::Scalar,
    ) {
        let relative_velocity = bodies[contact.b].velocity - bodies[contact.a].velocity;
        let tangential_velocity =
            relative_velocity - contact.normal * relative_velocity.dot(contact.normal);
        let tangential_speed = tangential_velocity.length();
        if self.friction > F::Scalar::ZERO && tangential_speed > F::Scalar::EPSILON {
            let tangent = tangential_velocity * (F::Scalar::ONE / tangential_speed);
            let jt =
                (-tangential_speed / total_inv_mass).clamp(-self.friction * j, self.friction * j);
            Self::apply_impulse(bodies, contact, tangent * jt);
        }
    }

    /// The positional-correction half of [`resolve`](Self::resolve): a
    /// small push separating `contact.a`/`contact.b` along `normal`,
    /// proportional to `penetration` (past a small `slop` tolerance so
    /// resting contacts don't endlessly correct a negligible overlap).
    /// Call this exactly once per tick per pair — see
    /// [`resolve`](Self::resolve)'s doc comment for why calling it once
    /// per relaxation pass instead over-corrects.
    pub fn apply_positional_correction(&self, bodies: &mut [RigidBody<F>], contact: &Contact<F>) {
        let inv_mass_a = bodies[contact.a].inverse_mass();
        let inv_mass_b = bodies[contact.b].inverse_mass();
        let total_inv_mass = inv_mass_a + inv_mass_b;
        if total_inv_mass <= F::Scalar::ZERO {
            return;
        }

        let correction_percent = F::Scalar::from_f64(0.8);
        let slop = F::Scalar::from_f64(0.01);
        let correction_mag =
            (contact.penetration - slop).max(F::Scalar::ZERO) / total_inv_mass * correction_percent;
        let correction = contact.normal * correction_mag;
        bodies[contact.a].frame = bodies[contact.a]
            .frame
            .compose(F::Motor::translation(correction * -inv_mass_a));
        bodies[contact.b].frame = bodies[contact.b]
            .frame
            .compose(F::Motor::translation(correction * inv_mass_b));
    }
}
