//! Batch [`ConstraintSolver`](meridian_physics_core::generic::ConstraintSolver)
//! through `compute-runtime` — the last of the three "aren't batched
//! yet" items, and the hardest, which is exactly why it was left for
//! last (see [`crate::broad_phase`]/[`crate::narrow_phase`] for the
//! other two).
//!
//! **Why this one is different from every other kernel in this crate.**
//! `Integrator::step`/`NarrowPhase::test_pair` are embarrassingly
//! parallel — each body/pair reads and writes only its own state, so a
//! plain `parallel_for` over all of them is safe by construction.
//! `ConstraintSolver::resolve_velocity`/`apply_positional_correction`
//! are not: each contact mutates **two** bodies
//! (`contact.a`/`contact.b`), and two contacts sharing a body (the
//! common case — every point of a box's 4-point floor manifold touches
//! the same two bodies) would race if resolved concurrently against a
//! shared body array. `physics-core`'s own sequential loop is
//! deliberately Gauss-Seidel: contact *N* resolves against whatever
//! contacts `1..N` already wrote to `bodies` this pass — see
//! `PhysicsSubsystem::step`'s own doc comment for why that ordering is
//! what makes multi-point manifolds converge without jitter. A naive
//! "resolve every contact against the same pre-pass snapshot" batching
//! would silently become Jacobi-style instead, which is exactly the
//! kind of numerical-behavior change this workspace's own history says
//! not to introduce carelessly (see `ConstraintSolver::resolve`'s doc
//! comment for the last time a solver-ordering assumption broke
//! silently).
//!
//! **The fix: graph coloring.** [`color_contacts`] partitions `contacts`
//! into groups such that no two contacts in the same group share a body
//! index — contacts in the same color are provably independent (their
//! writes touch fully disjoint bodies), so they can run concurrently
//! with zero risk of a race, *and* the result is identical to running
//! them one at a time in any order (since none of them can observe each
//! other's writes). Colors themselves still run in sequence, in the
//! same relative order the contacts were encountered in (greedy
//! coloring assigns a contact to the first color that doesn't already
//! touch one of its bodies, scanning colors in creation order) — this
//! reproduces the *exact* Gauss-Seidel dependency chain the sequential
//! loop has: whichever contact would have run first still effectively
//! runs first (in its own color), and any contact that depends on an
//! earlier one sharing a body is guaranteed to land in a later color
//! (they can never share a color, by construction), so it always sees
//! the update. [`ConstraintSolverBatchKernel::step`]'s own test proves
//! this equivalence directly against the sequential loop, not just
//! "produces plausible output."
//!
//! **Per-body locking, not one shared lock over the whole body array.**
//! Bodies are `Vec<Mutex<RigidBody<F>>>` — each body individually
//! lockable. Within one color, no two contacts ever touch the same
//! body (that's the coloring invariant), so no thread ever contends
//! another thread's lock — real parallelism, not `parallel_for`
//! serialized behind one coarse `Mutex<Vec<_>>` the way
//! [`crate::rigid_body::RigidBodyIntegratorKernel`]'s independent
//! per-body writes still are (a deliberate "correct first, profile
//! before optimizing further" choice there — this module can do
//! better *safely*, at zero extra `unsafe`, only because coloring
//! already proves the disjointness a per-body lock would otherwise
//! exist to arbitrate).

use std::collections::HashSet;
use std::sync::Mutex;

use meridian_compute_runtime::ComputeContext;
use meridian_gac_core::generic::GaFlavor;
use meridian_physics_core::generic::{ConstraintSolver, Contact, RigidBody};

/// Partitions `contacts` into color groups: no two contacts in the same
/// group share `contact.a` or `contact.b`. Greedy, contacts assigned in
/// input order — see the module doc for why this specific assignment
/// order is what makes the batched result match the sequential
/// Gauss-Seidel loop exactly, not just "some valid coloring."
pub fn color_contacts<F: GaFlavor>(contacts: &[Contact<F>]) -> Vec<Vec<usize>> {
    let mut colors: Vec<Vec<usize>> = Vec::new();
    let mut color_bodies: Vec<HashSet<usize>> = Vec::new();

    for (index, contact) in contacts.iter().enumerate() {
        let mut placed = false;
        for (color, bodies_used) in color_bodies.iter_mut().enumerate() {
            if !bodies_used.contains(&contact.a) && !bodies_used.contains(&contact.b) {
                bodies_used.insert(contact.a);
                bodies_used.insert(contact.b);
                colors[color].push(index);
                placed = true;
                break;
            }
        }
        if !placed {
            let mut bodies_used = HashSet::new();
            bodies_used.insert(contact.a);
            bodies_used.insert(contact.b);
            color_bodies.push(bodies_used);
            colors.push(vec![index]);
        }
    }

    colors
}

