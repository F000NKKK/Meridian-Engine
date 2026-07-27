//! `Runtime`: the single entry point tying every other crate into the
//! main loop. Owns real instances of the driver-independent subsystems
//! that exist today — an `ecs-core` [`World`], [`PhysicsSubsystem`]
//! (`physics-core`'s body list, broad/narrow-phase, solver/integrator)
//! and [`AudioSubsystem`] (`audio-core`'s listener, emitters, mixer) —
//! this is still the one place in the workspace allowed to know about
//! every `*-core` at once (see docs/dependency-rules.md rule 7).
//!
//! **One mechanism, not two.** Earlier revisions of this crate had a
//! plain sequential `Runtime::tick` (physics-then-audio, hardcoded
//! order, no extensibility) *and* a separate `meridian-sdk`-level
//! `Pipeline` built on `task-core::JobGraph` (composable, dependency-
//! ordered `Stage`s, fine-grained locking) — two competing answers to
//! "how does a frame's work get run," with the second only invented
//! because the first couldn't be extended without engine-core knowing
//! about every possible consumer's specific need (`BinauralRenderer`'s
//! per-sample synthesis being the concrete case that broke it). That
//! split is gone: [`Stage`]/[`StageContext`]/[`Runtime`] (the
//! `JobGraph`-based mechanism, promoted here from where it used to live
//! in `meridian-sdk`) *is* this crate's `Runtime` now — there is no
//! second, lower-level "raw pipeline" type an application reaches for
//! instead; [`Runtime::tick`]/[`Runtime::tick_only`] are the only
//! per-frame entry points, and each rebuilds and runs a fresh `JobGraph`
//! every call internally (see
//! [`Runtime::tick`]'s own doc for why a fresh graph, not a cached one).
//!
//! **Locking: fine-grained, deadlock-free by construction.**
//! [`RuntimeState`] splits shared state into independently-lockable
//! pieces ([`World`], [`PhysicsSubsystem`], [`AudioSubsystem`]) so two
//! stages that only touch *different* resources genuinely don't contend
//! on each other's lock. The risk with more than one lock is a classic
//! lock-order-inversion deadlock: stage A locks physics-then-audio
//! while an independent stage B locks audio-then-physics, running
//! concurrently. [`StageContext`] makes that inversion structurally
//! unrepresentable rather than documenting a convention a stage author
//! could still get wrong: every accessor takes exactly one lock, and
//! the only ways to hold more than one at once
//! ([`StageContext::physics_and_audio`]/
//! [`StageContext::world_physics_and_audio`]) always acquire them in
//! one fixed order (`world`, then `physics`, then `audio`) internally.
//! There is no API path that lets calling code choose the order itself.
//!
//! **Extensibility — this is the actual point of the merge.** Physics
//! ([`PhysicsStepStage`] — a plain sequential loop; or
//! [`PhysicsComputeStepStage`], its batched-dispatch counterpart
//! through `meridian-physics-compute`'s kernels, see that type's own
//! doc for when to reach for which) and arbitrary GPU/CPU compute
//! ([`ComputeStage`], wrapping any `compute-runtime::ComputeKernel` —
//! `gac-compute`/`physics-compute`'s own kernels included, or a game's
//! own) are both just [`Stage`]s registered the same way, dependency-
//! ordered against each other and anything else an application adds.
//! Rendering is the one thing that *can't* live as a built-in stage
//! here — presenting a frame needs a real windowed `Device`/`Surface`
//! (driver state), and this crate deliberately never depends on
//! `graphics-driver` (only `graphics-core` — see
//! docs/dependency-rules.md). That isn't a gap in this mechanism,
//! though: a render-presenting [`Stage`] is exactly what an application
//! (via `meridian-sdk`, the one crate allowed to hold both `Runtime` and
//! `*-driver` types) implements and registers itself, through the exact
//! same `Stage` trait every other stage uses — not a second, parallel
//! "render pipeline" mechanism. See `meridian-sdk`'s own module docs for
//! that implementation once it exists.
//!
//! **`examples/physic_figures` proves this end-to-end** — physics *and*
//! rendering both go through one `Runtime`: a registered
//! [`PhysicsStepStage`] and a `meridian-sdk`-implemented render-
//! presenting `Stage`, the concrete case the previous paragraph
//! describes. They run at different multiplicities per real display
//! frame (a fixed-timestep accumulator calls the physics stage 0-8
//! times to catch up; rendering must run exactly once) — see
//! [`Runtime::tick_only`]'s own doc for the selective-tick mechanism
//! that makes both still go through this one `Runtime` rather than
//! either bypassing it. `examples/magic_figures` still doesn't use
//! `Runtime` at all: it has no physics bodies (pure kinematic orbit
//! motion) and needs `audio-core::BinauralRenderer`'s real per-sample
//! stereo synthesis, which doesn't reduce to a `StageContext`-shaped
//! `Stage` cleanly (its inputs — orbiting shapes' positions, the
//! free-fly camera's pose — are plain application state, not anything
//! `RuntimeState` owns) — see [`AudioSubsystem::mix`]'s own doc comment
//! for why forcing it through `AudioSubsystem` was rejected instead.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use meridian_audio_core::{Emitter, Listener, Mixer};
use meridian_compute_runtime::{ComputeContext, ComputeKernel, ComputeScheduler, DispatchSize};
use meridian_ecs_core::World;
use meridian_physics_compute::broad_phase::BroadPhasePairsKernel;
use meridian_physics_compute::constraint_solver::ConstraintSolverBatchKernel;
use meridian_physics_compute::narrow_phase::GenerateContactsKernel;
use meridian_physics_compute::rigid_body::RigidBodyIntegratorKernel;
use meridian_physics_core::{BroadPhase, ConstraintSolver, Integrator, NarrowPhase, RigidBody};
use meridian_platform_core::CpuCapabilities;
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
/// bundled as one independently-lockable unit inside [`RuntimeState`] so
/// a [`Stage`] that only touches physics never blocks on
/// [`AudioSubsystem`]'s or `World`'s lock. `engine-core` still owns the
/// actual stepping *logic* here (rule 7: this is where cross-`*-core`
/// domain logic lives, not a downstream orchestration crate re-deriving
/// it) — only *when* it runs is `Stage`/`Runtime`'s concern.
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
    /// effect chain — rather than fixing it. [`Stage`] is the actual,
    /// now-real fix (an app-defined `Stage` can own a `BinauralRenderer`
    /// directly); `examples/magic_figures` still doesn't route through
    /// one, though — see this module's own top-level doc for why its
    /// specific inputs (orbiting shapes' positions, the free-fly
    /// camera's pose) don't reduce to a `StageContext`-shaped `Stage`
    /// cleanly, not a gap in the mechanism itself.
    pub fn mix(&self) -> Vec<(meridian_audio_core::Channel, f32)> {
        self.mixer.mix(&self.listener, &self.emitters)
    }
}

