//! Runtime: frame scheduler, event system, subsystem manager; ties every other crate into the main loop.
//!
//! [`SubsystemManager`] is the one place in the workspace allowed to know
//! about every `*-core` at once (see docs/dependency-rules.md rule 7) — it
//! owns real instances of the driver-independent subsystems that exist
//! today: an `ecs-core` [`World`], `physics-core`'s body list and
//! pipeline, and `audio-core`'s listener/mixer. `graphics-core` itself
//! has a real scene/material vocabulary and GPU-submission bridge now
//! (`Scene3D`/`Material`/`Light`, `SceneRenderer` — see
//! `graphics-core::scene`/`submission`); it isn't wired into
//! [`Runtime::tick`] anyway, for a different reason: presenting a frame
//! needs a real windowed `Device`/`Surface` (driver state), and this
//! crate deliberately never depends on `graphics-driver` (see
//! docs/dependency-rules.md: `engine-core` depends on `graphics-core`,
//! not drivers) — a windowed app would compose
//! [`platform_core::run_windowed_app`](meridian_platform_core::run_windowed_app)
//! with its own `graphics-driver::Device`/`Surface` and
//! `graphics-core::SceneRenderer` around [`Runtime::tick`]'s [`Time`]
//! for animation/physics timing, gaining `graphics-driver`/`Surface`
//! access without `Runtime` itself ever depending on either.
//!
//! **That composition pattern is designed, not yet proven — no example
//! in this workspace actually uses `Runtime`/`SubsystemManager` today.**
//! `magic_figures` and `physic_figures` each hand-roll their own
//! physics stepping and audio wiring directly against
//! `physics-core`/`audio-core` (see each example's own code), not
//! through this crate — a real, open gap between "tested in isolation"
//! and "proven end-to-end," not a documentation nit. See
//! docs/roadmap.md's `Runtime`-adoption entry for the follow-up.
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

/// Velocity-relaxation pass count for [`PhysicsSubsystem::step`] — matches
/// `examples/physic_figures`' own tuned value, chosen because a
/// box/pyramid face-face manifold (up to 4 points) needs several passes
/// before each point's impulse is computed against the others' relaxed
/// (not stale pre-solve) velocity.
const RELAXATION_ITERATIONS: u32 = 4;

/// `physics-core`'s body list plus its broad/narrow-phase and
/// solver/integrator — everything [`PhysicsSubsystem::step`] needs,
/// bundled as one independently-ownable/-lockable unit. Split out of
/// [`SubsystemManager`] (which used to hold these fields directly) so a
/// caller that wants to run physics on its own thread/lock (e.g.
/// `meridian-sdk`'s job-graph pipeline — see docs/roadmap.md's
/// `Runtime`-adoption entry) can do so without also taking
/// [`AudioSubsystem`]'s or `World`'s lock, while `SubsystemManager`
/// itself keeps owning the actual stepping *logic* (rule 7: engine-core
/// is where cross-`*-core` domain logic like this lives, not a
/// downstream orchestration crate re-deriving it).
#[derive(Debug)]
pub struct PhysicsSubsystem {
    pub bodies: Vec<RigidBody>,
    pub broad_phase: BroadPhase,
    pub narrow_phase: NarrowPhase,
    pub solver: ConstraintSolver,
    pub integrator: Integrator,
}

impl Default for PhysicsSubsystem {
    fn default() -> Self {
        Self {
            bodies: Vec::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            solver: ConstraintSolver::default(),
            integrator: Integrator::default(),
        }
    }
}

impl PhysicsSubsystem {
    /// Advances every body by `dt`: integrate, then relax contacts over
    /// `RELAXATION_ITERATIONS` velocity-only passes before a single
    /// final positional correction pass. **Not** one `resolve()` call per
    /// contact — that was this method's original shape (back when it
    /// lived directly on `SubsystemManager`), and it carries the exact
    /// "cube/pyramid bounces up/down and clips through the floor" bug
    /// `ConstraintSolver::resolve`'s own doc comment describes: a
    /// box/pyramid manifold is up to 4 contact points sharing one normal,
    /// and a single combined velocity+positional pass over all of them
    /// pushes the body's position by the same correction several times
    /// per tick. A single sphere-sphere contact (this method's own
    /// regression test) never exhibited it, so the bug went unnoticed
    /// here even after `examples/physic_figures` independently
    /// discovered and fixed it in its own hand-rolled physics stepping —
    /// see docs/roadmap.md's `Runtime`-adoption entry for that history.
    /// This method now mirrors `physic_figures::PhysicsRig::step`'s
    /// proven-correct shape exactly, centralized here so every caller
    /// gets multi-point-manifold stability for free instead of
    /// rediscovering it.
    pub fn step(&mut self, dt: f32) {
        self.integrator.step(&mut self.bodies, dt);

        for _ in 0..RELAXATION_ITERATIONS {
            let pairs = self.broad_phase.find_candidate_pairs(&self.bodies).to_vec();
            for contact in self.narrow_phase.generate_contacts(&self.bodies, &pairs) {
                self.solver.resolve_velocity(&mut self.bodies, &contact);
            }
        }
        // Positional correction exactly once per tick, against the final
        // (velocity-relaxed) contact set — see `ConstraintSolver::resolve`'s
        // doc comment for why calling it more than once per tick over-corrects.
        let pairs = self.broad_phase.find_candidate_pairs(&self.bodies).to_vec();
        for contact in self.narrow_phase.generate_contacts(&self.bodies, &pairs) {
            self.solver
                .apply_positional_correction(&mut self.bodies, &contact);
        }
    }
}

