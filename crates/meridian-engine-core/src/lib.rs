//! `Runtime`: the single entry point tying every other crate into the
//! main loop. Owns real instances of the driver-independent subsystems
//! that exist today — an `ecs-core` [`World`], [`PhysicsSubsystem`]
//! (`physics-core`'s body list, broad/narrow-phase, solver/integrator)
//! and [`AudioSubsystem`] (`audio-core`'s listener, emitters, mixer) —
//! this is still the one place in the workspace allowed to know about
//! every `*-core` at once (see docs/dependency-rules.md rule 7).
//!
//! Layer: application; bounded context: runtime (`craftbook.toml`,
//! docs/context-map.md).
//!
//! Design notes: docs/notes/meridian-engine-core.md (`lib`).

#![forbid(unsafe_code)]

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
#[derive(Debug, Clone)]
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
        self.world
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Locks the physics state directly — see [`world`](Self::world)'s
    /// doc comment for the same "between ticks, not mid-tick" scoping.
    pub fn physics(&self) -> MutexGuard<'_, PhysicsSubsystem> {
        self.physics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Locks the audio state directly — see [`world`](Self::world)'s
    /// doc comment for the same "between ticks, not mid-tick" scoping.
    pub fn audio(&self) -> MutexGuard<'_, AudioSubsystem> {
        self.audio
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// What a running [`Stage`] sees — locked access to [`RuntimeState`]'s
/// pieces, shaped so a lock-order inversion can't be expressed (see the
/// module doc's "Locking" section).
#[derive(Debug)]
pub struct StageContext {
    world: Arc<Mutex<World>>,
    physics: Arc<Mutex<PhysicsSubsystem>>,
    audio: Arc<Mutex<AudioSubsystem>>,
}

impl StageContext {
    /// Locks only the world state.
    pub fn world(&self) -> MutexGuard<'_, World> {
        self.world
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Locks only the physics state. Blocks if another stage currently
    /// holds it (either a genuinely concurrent independent stage, or —
    /// if this stage depends on that one — this call simply never
    /// contends, since the dependency already guaranteed the earlier
    /// stage finished and dropped its guard first).
    pub fn physics(&self) -> MutexGuard<'_, PhysicsSubsystem> {
        self.physics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Locks only the audio state.
    pub fn audio(&self) -> MutexGuard<'_, AudioSubsystem> {
        self.audio
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
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
        let physics = self
            .physics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let audio = self
            .audio
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
        let world = self
            .world
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let physics = self
            .physics
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let audio = self
            .audio
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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

impl std::fmt::Debug for StageEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StageEntry")
            .field("name", &self.name)
            .field("deps", &self.deps)
            .finish_non_exhaustive()
    }
}

/// Owns [`RuntimeState`] and a set of registered [`Stage`]s, and runs
/// them in dependency order every [`Runtime::tick`] — see the module doc
/// for the `JobGraph`-based mechanism and the locking guarantee. The
/// single entry point for a frame's work in this workspace — see the
/// module doc's "one mechanism, not two" section.
#[derive(Debug)]
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
                stage
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .run(&ctx);
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
            entry
                .stage
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .resize(width, height);
        }
    }
}

/// Selects [`PhysicsStepStage`]'s dispatch shape — the flag this crate
/// uses instead of two separate `Stage` types: one type switching on this
/// enum is the same "one mechanism, not two" reasoning this crate applies
/// to `Runtime` itself, at the stage level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhysicsDispatchMode {
    /// Plain sequential CPU loop: [`PhysicsSubsystem::step`] as-is.
    #[default]
    Sequential,
    /// `meridian-physics-compute`'s kernels, dispatched through
    /// `ComputeContext::parallel_for`'s CPU backend (real multi-core
    /// parallelism, not single-threaded, but not literally
    /// GPU-executing — see [`PhysicsStepStage`]'s doc for the disclosed
    /// gap between this and an eventual WGSL path).
    Batched,
}

/// Built-in convenience stage wrapping physics stepping — either
/// [`PhysicsSubsystem::step`]'s plain sequential loop, or
/// `meridian-physics-compute`'s batched kernel pipeline, selected via
/// [`PhysicsDispatchMode`] rather than two separate types. Most
/// applications that want physics in their runtime can register this
/// directly instead of writing their own [`Stage`] impl for it.
///
/// **Same algorithm, different dispatch shape — not a semantic
/// rewrite.** [`PhysicsDispatchMode::Batched`] mirrors
/// `PhysicsSubsystem::step`'s exact structure (integrate,
/// `RELAXATION_ITERATIONS` velocity-only passes, one final
/// positional-correction pass) with one deliberate difference already
/// proven equivalent by `physics-compute`'s own tests: broad/narrow
/// phase run *once* per tick, not once per relaxation pass (see
/// `ConstraintSolverBatchKernel`'s own module doc for why that's
/// harmless — `resolve_velocity` never changes a body's `frame`, only
/// its velocity, so the contact set genuinely can't change between
/// velocity-only passes; `PhysicsSubsystem::step`'s repeated
/// recomputation there is wasteful, not more correct).
///
/// **"Batched," not (yet) literally GPU-executing.** Every kernel in
/// `PhysicsDispatchMode::Batched` dispatches through
/// `ComputeContext::parallel_for`'s CPU backend today — none of the four
/// (integrator/broad-phase/narrow-phase/solver) has a WGSL shader behind
/// it the way `meridian-physics-compute`'s own `float`/`fixed` soft-body
/// kernels do. A real GPU broad-phase kernel exists in
/// `meridian-physics-compute` (`float::BroadPhaseGpuKernel`) as the
/// first concrete step down that path — not yet wired in here, since
/// the other three kernels (integrator's rotor exponential/motor
/// composition, narrow-phase's SAT/manifold clipping, the solver's
/// graph-colored Gauss-Seidel passes) would need their own WGSL first
/// for a `Gpu` dispatch mode to mean "the whole pipeline runs on the
/// GPU" rather than silently mixing GPU and CPU work under one name.
#[derive(Debug)]
pub struct PhysicsStepStage {
    pub dt: f32,
    pub mode: PhysicsDispatchMode,
    context: ComputeContext,
}

impl PhysicsStepStage {
    pub fn new(dt: f32, mode: PhysicsDispatchMode) -> Self {
        Self {
            dt,
            mode,
            context: ComputeContext::new(),
        }
    }

    fn run_batched(&self, ctx: &StageContext) {
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

impl Stage for PhysicsStepStage {
    fn run(&mut self, ctx: &StageContext) {
        match self.mode {
            PhysicsDispatchMode::Sequential => ctx.physics().step(self.dt),
            PhysicsDispatchMode::Batched => self.run_batched(ctx),
        }
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
#[derive(Debug)]
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

#[cfg(test)]
mod tests;