/// Shared per-frame state, split into independently-lockable pieces —
/// see the module doc's "Locking" section for why this shape and not one
/// combined lock. `World` is available for application-level entity/
/// `Transform` use; not synced with `physics`'s bodies — no such mapping
/// is defined anywhere in the workspace yet, and inventing one here
/// would be new, undocumented design, not wiring together what already
/// exists.
#[derive(Clone)]
pub struct RuntimeState {
    world: Arc<Mutex<World>>,
    physics: Arc<Mutex<PhysicsSubsystem>>,
    audio: Arc<Mutex<AudioSubsystem>>,
}

impl RuntimeState {
    pub fn new(physics: PhysicsSubsystem, audio: AudioSubsystem) -> Self {
        Self {
            world: Arc::new(Mutex::new(World::new())),
            physics: Arc::new(Mutex::new(physics)),
            audio: Arc::new(Mutex::new(audio)),
        }
    }

    /// A [`StageContext`] holding clones of this state's locks — cheap
    /// (`Arc::clone`), one made fresh per stage per [`Runtime::tick`]
    /// call.
    fn context(&self) -> StageContext {
        StageContext {
            world: self.world.clone(),
            physics: self.physics.clone(),
            audio: self.audio.clone(),
        }
    }

    /// Locks the world directly — for application code that isn't
    /// itself a [`Stage`], between [`Runtime::tick`] calls. No stage
    /// runs concurrently with application code between ticks, so
    /// there's no lock-order-inversion risk to guard against here the
    /// way [`StageContext`] does mid-tick — this is a plain, single
    /// lock.
    pub fn world(&self) -> MutexGuard<'_, World> {
        self.world.lock().unwrap()
    }

    /// Locks the physics state directly — see [`world`](Self::world)'s
    /// doc comment for the same "between ticks, not mid-tick" scoping.
    pub fn physics(&self) -> MutexGuard<'_, PhysicsSubsystem> {
        self.physics.lock().unwrap()
    }

    /// Locks the audio state directly — see [`world`](Self::world)'s
    /// doc comment for the same "between ticks, not mid-tick" scoping.
    pub fn audio(&self) -> MutexGuard<'_, AudioSubsystem> {
        self.audio.lock().unwrap()
    }
}

/// What a running [`Stage`] sees — locked access to [`RuntimeState`]'s
/// pieces, shaped so a lock-order inversion can't be expressed (see the
/// module doc's "Locking" section).
pub struct StageContext {
    world: Arc<Mutex<World>>,
    physics: Arc<Mutex<PhysicsSubsystem>>,
    audio: Arc<Mutex<AudioSubsystem>>,
}

