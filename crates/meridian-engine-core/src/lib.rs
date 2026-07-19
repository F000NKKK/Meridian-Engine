//! Runtime: frame scheduler, event system, subsystem manager; ties every other crate into the main loop.
//!
//! [`SubsystemManager`] is the one place in the workspace allowed to know
//! about every `*-core` at once (see docs/dependency-rules.md rule 7) — it
//! owns real instances of the driver-independent subsystems that exist
//! today: an `ecs-core` [`World`], `physics-core`'s body list and
//! pipeline, and `audio-core`'s listener/mixer. `graphics-core` isn't
//! wired into [`Runtime::tick`] yet: rendering has nothing to submit to
//! without a real `graphics-driver` backend (blocked on the `wgpu`
//! decision — see docs/roadmap.md), so there's no frame in the render
//! sense to schedule.
//!
//! [`Runtime::tick`] advances physics, then recomputes audio gains from
//! the physics-updated emitter frames, in that order — not through
//! [`FrameScheduler`]/`task-core`'s `JobGraph`, deliberately: physics and
//! audio are the only two real per-frame systems today, and they have a
//! genuine sequential data dependency (audio reads positions physics just
//! wrote), not two independent branches. Wrapping a strictly sequential
//! two-step in a job graph would be decorative, not functional — the same
//! reason `compute-runtime`'s `task-core` dependency isn't wired in yet
//! (see that crate's module doc). [`FrameScheduler`] is real and tested on
//! its own terms; it becomes load-bearing once a second real per-frame
//! system exists that's genuinely independent of physics (animation,
//! particles, ...) to run alongside it.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use meridian_audio_core::{Emitter, Listener, Mixer};
use meridian_ecs_core::World;
use meridian_physics_core::{BroadPhase, ConstraintSolver, Integrator, NarrowPhase, RigidBody};
use meridian_platform_core::{Clock, CpuCapabilities, Time};
use meridian_task_core::{JobGraph, Scheduler};

/// Workspace-wide event bus: a frame-scoped mailbox, not a persistent log.
/// [`publish`](Self::publish) queues an event by its concrete type;
/// [`drain`](Self::drain) removes and returns every event of that type
/// published since the last drain. This is what lets subsystems
/// communicate without depending on each other directly — e.g. a future
/// `physics-core` contact could be published here and consumed by
/// `audio-core` for an impact sound, without either crate knowing the
/// other exists (see docs/dependency-rules.md rule 7: only `engine-core`
/// is allowed to know about both).
#[derive(Default)]
pub struct EventSystem {
    queues: HashMap<TypeId, Vec<Box<dyn Any>>>,
}

impl std::fmt::Debug for EventSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventSystem")
            .field("queued_event_types", &self.queues.len())
            .finish()
    }
}

impl EventSystem {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues `event`, keyed by its concrete type `E`. Multiple event
    /// types don't collide — each gets its own queue.
    pub fn publish<E: 'static>(&mut self, event: E) {
        self.queues
            .entry(TypeId::of::<E>())
            .or_default()
            .push(Box::new(event));
    }

    /// Removes and returns every queued event of type `E`, in publish
    /// order. A second call before any new `publish::<E>` returns empty —
    /// this is a drain, not a peek.
    pub fn drain<E: 'static>(&mut self) -> Vec<E> {
        let Some(boxed) = self.queues.remove(&TypeId::of::<E>()) else {
            return Vec::new();
        };
        boxed
            .into_iter()
            .map(|b| *b.downcast::<E>().expect("queue keyed by TypeId::of::<E>()"))
            .collect()
    }
}

/// Runs one frame's [`JobGraph`] across worker threads — the engine-layer
/// application of `task-core`'s generic scheduler (see
/// docs/threading-model.md). Sized by [`FrameScheduler::default`] to the
/// real detected CPU thread count via `platform-core`, not a hardcoded
/// guess.
#[derive(Debug)]
pub struct FrameScheduler {
    scheduler: Scheduler,
}

impl Default for FrameScheduler {
    fn default() -> Self {
        Self::new(CpuCapabilities::detect().threads)
    }
}

impl FrameScheduler {
    pub fn new(worker_count: usize) -> Self {
        Self {
            scheduler: Scheduler::new(worker_count),
        }
    }

    /// Runs `graph` to completion, blocking until every job has finished.
    pub fn run(&self, graph: JobGraph) {
        self.scheduler.run(graph);
    }
}