/// `audio-core`'s listener, emitters and gain mixer — bundled as one
/// independently-ownable/-lockable unit for the same reason
/// [`PhysicsSubsystem`] is split out. **One specific, opinionated audio
/// path (`Mixer::mix`'s per-channel gain model), not "the" audio
/// pipeline every consumer must fit** — see [`AudioSubsystem::mix`]'s
/// own doc comment.
#[derive(Debug)]
pub struct AudioSubsystem {
    pub listener: Listener,
    pub emitters: Vec<(Emitter, f32)>,
    pub mixer: Mixer,
}

impl AudioSubsystem {
    pub fn new(mixer: Mixer) -> Self {
        Self {
            listener: Listener::default(),
            emitters: Vec::new(),
            mixer,
        }
    }

    /// Per-channel gains for every emitter against the current listener,
    /// via `audio-core`'s `Mixer` — reads whatever `emitters`' frames are
    /// *right now*, so calling this after
    /// [`PhysicsSubsystem::step`] reflects physics-updated positions.
    ///
    /// This is one specific, opinionated audio path (`Mixer::mix`'s
    /// per-channel gain model), not "the" audio pipeline every consumer
    /// must fit. A richer consumer (`examples/magic_figures`, which
    /// needs `BinauralRenderer`'s real per-sample stereo synthesis —
    /// ITD, head-shadow filtering, its own declick stage — none of which
    /// `Mixer::mix`'s gains can express) is deliberately **not** forced
    /// through this type: bolting a second fixed pipeline (a
    /// `binaural: Option<BinauralRenderer>` field plus a matching
    /// hardcoded declick stage) onto `AudioSubsystem` alongside this one
    /// would repeat the same mistake — engine-core dictating a specific
    /// effect chain — rather than fixing it. A real, composable
    /// rendering-pipeline abstraction (stages an app assembles itself,
    /// `mixer` becoming one possible stage among others) is real design
    /// work, tracked as follow-up rather than improvised mid-migration —
    /// see docs/roadmap.md's `Runtime`-adoption entry. Until that
    /// exists, `examples/magic_figures` keeps owning its
    /// `BinauralRenderer`/`Declicker` pipeline directly.
    pub fn mix(&self) -> Vec<(meridian_audio_core::Channel, f32)> {
        self.mixer.mix(&self.listener, &self.emitters)
    }
}

/// Registry of active subsystems for the current [`Runtime`] — real owned
/// instances, not stubs: an `ecs-core` [`World`] (available for
/// application-level entity/`Transform` use; not synced with
/// [`physics`](Self::physics)'s bodies — no such mapping is defined
/// anywhere in the workspace yet, and inventing one here would be new,
/// undocumented design, not wiring together what already exists),
/// [`PhysicsSubsystem`] and [`AudioSubsystem`]. The only place in the
/// workspace allowed to know about every `*-core` at once — see
/// docs/dependency-rules.md rule 7.
pub struct SubsystemManager {
    pub world: World,
    pub physics: PhysicsSubsystem,
    pub audio: AudioSubsystem,
}

impl std::fmt::Debug for SubsystemManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `World` doesn't derive `Debug` (it holds type-erased archetype
        // storage — see meridian-ecs-core), so this summarizes it rather
        // than deriving through it.
        f.debug_struct("SubsystemManager")
            .field("bodies", &self.physics.bodies.len())
            .field("emitters", &self.audio.emitters.len())
            .field("listener", &self.audio.listener)
            .finish_non_exhaustive()
    }
}

impl SubsystemManager {
    pub fn new(mixer: Mixer) -> Self {
        Self {
            world: World::new(),
            physics: PhysicsSubsystem::default(),
            audio: AudioSubsystem::new(mixer),
        }
    }