impl StageContext {
    /// Locks only the world state.
    pub fn world(&self) -> MutexGuard<'_, World> {
        self.world.lock().unwrap()
    }

    /// Locks only the physics state. Blocks if another stage currently
    /// holds it (either a genuinely concurrent independent stage, or —
    /// if this stage depends on that one — this call simply never
    /// contends, since the dependency already guaranteed the earlier
    /// stage finished and dropped its guard first).
    pub fn physics(&self) -> MutexGuard<'_, PhysicsSubsystem> {
        self.physics.lock().unwrap()
    }

    /// Locks only the audio state.
    pub fn audio(&self) -> MutexGuard<'_, AudioSubsystem> {
        self.audio.lock().unwrap()
    }

    /// Locks both physics and audio, always physics-then-audio
    /// internally — one of the only two ways this module lets a stage
    /// hold more than one lock at once, precisely so no caller can
    /// independently choose (and accidentally invert) the acquisition
    /// order.
    pub fn physics_and_audio(
        &self,
    ) -> (
        MutexGuard<'_, PhysicsSubsystem>,
        MutexGuard<'_, AudioSubsystem>,
    ) {
        let physics = self.physics.lock().unwrap();
        let audio = self.audio.lock().unwrap();
        (physics, audio)
    }

    /// Locks all three, always world-then-physics-then-audio internally
    /// — the other of the only two multi-lock accessors this module
    /// exposes, same reasoning as
    /// [`physics_and_audio`](Self::physics_and_audio).
    pub fn world_physics_and_audio(
        &self,
    ) -> (
        MutexGuard<'_, World>,
        MutexGuard<'_, PhysicsSubsystem>,
        MutexGuard<'_, AudioSubsystem>,
    ) {
        let world = self.world.lock().unwrap();
        let physics = self.physics.lock().unwrap();
        let audio = self.audio.lock().unwrap();
        (world, physics, audio)
    }
}

/// One unit of per-frame work. Implementations are free to do anything —
/// step physics, mix audio, dispatch a compute kernel, present a
/// rendered frame, run gameplay logic — this crate has no opinion; see
/// the module doc for why that's the point. `run` takes `&mut self` so a
/// stage can hold its own private state across frames (e.g. a
/// `BinauralRenderer`'s internal delay lines, or a `ComputeStage`'s
/// `ComputeScheduler`, which must persist call-to-call).
pub trait Stage: Send {
    fn run(&mut self, ctx: &StageContext);

    /// Called by [`Runtime::resize_all`] when the application's window
    /// changes size. Default no-op — most stages (physics, compute) have
    /// no notion of size; a rendering stage (owning a swapchain-backed
    /// target) overrides this to rebuild its size-dependent resources.
    /// Generalizing resize to *every* stage, rather than a
    /// rendering-specific side channel, is what lets a render-presenting
    /// `Stage` (see `meridian-sdk`'s own module doc) live in the same
    /// registry as every other stage instead of needing special-cased
    /// access from application code.
    fn resize(&mut self, _width: u32, _height: u32) {}
}

/// Opaque handle to a stage registered with a [`Runtime`] — used to
/// declare it as another stage's dependency. Not valid across different
/// `Runtime`s, the same restriction `meridian_task_core::JobId` has and
/// for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageId(usize);

struct StageEntry {
    name: &'static str,
    deps: Vec<StageId>,
    stage: Arc<Mutex<dyn Stage>>,
}

/// Owns [`RuntimeState`] and a set of registered [`Stage`]s, and runs
/// them in dependency order every [`Runtime::tick`] — see the module doc
/// for the `JobGraph`-based mechanism and the locking guarantee. The
/// single entry point for a frame's work in this workspace — see the
/// module doc's "one mechanism, not two" section.
pub struct Runtime {
    state: RuntimeState,
    scheduler: FrameScheduler,
    stages: Vec<StageEntry>,
}

impl Runtime {
    /// Builds a `Runtime` with no stages registered yet (see
    /// [`add_stage`](Self::add_stage)) and a [`FrameScheduler`] sized to
    /// the real detected CPU thread count. Use
    /// [`with_worker_count`](Self::with_worker_count) to size it
    /// explicitly instead (e.g. a fixed worker count for reproducible
    /// benchmarks).
    pub fn new(physics: PhysicsSubsystem, audio: AudioSubsystem) -> Self {
        Self::with_worker_count(physics, audio, CpuCapabilities::detect().threads)
    }

    pub fn with_worker_count(
        physics: PhysicsSubsystem,
        audio: AudioSubsystem,
        worker_count: usize,
    ) -> Self {
        meridian_foundation::log_info!("engine-core Runtime initialized");
        Self {
            state: RuntimeState::new(physics, audio),
            scheduler: FrameScheduler::new(worker_count),
            stages: Vec::new(),
        }
    }

    /// The shared state this runtime's stages run against — clone it
    /// (cheap: `Arc` clones) to read/write it directly between ticks,
    /// e.g. to seed initial bodies or read back positions after
    /// [`tick`](Self::tick) returns.
    pub fn state(&self) -> &RuntimeState {
        &self.state
    }

