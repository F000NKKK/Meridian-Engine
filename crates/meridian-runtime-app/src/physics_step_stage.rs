//! The stage that steps physics.

use meridian_physics_app::{ContactWatcher, PhysicsStepper, PhysicsSubsystem};

use crate::{EventSystem, Shared, Stage};

/// Built-in stage that advances a shared [`PhysicsSubsystem`] by a fixed
/// `dt` every run, through whichever [`PhysicsStepper`] the composition
/// root chose: the sequential CPU loop (`SequentialStepper`) or the batched
/// compute stepper of `meridian-physics-wgpu`. Same algorithm, a
/// different dispatch shape; every stepper passes one contract suite.
pub struct PhysicsStepStage {
    /// Fixed simulation timestep in seconds.
    pub dt: f32,
    stepper: Box<dyn PhysicsStepper>,
    physics: Shared<PhysicsSubsystem>,
    contacts: Option<(ContactWatcher, Shared<EventSystem>)>,
}

impl PhysicsStepStage {
    /// A stage stepping `physics` by `dt` with `stepper`.
    #[must_use]
    pub fn new(
        dt: f32,
        stepper: impl PhysicsStepper + 'static,
        physics: Shared<PhysicsSubsystem>,
    ) -> Self {
        Self {
            dt,
            stepper: Box::new(stepper),
            physics,
            contacts: None,
        }
    }

    /// Also publishes a `ContactEvent` into `events` for every pair that begins
    /// touching during a step, as `watcher` judges (ADR 031).
    #[must_use]
    pub fn with_contact_events(
        mut self,
        watcher: ContactWatcher,
        events: Shared<EventSystem>,
    ) -> Self {
        self.contacts = Some((watcher, events));
        self
    }
}

impl std::fmt::Debug for PhysicsStepStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhysicsStepStage")
            .field("dt", &self.dt)
            .finish_non_exhaustive()
    }
}

impl Stage for PhysicsStepStage {
    fn run(&mut self) {
        let mut physics = self.physics.lock();
        if let Some((watcher, _)) = &mut self.contacts {
            watcher.before_step(&physics);
        }
        self.stepper.step(&mut physics, self.dt);
        if let Some((watcher, events)) = &mut self.contacts {
            let mut bus = events.lock();
            for event in watcher.after_step(&mut physics) {
                bus.publish(event);
            }
        }
    }
}