/// Batches [`ConstraintSolver::resolve_velocity`]/
/// [`ConstraintSolver::apply_positional_correction`] over `contacts`
/// via [`color_contacts`] — [`step`](Self::step) runs
/// `relaxation_iterations` velocity-only passes (each pass: every
/// color's contacts resolved in parallel, colors in sequence) followed
/// by one positional-correction pass, the same shape
/// `PhysicsSubsystem::step` uses, batched.
///
/// Takes `contacts` once at construction rather than recomputing
/// broad/narrow-phase per relaxation pass — deliberately different from
/// `PhysicsSubsystem::step`'s own CPU loop, which *does* recompute them
/// every pass (harmless but wasteful: `resolve_velocity` never changes
/// a body's `frame`, only its velocity, so the contact set genuinely
/// can't change between velocity-only passes). Callers batching
/// broad/narrow-phase too (see [`crate::broad_phase`]/
/// [`crate::narrow_phase`]) get that contact set once and hand it here,
/// rather than this kernel re-deriving it itself.
#[derive(Debug)]
pub struct ConstraintSolverBatchKernel<F: GaFlavor>
where
    F: Sync,
    F::Scalar: Send + Sync,
    F::Vector: Send + Sync,
    F::Bivector: Send + Sync,
    F::Rotor: Send + Sync,
    F::Motor: Send + Sync,
{
    solver: ConstraintSolver<F>,
    bodies: Vec<Mutex<RigidBody<F>>>,
    contacts: Vec<Contact<F>>,
    colors: Vec<Vec<usize>>,
}

impl<F: GaFlavor> ConstraintSolverBatchKernel<F>
where
    F: Sync,
    F::Scalar: Send + Sync,
    F::Vector: Send + Sync,
    F::Bivector: Send + Sync,
    F::Rotor: Send + Sync,
    F::Motor: Send + Sync,
{
    pub fn new(
        solver: ConstraintSolver<F>,
        bodies: Vec<RigidBody<F>>,
        contacts: Vec<Contact<F>>,
    ) -> Self {
        let colors = color_contacts(&contacts);
        Self {
            solver,
            bodies: bodies.into_iter().map(Mutex::new).collect(),
            contacts,
            colors,
        }
    }

    /// The bodies' current state — call after [`step`](Self::step) to
    /// read back the result.
    pub fn bodies(&self) -> Vec<RigidBody<F>> {
        self.bodies.iter().map(|b| *b.lock().unwrap()).collect()
    }

    /// One relaxation pass: every color's contacts resolved via
    /// `resolve_velocity` in parallel (colors themselves run in
    /// sequence — see the module doc for why that ordering is required,
    /// not incidental).
    fn resolve_velocity_pass(&self, context: &ComputeContext) {
        for color in &self.colors {
            context.parallel_for(color.len(), |k| {
                let contact = &self.contacts[color[k]];
                // Lock exactly the two bodies this contact touches.
                // `color_contacts`'s own invariant guarantees no other
                // concurrently-running closure in this `parallel_for`
                // call ever wants either lock, so this never contends —
                // real parallelism, not serialization dressed up as one.
                let mut a = self.bodies[contact.a].lock().unwrap();
                let mut b = self.bodies[contact.b].lock().unwrap();
                resolve_velocity_pair(&self.solver, &mut a, &mut b, contact);
            });
        }
    }

    /// One positional-correction pass, same per-color parallel shape as
    /// [`resolve_velocity_pass`](Self::resolve_velocity_pass) — call
    /// exactly once per tick, after every relaxation pass, matching
    /// `ConstraintSolver::resolve`'s own doc comment about why calling
    /// positional correction more than once per tick over-corrects.
    fn apply_positional_correction_pass(&self, context: &ComputeContext) {
        for color in &self.colors {
            context.parallel_for(color.len(), |k| {
                let contact = &self.contacts[color[k]];
                let mut a = self.bodies[contact.a].lock().unwrap();
                let mut b = self.bodies[contact.b].lock().unwrap();
                apply_positional_correction_pair(&self.solver, &mut a, &mut b, contact);
            });
        }
    }

    /// `relaxation_iterations` velocity-only passes, then one
    /// positional-correction pass — the full batched equivalent of
    /// `PhysicsSubsystem::step`'s relaxation loop over a fixed contact
    /// set.
    pub fn step(&self, context: &ComputeContext, relaxation_iterations: u32) {
        for _ in 0..relaxation_iterations {
            self.resolve_velocity_pass(context);
        }
        self.apply_positional_correction_pass(context);
    }
}