    /// Registers `stage`, runnable only once every stage in `deps` has
    /// completed this tick — the same contract
    /// `meridian_task_core::JobGraph::add_job` documents, one layer
    /// down. `deps` must be [`StageId`]s this same `Runtime` already
    /// returned (stages are registered in dependency order, same as
    /// `JobGraph` itself).
    pub fn add_stage(
        &mut self,
        name: &'static str,
        deps: &[StageId],
        stage: impl Stage + 'static,
    ) -> StageId {
        let id = StageId(self.stages.len());
        self.stages.push(StageEntry {
            name,
            deps: deps.to_vec(),
            stage: Arc::new(Mutex::new(stage)),
        });
        id
    }

    /// Runs every registered stage once, in dependency order,
    /// independent branches in parallel — rebuilds a fresh `JobGraph`
    /// every call (matching `JobGraph`'s own "a dependency-ordered set
    /// of jobs for one frame" contract — its job actions are `FnOnce`,
    /// consumed once by `Scheduler::run`, not meant to be reused across
    /// frames). Blocks until every stage has finished.
    pub fn tick(&self) {
        let all_ids: Vec<StageId> = (0..self.stages.len()).map(StageId).collect();
        self.tick_only(&all_ids);
    }

    /// Like [`tick`](Self::tick), but runs only the stages named in
    /// `ids` (each still gated on any of *its own* dependencies that are
    /// also in `ids` — a dependency left out of `ids` is treated as
    /// already satisfied, not an error, since the whole point of this
    /// method is running a subset on its own schedule). The concrete
    /// reason this exists rather than every application always calling
    /// plain [`tick`](Self::tick): physics and rendering genuinely run
    /// at different multiplicities per real display frame — a
    /// fixed-timestep physics accumulator calls its stage 0-8 times to
    /// catch up on one real frame's elapsed time, while a rendering
    /// stage must run exactly once per display frame (a
    /// `Runtime::tick`-per-catch-up-step render would present several
    /// swapchain frames per real frame, wrong and wasteful). Both still
    /// go through this same `Runtime` and the same dependency-graph
    /// mechanism — `tick_only(&[physics_id])` in the accumulator loop,
    /// `tick_only(&[render_id])` once after it, rather than either stage
    /// being invoked outside `Runtime` entirely.
    pub fn tick_only(&self, ids: &[StageId]) {
        let selected: std::collections::HashSet<usize> = ids.iter().map(|id| id.0).collect();
        let mut graph = JobGraph::new();
        let mut job_ids: Vec<Option<meridian_task_core::JobId>> = vec![None; self.stages.len()];

        for &id in ids {
            let entry = &self.stages[id.0];
            let dep_job_ids: Vec<_> = entry
                .deps
                .iter()
                .filter(|dep| selected.contains(&dep.0))
                .map(|dep| {
                    job_ids[dep.0]
                        .expect("tick_only: a dependency must appear before its dependent in `ids`")
                })
                .collect();
            let stage = entry.stage.clone();
            let ctx = self.state.context();
            let job_id = graph.add_job(entry.name, &dep_job_ids, move || {
                stage.lock().unwrap().run(&ctx);
            });
            job_ids[id.0] = Some(job_id);
        }

        self.scheduler.run(graph);
    }

    /// Calls [`Stage::resize`] on every registered stage, in
    /// registration order (no dependency graph needed here — resize
    /// isn't a per-frame data-flow operation, and stages don't depend on
    /// each other's resize side effects the way [`tick`](Self::tick)'s
    /// stages depend on each other's *data*). Call this from an
    /// application's window-resize handler.
    pub fn resize_all(&self, width: u32, height: u32) {
        for entry in &self.stages {
            entry.stage.lock().unwrap().resize(width, height);
        }
    }
}

/// Built-in convenience stage wrapping [`PhysicsSubsystem::step`] — a
/// thin forward, not a reimplementation (see this module's own doc for
/// why that distinction matters). Most applications that want physics
/// in their runtime can register this directly instead of writing their
/// own [`Stage`] impl for it.
pub struct PhysicsStepStage {
    pub dt: f32,
}

impl PhysicsStepStage {
    pub fn new(dt: f32) -> Self {
        Self { dt }
    }
}

impl Stage for PhysicsStepStage {
    fn run(&mut self, ctx: &StageContext) {
        ctx.physics().step(self.dt);
    }
}

/// Built-in convenience stage wrapping an arbitrary
/// `compute-runtime::ComputeKernel` — the concrete answer to "how does a
/// game hand the runtime its own GPU/CPU compute work," registered the
/// exact same way as any other [`Stage`]: dependency-ordered against
/// physics, audio, or other compute stages, with `StageContext` giving
/// it nothing it doesn't ask for (a kernel that needs `physics`/`audio`
/// state locks them itself inside `run`, same as any other `Stage`). Not
/// specific to any domain — `gac-compute`/`physics-compute`'s own
/// kernels work here exactly as well as a game-specific one; this crate
/// still doesn't know what any given kernel actually computes.
pub struct ComputeStage<K> {
    scheduler: ComputeScheduler,
    kernel: K,
    size: DispatchSize,
}

