//! Detects when pairs of bodies begin touching across a physics step.

use std::collections::HashMap;

use meridian_gac::Vec3;

use crate::{ContactEvent, PhysicsSubsystem};

/// Consecutive ticks a pair must be apart before it counts as separated.
/// The solver's positional correction can make a resting pair report no
/// overlap on a tick; without this a resting body would "land" repeatedly.
const SEPARATION_TICKS: u8 = 3;

/// Reports pairs that began touching, once per touch.
///
/// Call [`before_step`](Self::before_step), run any
/// [`PhysicsStepper`](crate::PhysicsStepper), then
/// [`after_step`](Self::after_step). It reads only the
/// [`PhysicsSubsystem`], so every stepper behaves the same and the stepper
/// contract is untouched. A pair keeps touching until it has been apart for
/// three consecutive ticks; only a new touch whose closing
/// speed reaches `min_impact_speed` produces an event.
#[derive(Debug, Clone, Default)]
pub struct ContactWatcher {
    min_impact_speed: f32,
    velocities: Vec<Vec3>,
    /// Pair `(smaller, larger)` -> ticks since it was last seen touching.
    touching: HashMap<(usize, usize), u8>,
}

impl ContactWatcher {
    /// A watcher that ignores touches closing slower than `min_impact_speed`.
    #[must_use]
    pub fn new(min_impact_speed: f32) -> Self {
        Self {
            min_impact_speed,
            ..Self::default()
        }
    }

    /// Records the bodies' velocities, to measure the impact of the coming step.
    pub fn before_step(&mut self, physics: &PhysicsSubsystem) {
        self.velocities.clear();
        self.velocities
            .extend(physics.bodies.iter().map(|body| body.velocity));
    }

    /// Finds the pairs touching after the step and returns an event for each
    /// one that was not touching before, ordered by body indices.
    pub fn after_step(&mut self, physics: &mut PhysicsSubsystem) -> Vec<ContactEvent> {
        let deepest = Self::deepest_contacts(physics);
        let mut events = Vec::new();
        for (&pair, &contact) in &deepest {
            let is_new = self.touching.insert(pair, 0).is_none();
            let event = self.event_for(pair, contact);
            if is_new && event.impact_speed >= self.min_impact_speed {
                events.push(event);
            }
        }
        self.touching.retain(|pair, apart_ticks| {
            if deepest.contains_key(pair) {
                return true;
            }
            *apart_ticks += 1;
            *apart_ticks < SEPARATION_TICKS
        });
        events.sort_by_key(|event| (event.a, event.b));
        events
    }

    /// The deepest contact of each touching pair, keyed `(smaller, larger)`.
    fn deepest_contacts(
        physics: &mut PhysicsSubsystem,
    ) -> HashMap<(usize, usize), meridian_physics_domain::Contact> {
        let pairs = physics
            .broad_phase
            .find_candidate_pairs(&physics.bodies)
            .to_vec();
        let mut deepest = HashMap::new();
        for contact in physics
            .narrow_phase
            .generate_contacts(&physics.bodies, &pairs)
        {
            let key = (contact.a.min(contact.b), contact.a.max(contact.b));
            let slot = deepest.entry(key).or_insert(contact);
            if contact.penetration > slot.penetration {
                *slot = contact;
            }
        }
        deepest
    }

    fn event_for(
        &self,
        (a, b): (usize, usize),
        contact: meridian_physics_domain::Contact,
    ) -> ContactEvent {
        // `contact.normal` points from `contact.a` to `contact.b`; report it from `a` to `b`.
        let normal = if contact.a == a {
            contact.normal
        } else {
            -contact.normal
        };
        let velocity_of = |index: usize| self.velocities.get(index).copied().unwrap_or(Vec3::ZERO);
        let closing = (velocity_of(a) - velocity_of(b)).dot(normal);
        ContactEvent {
            a,
            b,
            point: contact.point,
            normal,
            impact_speed: closing.max(0.0),
        }
    }
}

#[cfg(test)]
mod tests;