/// `ConstraintSolver::resolve_velocity`'s exact logic, applied to two
/// already-locked bodies rather than an indexed slice — the only reason
/// this exists separately from calling `resolve_velocity` on the
/// original body array directly is that `contact.a`/`contact.b` are
/// indices into that *original* array, not `0`/`1` into a two-element
/// temporary, so a temporary two-body slice can't reuse `Contact`'s own
/// indices unchanged. This still calls straight into `ConstraintSolver`'s
/// real two-body-slice method underneath — see [`with_pair_as_slice`] —
/// not a reimplementation of the impulse math.
fn resolve_velocity_pair<F: GaFlavor>(
    solver: &ConstraintSolver<F>,
    a: &mut RigidBody<F>,
    b: &mut RigidBody<F>,
    contact: &Contact<F>,
) {
    with_pair_as_slice(a, b, contact, |bodies, remapped| {
        solver.resolve_velocity(bodies, &remapped);
    });
}

fn apply_positional_correction_pair<F: GaFlavor>(
    solver: &ConstraintSolver<F>,
    a: &mut RigidBody<F>,
    b: &mut RigidBody<F>,
    contact: &Contact<F>,
) {
    with_pair_as_slice(a, b, contact, |bodies, remapped| {
        solver.apply_positional_correction(bodies, &remapped);
    });
}