impl<K: ComputeKernel + Send + 'static> ComputeStage<K> {
    /// `size` is the dispatch shape passed to `ComputeKernel::dispatch`
    /// every tick — most kernels use `DispatchSize::linear(item_count)`
    /// (see that constructor's own doc comment).
    pub fn new(kernel: K, size: DispatchSize) -> Self {
        Self {
            scheduler: ComputeScheduler::new(),
            kernel,
            size,
        }
    }
}

impl<K: ComputeKernel + Send> Stage for ComputeStage<K> {
    fn run(&mut self, _ctx: &StageContext) {
        self.scheduler.run(&self.kernel, self.size);
    }
}

/// [`PhysicsStepStage`]'s batched-dispatch counterpart: steps physics by
/// running `meridian-physics-compute`'s
/// [`RigidBodyIntegratorKernel`]/[`BroadPhasePairsKernel`]/
/// [`GenerateContactsKernel`]/[`ConstraintSolverBatchKernel`] instead of
/// [`PhysicsSubsystem::step`]'s plain sequential `for` loop — the
/// concrete answer to docs/roadmap.md's "physics needs to scale past a
/// handful of bodies" entry. Registered exactly like
/// [`PhysicsStepStage`] (same `dt`-per-tick shape, same
/// `ctx.physics()` lock); swap one for the other without touching
/// anything else about how an application composes its `Runtime`.
///
/// **Same algorithm, different dispatch shape — not a semantic
/// rewrite.** This mirrors `PhysicsSubsystem::step`'s exact structure
/// (integrate, `RELAXATION_ITERATIONS` velocity-only passes, one final
/// positional-correction pass) with one deliberate difference already
/// proven equivalent by `physics-compute`'s own tests: broad/narrow
/// phase run *once* per tick here, not once per relaxation pass (see
/// `ConstraintSolverBatchKernel`'s own module doc for why that's
/// harmless — `resolve_velocity` never changes a body's `frame`, only
/// its velocity, so the contact set genuinely can't change between
/// velocity-only passes; `PhysicsSubsystem::step`'s repeated
/// recomputation there is wasteful, not more correct).
///
/// **"Batched," not (yet) literally GPU-executing.** Every kernel here
/// dispatches through `ComputeContext::parallel_for`'s CPU backend
/// today (real multi-core parallelism, not single-threaded) — none of
/// the five has a WGSL shader behind it the way
/// `meridian-physics-compute`'s own `float`/`fixed` soft-body kernels
/// do. The architecture (one `ComputeKernel` per pipeline stage,
/// dispatched through `compute-runtime`) is the same shape a real GPU
/// path would slot into; that GPU path itself is still a real, disclosed
/// gap, not silently implied by this type's name.
pub struct PhysicsComputeStepStage {
    pub dt: f32,
    context: ComputeContext,
}

impl PhysicsComputeStepStage {
    pub fn new(dt: f32) -> Self {
        Self {
            dt,
            context: ComputeContext::new(),
        }
    }
}