    /// Forwards to [`PhysicsSubsystem::step`] — see that method's own
    /// doc comment.
    pub fn step_physics(&mut self, dt: f32) {
        self.physics.step(dt);
    }

    /// Forwards to [`AudioSubsystem::mix`] — see that method's own doc
    /// comment.
    pub fn mix_audio(&self) -> Vec<(meridian_audio_core::Channel, f32)> {
        self.audio.mix()
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
        meridian_foundation::log_info!("engine-core Runtime initialized");
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
            delta_seconds: time.delta_seconds,
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
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

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
        subsystems.physics.bodies.push(falling_body());
        let mut runtime = Runtime::new(subsystems);

        // Checking velocity, not position: position starts at y=10.0, and
        // a real wall-clock dt (microseconds, since Clock isn't a fixed
        // step) produces a position delta many orders of magnitude below
        // f32's precision at that magnitude — it would round away to
        // nothing even though physics genuinely ran. Velocity starts at
        // exactly 0.0, so any nonzero gravity contribution is visible
        // regardless of how small the real elapsed dt was.
        for _ in 0..5 {
            runtime.tick();
        }
        assert!(
            runtime.subsystems.physics.bodies[0].velocity.y < 0.0,
            "gravity must have been applied across ticks"
        );
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
        subsystems.audio.listener = Listener {
            frame: Motor3::identity(),
        };
        // Local +Z is "right" per audio-core's listener convention.
        subsystems.audio.emitters.push((
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
        subsystems.physics.bodies.push(RigidBody {
            frame: Motor3::translation(Vec3::new(0.0, -50.0, 0.0)),
            mass: 0.0, // static floor
            shape: ColliderShape::Sphere { radius: 50.0 },
            ..Default::default()
        });
        subsystems.physics.bodies.push(falling_body());

        for _ in 0..600 {
            subsystems.step_physics(1.0 / 60.0);
        }

        let resting_height = subsystems.physics.bodies[1].position().y;
        assert!(
            (resting_height - 0.5).abs() < 0.5,
            "ball should settle near the floor surface, got y={resting_height}"
        );
    }

    /// A box-on-box manifold (up to 4 contact points, unlike the
    /// single-point sphere case above) — regression coverage for the bug
    /// `step_physics`'s doc comment describes: the original
    /// one-`resolve()`-call-per-contact shape over-applied positional
    /// correction on every relaxation-worthy contact set, bouncing a
    /// settled box up/down and eventually clipping it through the floor.
    /// A sphere never exercised this (always exactly one contact point),
    /// which is exactly why the bug went unnoticed in this method even
    /// after `examples/physic_figures` independently found and fixed it
    /// in its own hand-rolled stepping — see
    /// `meridian-physics-core::float`'s own
    /// `cuboid_settles_without_runaway_spin` test for the equivalent,
    /// non-centralized version of this same assertion.
    #[test]
    fn subsystem_manager_step_physics_settles_a_box_without_bouncing_or_sinking() {
        use meridian_physics_core::ConstraintSolver;

        let mut subsystems = SubsystemManager::new(Mixer::new(SpeakerLayout::mono()));
        subsystems.physics.solver = ConstraintSolver::new(0.0).with_friction(0.6);
        subsystems.physics.bodies.push(RigidBody {
            frame: Motor3::translation(Vec3::new(0.0, -0.5, 0.0)),
            mass: 0.0, // static floor
            shape: ColliderShape::Cuboid {
                half_extents: Vec3::new(14.0, 0.5, 14.0),
            },
            ..Default::default()
        });
        subsystems.physics.bodies.push(RigidBody {
            frame: Motor3::translation(Vec3::new(0.0, 3.0, 0.0)),
            mass: 1.0,
            shape: ColliderShape::Cuboid {
                half_extents: Vec3::new(0.6, 0.6, 0.6),
            },
            ..Default::default()
        });

        let mut min_height_after_landing = f32::MAX;
        let mut max_height_after_landing = f32::MIN;
        for step in 0..600 {
            subsystems.step_physics(1.0 / 60.0);
            if step > 200 {
                let height = subsystems.physics.bodies[1].position().y;
                min_height_after_landing = min_height_after_landing.min(height);
                max_height_after_landing = max_height_after_landing.max(height);
            }
        }

        assert!(
            max_height_after_landing - min_height_after_landing < 0.01,
            "a settled box (restitution 0) must not bounce up/down at all \
             (min {min_height_after_landing}, max {max_height_after_landing})"
        );
        assert!(
            min_height_after_landing > 0.0,
            "a settled box must not clip through the floor (min height {min_height_after_landing})"
        );
    }
}