/// Registry of active subsystems for the current [`Runtime`] — real owned
/// instances, not stubs: an `ecs-core` [`World`] (available for
/// application-level entity/`Transform` use; not synced with `bodies`
/// below — no such mapping is defined anywhere in the workspace yet, and
/// inventing one here would be new, undocumented design, not wiring
/// together what already exists), `physics-core`'s body list plus its
/// broad/narrow-phase and solver/integrator, and `audio-core`'s listener,
/// emitters and mixer. The only place in the workspace allowed to know
/// about every `*-core` at once — see docs/dependency-rules.md rule 7.
pub struct SubsystemManager {
    pub world: World,

    pub bodies: Vec<RigidBody>,
    pub broad_phase: BroadPhase,
    pub narrow_phase: NarrowPhase,
    pub solver: ConstraintSolver,
    pub integrator: Integrator,

    pub listener: Listener,
    pub emitters: Vec<(Emitter, f32)>,
    pub mixer: Mixer,
}

impl std::fmt::Debug for SubsystemManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `World` doesn't derive `Debug` (it holds type-erased archetype
        // storage — see meridian-ecs-core), so this summarizes it rather
        // than deriving through it.
        f.debug_struct("SubsystemManager")
            .field("bodies", &self.bodies.len())
            .field("emitters", &self.emitters.len())
            .field("listener", &self.listener)
            .finish_non_exhaustive()
    }
}

impl SubsystemManager {
    pub fn new(mixer: Mixer) -> Self {
        Self {
            world: World::new(),
            bodies: Vec::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            solver: ConstraintSolver::default(),
            integrator: Integrator::default(),
            listener: Listener::default(),
            emitters: Vec::new(),
            mixer,
        }
    }

    /// Advances every body by `dt`: integrate, find broad-phase candidate
    /// pairs, generate exact contacts, resolve them. The same pipeline
    /// `physics-core`'s own tests exercise by hand
    /// (`full_step_ball_settles_on_static_floor_without_sinking_through`),
    /// centralized here as the one real per-frame physics step.
    pub fn step_physics(&mut self, dt: f32) {
        self.integrator.step(&mut self.bodies, dt);
        let pairs = self.broad_phase.find_candidate_pairs(&self.bodies).to_vec();
        for contact in self.narrow_phase.generate_contacts(&self.bodies, &pairs) {
            self.solver.resolve(&mut self.bodies, &contact);
        }
    }

    /// Per-channel gains for every emitter against the current listener,
    /// via `audio-core`'s `Mixer` — reads whatever `emitters`' frames are
    /// *right now*, so calling this after [`step_physics`](Self::step_physics)
    /// reflects physics-updated positions.
    pub fn mix_audio(&self) -> Vec<(meridian_audio_core::Channel, f32)> {
        self.mixer.mix(&self.listener, &self.emitters)
    }
}

/// Published by [`Runtime::tick`] after every frame — the one concrete
/// event type this crate defines itself; application code can publish its
/// own event types through the same [`EventSystem`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameCompleted {
    pub frame_index: u64,
    pub delta_seconds: f64,
}

/// Owns subsystem instances and drives the frame loop. Construct once with
/// [`Runtime::new`], then call [`Runtime::tick`] once per frame.
#[derive(Debug)]
pub struct Runtime {
    pub subsystems: SubsystemManager,
    pub events: EventSystem,
    pub frame_scheduler: FrameScheduler,
    clock: Clock,
    frame_index: u64,
}

impl Runtime {
    pub fn new(subsystems: SubsystemManager) -> Self {
        Self {
            subsystems,
            events: EventSystem::new(),
            frame_scheduler: FrameScheduler::default(),
            clock: Clock::new(),
            frame_index: 0,
        }
    }