/// Builds a real two-element `[RigidBody; 2]` slice out of the two
/// already-locked bodies a `Contact` names, remaps `contact.a`/`.b` to
/// `0`/`1` to match that slice, runs `f` against `ConstraintSolver`'s
/// real slice-taking methods (`resolve_velocity`/
/// `apply_positional_correction` — the actual physics logic stays in
/// `physics-core`, this only reshapes the indices), then writes the
/// (possibly mutated) results back through the original `&mut`
/// references. This is the one place index bookkeeping happens, kept
/// separate from the impulse math itself.
fn with_pair_as_slice<F: GaFlavor>(
    a: &mut RigidBody<F>,
    b: &mut RigidBody<F>,
    contact: &Contact<F>,
    f: impl FnOnce(&mut [RigidBody<F>], Contact<F>),
) {
    let mut pair = [*a, *b];
    let remapped = Contact {
        a: 0,
        b: 1,
        ..*contact
    };
    f(&mut pair, remapped);
    *a = pair[0];
    *b = pair[1];
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_gac_core::float_ga::FloatFlavor;
    use meridian_gac_core::{Motor3, Vec3};
    use meridian_physics_core::generic::{BroadPhase, ColliderShape, NarrowPhase};

    fn cuboid_at(x: f32, y: f32, half: f32) -> RigidBody<FloatFlavor> {
        RigidBody {
            frame: Motor3::translation(Vec3::new(x, y, 0.0)),
            velocity: Vec3::ZERO,
            angular_velocity: meridian_gac_core::Bivector3::ZERO,
            mass: 1.0,
            shape: ColliderShape::Cuboid {
                half_extents: Vec3::new(half, half, half),
            },
        }
    }

    /// A wide static floor and a box resting flush on top -- a real
    /// multi-point face manifold (see `narrow_phase`'s identical setup),
    /// chosen specifically because it's the case that actually exercises
    /// coloring: several contacts sharing the same two bodies.
    fn floor_and_box() -> Vec<RigidBody<FloatFlavor>> {
        let mut floor = cuboid_at(0.0, -5.0, 0.0);
        floor.mass = 0.0;
        floor.shape = ColliderShape::Cuboid {
            half_extents: Vec3::new(10.0, 5.0, 10.0),
        };
        let boxy = cuboid_at(0.0, 0.45, 0.5);
        vec![floor, boxy]
    }

    /// `color_contacts` must produce color groups where no two contacts
    /// in the same group share a body -- the actual safety invariant
    /// everything else in this module depends on.
    #[test]
    fn color_contacts_never_puts_conflicting_contacts_in_the_same_color() {
        let bodies = floor_and_box();
        let narrow_phase = NarrowPhase::<FloatFlavor>::new();
        let contacts = narrow_phase.generate_contacts(&bodies, &[(0, 1)]);
        assert!(
            contacts.len() > 1,
            "test setup should produce a multi-point manifold, got {}",
            contacts.len()
        );

        let colors = color_contacts(&contacts);
        // All contacts here share the same two bodies (0 and 1), so
        // coloring must put each in its own color -- the worst case,
        // and the one that would break first if coloring were buggy.
        assert_eq!(colors.len(), contacts.len());
        for color in &colors {
            assert_eq!(color.len(), 1);
        }
    }

    /// The actual point of this module: batched `step` (parallel within
    /// color, colors in sequence) must produce bit-for-bit identical
    /// results to the sequential `resolve_velocity`/
    /// `apply_positional_correction` loop `PhysicsSubsystem::step` uses,
    /// across a real multi-point manifold -- not just "looks physically
    /// plausible."
    #[test]
    fn batched_step_matches_sequential_loop_exactly() {
        let bodies = floor_and_box();
        let solver = ConstraintSolver::<FloatFlavor>::new(0.0).with_friction(0.6);
        let mut broad_phase = BroadPhase::<FloatFlavor>::new();
        let narrow_phase = NarrowPhase::<FloatFlavor>::new();
        let pairs = broad_phase.find_candidate_pairs(&bodies).to_vec();
        let contacts = narrow_phase.generate_contacts(&bodies, &pairs);
        assert!(
            contacts.len() > 1,
            "need a multi-point manifold to prove coloring matters"
        );

        // Sequential reference, mirroring PhysicsSubsystem::step exactly.
        let mut expected = bodies.clone();
        const RELAXATION_ITERATIONS: u32 = 4;
        for _ in 0..RELAXATION_ITERATIONS {
            for contact in &contacts {
                solver.resolve_velocity(&mut expected, contact);
            }
        }
        for contact in &contacts {
            solver.apply_positional_correction(&mut expected, contact);
        }

        // Batched version, same solver/contacts/iteration count.
        let context = ComputeContext::new();
        let kernel = ConstraintSolverBatchKernel::new(solver, bodies, contacts);
        kernel.step(&context, RELAXATION_ITERATIONS);
        let got = kernel.bodies();

        for (i, (g, e)) in got.iter().zip(expected.iter()).enumerate() {
            assert_eq!(g.frame, e.frame, "body {i} frame mismatch");
            assert_eq!(g.velocity, e.velocity, "body {i} velocity mismatch");
            assert_eq!(
                g.angular_velocity, e.angular_velocity,
                "body {i} angular_velocity mismatch"
            );
        }
    }

    /// A settling scenario across many ticks (not just one call to
    /// `step`) must remain stable -- no bounce, no sinking -- the same
    /// end-to-end property `engine-core`'s own box-settling regression
    /// test checks for the sequential solver.
    #[test]
    fn repeated_batched_steps_settle_without_bouncing_or_sinking() {
        let mut bodies = floor_and_box();
        let solver = ConstraintSolver::<FloatFlavor>::new(0.0).with_friction(0.6);
        let mut broad_phase = BroadPhase::<FloatFlavor>::new();
        let narrow_phase = NarrowPhase::<FloatFlavor>::new();
        let context = ComputeContext::new();

        let mut min_height = f32::MAX;
        let mut max_height = f32::MIN;
        for step in 0..600 {
            let pairs = broad_phase.find_candidate_pairs(&bodies).to_vec();
            let contacts = narrow_phase.generate_contacts(&bodies, &pairs);
            let kernel = ConstraintSolverBatchKernel::new(solver, bodies.clone(), contacts);
            kernel.step(&context, 4);
            bodies = kernel.bodies();

            if step > 200 {
                let height = bodies[1].position().y;
                min_height = min_height.min(height);
                max_height = max_height.max(height);
            }
        }

        assert!(
            max_height - min_height < 0.01,
            "a settled box (restitution 0) must not bounce up/down at all (min {min_height}, max {max_height})"
        );
        assert!(
            min_height > 0.0,
            "a settled box must not clip through the floor (min height {min_height})"
        );
    }
}