impl Stage for PhysicsComputeStepStage {
    fn run(&mut self, ctx: &StageContext) {
        let mut physics = ctx.physics();
        let body_count = physics.bodies.len();

        let integrator_kernel =
            RigidBodyIntegratorKernel::new(physics.integrator, physics.bodies.clone(), self.dt);
        integrator_kernel.dispatch(&self.context, DispatchSize::linear(body_count as u32));
        let integrated = integrator_kernel.results();

        // Upper bounds, not exact counts — every kernel here clamps its
        // own dispatch count to its real candidate/pair list length
        // internally (see each kernel's own `dispatch` — `size.total()
        // .min(self.\u{2026}.len())`), so an over-generous `DispatchSize` is
        // always safe, just occasionally over-allocates a `parallel_for`
        // call that immediately no-ops past the real count.
        let pair_upper_bound = (body_count.saturating_mul(body_count.max(1))) as u32;
        let broad_kernel = BroadPhasePairsKernel::new(integrated.clone());
        broad_kernel.dispatch(&self.context, DispatchSize::linear(pair_upper_bound));
        let pairs = broad_kernel.pairs();

        let contacts_kernel = GenerateContactsKernel::new(integrated, pairs);
        contacts_kernel.dispatch(&self.context, DispatchSize::linear(pair_upper_bound));
        let contacts = contacts_kernel.results();

        let solver_kernel = ConstraintSolverBatchKernel::new(
            physics.solver,
            contacts_kernel.bodies.clone(),
            contacts,
        );
        solver_kernel.step(&self.context, RELAXATION_ITERATIONS);

        physics.bodies = solver_kernel.bodies();
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

    fn subsystem_pair() -> (PhysicsSubsystem, AudioSubsystem) {
        (
            PhysicsSubsystem::default(),
            AudioSubsystem::new(Mixer::new(SpeakerLayout::mono())),
        )
    }

    #[test]
    fn audio_subsystem_mixes_audio_from_current_emitter_positions() {
        let mut audio = AudioSubsystem::new(
            Mixer::new(SpeakerLayout::stereo_headphones()).with_attenuation(AttenuationModel {
                reference_distance: 1000.0,
                rolloff: 1.0,
                max_distance: 1000.0,
            }),
        );
        audio.listener = Listener {
            frame: Motor3::identity(),
        };
        // Local +Z is "right" per audio-core's listener convention.
        audio.emitters.push((
            Emitter {
                frame: Motor3::translation(Vec3::new(0.0, 0.0, 5.0)),
            },
            1.0,
        ));

        let gains = audio.mix();
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
    fn physics_subsystem_step_resolves_a_resting_contact() {
        let mut physics = PhysicsSubsystem::default();
        physics.bodies.push(RigidBody {
            frame: Motor3::translation(Vec3::new(0.0, -50.0, 0.0)),
            mass: 0.0, // static floor
            shape: ColliderShape::Sphere { radius: 50.0 },
            ..Default::default()
        });
        physics.bodies.push(falling_body());

        for _ in 0..600 {
            physics.step(1.0 / 60.0);
        }

        let resting_height = physics.bodies[1].position().y;
        assert!(
            (resting_height - 0.5).abs() < 0.5,
            "ball should settle near the floor surface, got y={resting_height}"
        );
    }

    /// A box-on-box manifold (up to 4 contact points, unlike the
    /// single-point sphere case above) — regression coverage for the bug
    /// `step`'s doc comment describes: the original one-`resolve()`-
    /// call-per-contact shape over-applied positional correction on
    /// every relaxation-worthy contact set, bouncing a settled box
    /// up/down and eventually clipping it through the floor. A sphere
    /// never exercised this (always exactly one contact point), which is
    /// exactly why the bug went unnoticed in this method even after
    /// `examples/physic_figures` independently found and fixed it in its
    /// own hand-rolled stepping — see `meridian-physics-core::float`'s
    /// own `cuboid_settles_without_runaway_spin` test for the
    /// equivalent, non-centralized version of this same assertion.
    #[test]
    fn physics_subsystem_step_settles_a_box_without_bouncing_or_sinking() {
        use meridian_physics_core::ConstraintSolver;

        let mut physics = PhysicsSubsystem {
            solver: ConstraintSolver::new(0.0).with_friction(0.6),
            ..Default::default()
        };
        physics.bodies.push(RigidBody {
            frame: Motor3::translation(Vec3::new(0.0, -0.5, 0.0)),
            mass: 0.0, // static floor
            shape: ColliderShape::Cuboid {
                half_extents: Vec3::new(14.0, 0.5, 14.0),
            },
            ..Default::default()
        });
        physics.bodies.push(RigidBody {
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
            physics.step(1.0 / 60.0);
            if step > 200 {
                let height = physics.bodies[1].position().y;
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

    /// A single [`PhysicsStepStage`] run through [`Runtime::tick`] must
    /// match calling `PhysicsSubsystem::step` directly — the runtime
    /// mechanism must not change what a stage does, only when it runs.
    #[test]
    fn physics_step_stage_matches_direct_step_call() {
        let (mut physics, audio) = subsystem_pair();
        physics.bodies.push(falling_body());
        let mut runtime = Runtime::with_worker_count(physics, audio, 2);
        runtime.add_stage("physics", &[], PhysicsStepStage::new(1.0 / 60.0));
        let state = runtime.state().clone();

        runtime.tick();

        let velocity_after_tick = state.physics().bodies[0].velocity;

        let mut expected = PhysicsSubsystem::default();
        expected.bodies.push(falling_body());
        expected.step(1.0 / 60.0);

        assert_eq!(velocity_after_tick, expected.bodies[0].velocity);
    }

    /// A stage that depends on another must see its finished output —
    /// the whole point of declaring the dependency, not an accident of
    /// scheduling order.
    #[test]
    fn dependent_stage_sees_upstream_stages_finished_work() {
        struct SetHeight(f32);
        impl Stage for SetHeight {
            fn run(&mut self, ctx: &StageContext) {
                ctx.physics().bodies[0].frame = Motor3::translation(Vec3::new(0.0, self.0, 0.0));
            }
        }
        struct ReadHeightInto(Arc<Mutex<f32>>);
        impl Stage for ReadHeightInto {
            fn run(&mut self, ctx: &StageContext) {
                let height = ctx.physics().bodies[0].position().y;
                *self.0.lock().unwrap() = height;
            }
        }

        let (mut physics, audio) = subsystem_pair();
        physics.bodies.push(falling_body());
        let mut runtime = Runtime::with_worker_count(physics, audio, 2);

        let observed = Arc::new(Mutex::new(0.0f32));
        let set = runtime.add_stage("set", &[], SetHeight(42.0));
        runtime.add_stage("read", &[set], ReadHeightInto(observed.clone()));

        runtime.tick();

        assert_eq!(*observed.lock().unwrap(), 42.0);
    }

    /// Two stages with no dependency between them, each locking a
    /// *different* resource (physics vs. audio), must both run to
    /// completion without deadlocking or blocking on each other's lock —
    /// this is the actual guarantee fine-grained locking exists for.
    /// Run many times in one process (repeated ticks below) to give a
    /// real deadlock a chance to manifest rather than pass by luck once.
    #[test]
    fn independent_stages_on_different_resources_never_deadlock() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct TouchPhysics(Arc<AtomicUsize>);
        impl Stage for TouchPhysics {
            fn run(&mut self, ctx: &StageContext) {
                let _guard = ctx.physics();
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        struct TouchAudio(Arc<AtomicUsize>);
        impl Stage for TouchAudio {
            fn run(&mut self, ctx: &StageContext) {
                let _guard = ctx.audio();
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let (physics, audio) = subsystem_pair();
        let physics_runs = Arc::new(AtomicUsize::new(0));
        let audio_runs = Arc::new(AtomicUsize::new(0));

        let mut runtime = Runtime::with_worker_count(physics, audio, 4);
        runtime.add_stage("physics", &[], TouchPhysics(physics_runs.clone()));
        runtime.add_stage("audio", &[], TouchAudio(audio_runs.clone()));

        for _ in 0..50 {
            runtime.tick();
        }

        assert_eq!(physics_runs.load(Ordering::SeqCst), 50);
        assert_eq!(audio_runs.load(Ordering::SeqCst), 50);
    }

    /// [`StageContext::physics_and_audio`] must observe the same values
    /// either single accessor would — the combined lock is a real join,
    /// not two independent, possibly-inconsistent reads.
    #[test]
    fn physics_and_audio_combined_lock_sees_both_pieces_of_state() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct ReadBoth {
            body_count: Arc<AtomicUsize>,
            emitter_count: Arc<AtomicUsize>,
        }
        impl Stage for ReadBoth {
            fn run(&mut self, ctx: &StageContext) {
                let (physics, audio) = ctx.physics_and_audio();
                self.body_count
                    .store(physics.bodies.len(), Ordering::SeqCst);
                self.emitter_count
                    .store(audio.emitters.len(), Ordering::SeqCst);
            }
        }

        let (mut physics, mut audio) = subsystem_pair();
        physics.bodies.push(falling_body());
        physics.bodies.push(falling_body());
        audio.emitters.push((
            Emitter {
                frame: Motor3::identity(),
            },
            1.0,
        ));

        let mut runtime = Runtime::with_worker_count(physics, audio, 2);
        let body_count = Arc::new(AtomicUsize::new(0));
        let emitter_count = Arc::new(AtomicUsize::new(0));
        runtime.add_stage(
            "read",
            &[],
            ReadBoth {
                body_count: body_count.clone(),
                emitter_count: emitter_count.clone(),
            },
        );

        runtime.tick();

        assert_eq!(body_count.load(Ordering::SeqCst), 2);
        assert_eq!(emitter_count.load(Ordering::SeqCst), 1);
    }

    /// [`StageContext::world_physics_and_audio`] must lock and expose
    /// all three pieces together — the third-lock counterpart to
    /// [`physics_and_audio_combined_lock_sees_both_pieces_of_state`].
    #[test]
    fn world_physics_and_audio_combined_lock_sees_all_three_pieces_of_state() {
        struct ReadWorldPhysicsAudio(Arc<Mutex<(usize, usize)>>);
        impl Stage for ReadWorldPhysicsAudio {
            fn run(&mut self, ctx: &StageContext) {
                let (_world, physics, audio) = ctx.world_physics_and_audio();
                *self.0.lock().unwrap() = (physics.bodies.len(), audio.emitters.len());
            }
        }

        let (mut physics, mut audio) = subsystem_pair();
        physics.bodies.push(falling_body());
        audio.emitters.push((
            Emitter {
                frame: Motor3::identity(),
            },
            1.0,
        ));

        let mut runtime = Runtime::with_worker_count(physics, audio, 2);
        let observed = Arc::new(Mutex::new((0, 0)));
        runtime.add_stage("read", &[], ReadWorldPhysicsAudio(observed.clone()));

        runtime.tick();

        assert_eq!(*observed.lock().unwrap(), (1, 1));
    }

    /// A [`ComputeStage`] wrapping a trivial `ComputeKernel` must
    /// actually dispatch it through `Runtime::tick`, the same as any
    /// other stage — proving the generic compute-kernel extension point
    /// (not just physics/audio) is real, not just documented.
    #[test]
    fn compute_stage_dispatches_its_kernel_through_tick() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        use meridian_compute_runtime::{ComputeContext, ComputeKernel};

        struct CountingKernel(Arc<AtomicUsize>);
        impl ComputeKernel for CountingKernel {
            fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
                context.parallel_for(size.x as usize, |_| {
                    self.0.fetch_add(1, Ordering::SeqCst);
                });
            }
        }

        let (physics, audio) = subsystem_pair();
        let mut runtime = Runtime::with_worker_count(physics, audio, 2);
        let dispatch_count = Arc::new(AtomicUsize::new(0));
        runtime.add_stage(
            "compute",
            &[],
            ComputeStage::new(
                CountingKernel(dispatch_count.clone()),
                DispatchSize { x: 10, y: 1, z: 1 },
            ),
        );

        runtime.tick();

        assert_eq!(dispatch_count.load(Ordering::SeqCst), 10);
    }

    /// `tick_only` must run *only* the named stages — the physics-many-
    /// times/render-once split its own doc comment describes. Two
    /// stages registered, only one selected repeatedly: the unselected
    /// one must never run.
    #[test]
    fn tick_only_runs_exactly_the_selected_stages() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct Counter(Arc<AtomicUsize>);
        impl Stage for Counter {
            fn run(&mut self, _ctx: &StageContext) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let (physics, audio) = subsystem_pair();
        let mut runtime = Runtime::with_worker_count(physics, audio, 2);
        let physics_runs = Arc::new(AtomicUsize::new(0));
        let render_runs = Arc::new(AtomicUsize::new(0));
        let physics_id = runtime.add_stage("physics", &[], Counter(physics_runs.clone()));
        let render_id = runtime.add_stage("render", &[physics_id], Counter(render_runs.clone()));

        // Simulate an accumulator catching up 3 physics steps, then one
        // render.
        runtime.tick_only(&[physics_id]);
        runtime.tick_only(&[physics_id]);
        runtime.tick_only(&[physics_id]);
        runtime.tick_only(&[render_id]);

        assert_eq!(physics_runs.load(Ordering::SeqCst), 3);
        assert_eq!(render_runs.load(Ordering::SeqCst), 1);
    }

    /// `PhysicsComputeStepStage` must settle a box exactly the way
    /// `PhysicsStepStage`/`PhysicsSubsystem::step` do — same test shape
    /// as `physics_subsystem_step_settles_a_box_without_bouncing_or_sinking`,
    /// run through the batched-dispatch stage instead of the plain
    /// sequential loop, proving the swap is behavior-preserving, not
    /// just "compiles and looks physically plausible."
    #[test]
    fn physics_compute_step_stage_settles_a_box_without_bouncing_or_sinking() {
        let mut physics = PhysicsSubsystem {
            solver: ConstraintSolver::new(0.0).with_friction(0.6),
            ..Default::default()
        };
        physics.bodies.push(RigidBody {
            frame: Motor3::translation(Vec3::new(0.0, -0.5, 0.0)),
            mass: 0.0, // static floor
            shape: ColliderShape::Cuboid {
                half_extents: Vec3::new(14.0, 0.5, 14.0),
            },
            ..Default::default()
        });
        physics.bodies.push(RigidBody {
            frame: Motor3::translation(Vec3::new(0.0, 3.0, 0.0)),
            mass: 1.0,
            shape: ColliderShape::Cuboid {
                half_extents: Vec3::new(0.6, 0.6, 0.6),
            },
            ..Default::default()
        });

        let audio = AudioSubsystem::new(Mixer::new(SpeakerLayout::mono()));
        let mut runtime = Runtime::with_worker_count(physics, audio, 2);
        let physics_id =
            runtime.add_stage("physics", &[], PhysicsComputeStepStage::new(1.0 / 60.0));

        let mut min_height_after_landing = f32::MAX;
        let mut max_height_after_landing = f32::MIN;
        for step in 0..600 {
            runtime.tick_only(&[physics_id]);
            if step > 200 {
                let height = runtime.state().physics().bodies[1].position().y;
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

    /// One tick of `PhysicsComputeStepStage` must match one tick of
    /// `PhysicsStepStage` bit-for-bit on a simple falling body — the
    /// direct "same algorithm, different dispatch" proof, independent
    /// of the longer settling regression above.
    #[test]
    fn physics_compute_step_stage_matches_physics_step_stage_for_one_tick() {
        let (physics_a, audio_a) = subsystem_pair();
        let mut physics_a = physics_a;
        physics_a.bodies.push(falling_body());
        let mut runtime_a = Runtime::with_worker_count(physics_a, audio_a, 2);
        let stage_a = runtime_a.add_stage("physics", &[], PhysicsStepStage::new(1.0 / 60.0));
        runtime_a.tick_only(&[stage_a]);

        let (physics_b, audio_b) = subsystem_pair();
        let mut physics_b = physics_b;
        physics_b.bodies.push(falling_body());
        let mut runtime_b = Runtime::with_worker_count(physics_b, audio_b, 2);
        let stage_b = runtime_b.add_stage("physics", &[], PhysicsComputeStepStage::new(1.0 / 60.0));
        runtime_b.tick_only(&[stage_b]);

        assert_eq!(
            runtime_a.state().physics().bodies[0].frame,
            runtime_b.state().physics().bodies[0].frame
        );
        assert_eq!(
            runtime_a.state().physics().bodies[0].velocity,
            runtime_b.state().physics().bodies[0].velocity
        );
    }
}