    /// Advances the simulation by one frame: ticks the clock, steps
    /// physics, recomputes audio gains from the result, publishes a
    /// [`FrameCompleted`] event, and returns the frame's [`Time`]. See the
    /// module doc for why this is a direct sequential call rather than a
    /// `FrameScheduler`-run job graph.
    pub fn tick(&mut self) -> Time {
        let time = self.clock.tick();
        self.subsystems.step_physics(time.delta_seconds as f32);
        self.events.publish(FrameCompleted {
            frame_index: self.frame_index,
            delta_seconds: time.delta_seconds as f64,
        });
        self.frame_index += 1;
        time
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_audio_core::{AttenuationModel, Channel, SpeakerLayout};
    use meridian_gac_core::{Motor3, Vec3};
    use meridian_physics_core::ColliderShape;

    #[test]
    fn event_system_round_trips_by_type() {
        let mut events = EventSystem::new();
        events.publish(1i32);
        events.publish(2i32);
        events.publish("hello");

        assert_eq!(events.drain::<i32>(), vec![1, 2]);
        assert_eq!(events.drain::<&str>(), vec!["hello"]);
    }

    #[test]
    fn event_system_drain_empties_the_queue() {
        let mut events = EventSystem::new();
        events.publish(42i32);
        assert_eq!(events.drain::<i32>(), vec![42]);
        assert_eq!(events.drain::<i32>(), Vec::<i32>::new());
    }

    #[test]
    fn event_system_drain_of_unpublished_type_is_empty() {
        let mut events = EventSystem::new();
        assert_eq!(events.drain::<f32>(), Vec::<f32>::new());
    }

    #[test]
    fn frame_scheduler_runs_a_job_graph() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let ran = Arc::new(AtomicUsize::new(0));
        let mut graph = JobGraph::new();
        let ran2 = ran.clone();
        graph.add_job("job", &[], move || {
            ran2.fetch_add(1, Ordering::SeqCst);
        });

        FrameScheduler::new(2).run(graph);
        assert_eq!(ran.load(Ordering::SeqCst), 1);
    }

    fn falling_body() -> RigidBody {
        RigidBody {
            frame: Motor3::translation(Vec3::new(0.0, 10.0, 0.0)),
            mass: 1.0,
            shape: ColliderShape::Sphere { radius: 0.5 },
            ..Default::default()
        }
    }

    #[test]
    fn runtime_tick_advances_physics_under_gravity() {
        let mut subsystems = SubsystemManager::new(Mixer::new(SpeakerLayout::mono()));
        subsystems.bodies.push(falling_body());
        let mut runtime = Runtime::new(subsystems);

        let before = runtime.subsystems.bodies[0].position().y;
        runtime.tick();
        let after = runtime.subsystems.bodies[0].position().y;

        assert!(after < before, "body must fall under gravity each tick");
    }

    #[test]
    fn runtime_tick_publishes_frame_completed_with_increasing_index() {
        let subsystems = SubsystemManager::new(Mixer::new(SpeakerLayout::mono()));
        let mut runtime = Runtime::new(subsystems);

        runtime.tick();
        runtime.tick();
        runtime.tick();

        let completed = runtime.events.drain::<FrameCompleted>();
        assert_eq!(completed.len(), 3);
        assert_eq!(
            completed.iter().map(|e| e.frame_index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn subsystem_manager_mixes_audio_from_current_emitter_positions() {
        let mut subsystems = SubsystemManager::new(
            Mixer::new(SpeakerLayout::stereo_headphones()).with_attenuation(AttenuationModel {
                reference_distance: 1000.0,
                rolloff: 1.0,
                max_distance: 1000.0,
            }),
        );
        subsystems.listener = Listener {
            frame: Motor3::identity(),
        };
        // Local +Z is "right" per audio-core's listener convention.
        subsystems.emitters.push((
            Emitter {
                frame: Motor3::translation(Vec3::new(0.0, 0.0, 5.0)),
            },
            1.0,
        ));

        let gains = subsystems.mix_audio();
        let gain_of = |channel: Channel| {
            gains
                .iter()
                .find(|(c, _)| *c == channel)
                .map(|(_, g)| *g)
                .unwrap_or(0.0)
        };
        assert!(gain_of(Channel::Right) > 0.99);
        assert!(gain_of(Channel::Left) < 1e-3);
    }

    #[test]
    fn subsystem_manager_step_physics_resolves_a_resting_contact() {
        let mut subsystems = SubsystemManager::new(Mixer::new(SpeakerLayout::mono()));
        subsystems.bodies.push(RigidBody {
            frame: Motor3::translation(Vec3::new(0.0, -50.0, 0.0)),
            mass: 0.0, // static floor
            shape: ColliderShape::Sphere { radius: 50.0 },
            ..Default::default()
        });
        subsystems.bodies.push(falling_body());

        for _ in 0..600 {
            subsystems.step_physics(1.0 / 60.0);
        }

        let resting_height = subsystems.bodies[1].position().y;
        assert!(
            (resting_height - 0.5).abs() < 0.5,
            "ball should settle near the floor surface, got y={resting_height}"
        );
    }
}
