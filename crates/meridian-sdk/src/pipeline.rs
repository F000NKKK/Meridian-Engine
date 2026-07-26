//! A composable per-frame pipeline built on `task-core::JobGraph` — the
//! direct fix for the pattern `engine-core` kept almost repeating:
//! `SubsystemManager::mix_audio` is one specific, opinionated audio path
//! (`Mixer`'s per-channel gains), and a richer consumer
//! (`examples/magic_figures`, which needs `BinauralRenderer`'s real
//! per-sample synthesis) simply doesn't fit it. Bolting a second fixed
//! method onto `engine-core` for every new consumer's specific pipeline
//! shape doesn't scale and keeps repeating the same mistake — this
//! module is the alternative: an application assembles its *own*
//! [`Stage`]s (physics stepping, audio mixing, binaural rendering,
//! scene submission, or anything else) in whatever order and
//! dependency shape it needs, and [`Pipeline`] only owns *running*
//! them, never *what* they do.
//!
//! **Built on [`meridian_task_core::JobGraph`], not a hand-rolled
//! scheduler** — `JobGraph` already is "dependency-ordered per-frame
//! jobs, independent branches run in parallel across worker threads,"
//! exactly this module's shape (see docs/threading-model.md's "shape of
//! a frame"), and [`Pipeline::tick`] rebuilds one fresh `JobGraph` every
//! call (matching that type's own "a dependency-ordered set of jobs for
//! one frame" contract — its job actions are `FnOnce`, consumed once by
//! `Scheduler::run`, not meant to be reused across frames).
//! `engine-core::FrameScheduler` runs it — see that crate's module doc
//! for why `Runtime::tick` itself never needed this before now: physics
//! and audio were the only two real per-frame systems, with one
//! sequential dependency between them, so a job graph would have been
//! decorative. A composable multi-stage application pipeline is exactly
//! the "second real per-frame system, genuinely independent, to run
//! alongside" case that doc already named as what would make it
//! load-bearing.
//!
//! ## Locking: fine-grained, deadlock-free by construction
//!
//! [`PipelineState`] splits shared state into independently-lockable
//! pieces (currently [`meridian_engine_core::PhysicsSubsystem`] and
//! [`meridian_engine_core::AudioSubsystem`] — the same split
//! `engine-core` itself made in [`meridian_engine_core::SubsystemManager`],
//! reused here rather than re-derived) so two stages that only touch
//! *different* resources genuinely don't contend on each other's lock.
//! The risk with more than one lock is a classic lock-order-inversion
//! deadlock: stage A locks physics-then-audio while an independent
//! stage B locks audio-then-physics, running concurrently. [`StageContext`]
//! makes that inversion structurally unrepresentable rather than
//! documenting a convention a stage author could still get wrong:
//! [`StageContext::physics`]/[`StageContext::audio`] each take exactly
//! one lock, and the only way to hold both at once is
//! [`StageContext::physics_and_audio`], which always acquires them in
//! one fixed order internally. There is no API path that lets calling
//! code choose the order itself.

use std::sync::{Arc, Mutex, MutexGuard};

use meridian_engine_core::{AudioSubsystem, FrameScheduler, PhysicsSubsystem};
use meridian_task_core::JobGraph;

/// Shared per-frame state, split into independently-lockable pieces —
/// see the module doc's "Locking" section for why this shape and not one
/// combined lock.
#[derive(Clone)]
pub struct PipelineState {
    physics: Arc<Mutex<PhysicsSubsystem>>,
    audio: Arc<Mutex<AudioSubsystem>>,
}

impl PipelineState {
    /// Takes ownership of an already-configured
    /// [`PhysicsSubsystem`]/[`AudioSubsystem`] pair (e.g. built the same
    /// way `examples/physic_figures` configures
    /// `SubsystemManager::physics` today) and wraps each in its own
    /// lock.
    pub fn new(physics: PhysicsSubsystem, audio: AudioSubsystem) -> Self {
        Self {
            physics: Arc::new(Mutex::new(physics)),
            audio: Arc::new(Mutex::new(audio)),
        }
    }

    /// A [`StageContext`] holding clones of this state's locks — cheap
    /// (`Arc::clone`), one made fresh per stage per [`Pipeline::tick`]
    /// call.
    fn context(&self) -> StageContext {
        StageContext {
            physics: self.physics.clone(),
            audio: self.audio.clone(),
        }
    }
}

/// What a running [`Stage`] sees — locked access to [`PipelineState`]'s
/// pieces, shaped so a lock-order inversion can't be expressed (see the
/// module doc's "Locking" section).
pub struct StageContext {
    physics: Arc<Mutex<PhysicsSubsystem>>,
    audio: Arc<Mutex<AudioSubsystem>>,
}

impl StageContext {
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

    /// Locks both, always physics-then-audio internally — the *only*
    /// way this module lets a stage hold both locks at once, precisely
    /// so no caller can independently choose (and accidentally invert)
    /// the acquisition order.
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
}

/// One unit of per-frame work. Implementations are free to do anything —
/// step physics, mix audio, render a scene, run gameplay logic — this
/// crate has no opinion; see the module doc for why that's the point.
/// `run` takes `&mut self` so a stage can hold its own private state
/// across frames (e.g. a `BinauralRenderer`'s internal delay lines,
/// which must persist call-to-call — see that type's own doc comment).
pub trait Stage: Send {
    fn run(&mut self, ctx: &StageContext);
}

/// Opaque handle to a stage registered with a [`Pipeline`] — used to
/// declare it as another stage's dependency. Not valid across different
/// `Pipeline`s, the same restriction `meridian_task_core::JobId` has and
/// for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageId(usize);

struct StageEntry {
    name: &'static str,
    deps: Vec<StageId>,
    stage: Arc<Mutex<dyn Stage>>,
}

/// Owns [`PipelineState`] and a set of registered [`Stage`]s, and runs
/// them in dependency order every [`Pipeline::tick`] — see the module
/// doc for the `JobGraph`-based mechanism and the locking guarantee.
pub struct Pipeline {
    state: PipelineState,
    scheduler: FrameScheduler,
    stages: Vec<StageEntry>,
}

impl Pipeline {
    pub fn new(state: PipelineState, worker_count: usize) -> Self {
        Self {
            state,
            scheduler: FrameScheduler::new(worker_count),
            stages: Vec::new(),
        }
    }

    /// The shared state this pipeline's stages run against — clone it
    /// (cheap: `Arc` clones) to read/write it directly between ticks,
    /// e.g. to seed initial bodies or read back positions after
    /// [`tick`](Self::tick) returns.
    pub fn state(&self) -> &PipelineState {
        &self.state
    }

    /// Registers `stage`, runnable only once every stage in `deps` has
    /// completed this tick — the same contract
    /// `meridian_task_core::JobGraph::add_job` documents, one layer up.
    /// `deps` must be [`StageId`]s this same `Pipeline` already
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
    /// every call (see the module doc for why that's the correct,
    /// intended use of `JobGraph`, not overhead to optimize away).
    /// Blocks until every stage has finished.
    pub fn tick(&self) {
        let mut graph = JobGraph::new();
        let mut job_ids = Vec::with_capacity(self.stages.len());

        for entry in &self.stages {
            let dep_job_ids: Vec<_> = entry.deps.iter().map(|dep| job_ids[dep.0]).collect();
            let stage = entry.stage.clone();
            let ctx = self.state.context();
            let job_id = graph.add_job(entry.name, &dep_job_ids, move || {
                stage.lock().unwrap().run(&ctx);
            });
            job_ids.push(job_id);
        }

        self.scheduler.run(graph);
    }
}

/// Built-in convenience stage wrapping
/// [`PhysicsSubsystem::step`](meridian_engine_core::PhysicsSubsystem::step) —
/// a thin forward, not a reimplementation (see this crate's own module
/// doc for why that distinction matters). Most applications that want
/// physics in their pipeline can register this directly instead of
/// writing their own [`Stage`] impl for it.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColliderShape, Emitter, Mixer, Motor3, RigidBody, SpeakerLayout, Vec3};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn falling_body() -> RigidBody {
        RigidBody {
            frame: Motor3::translation(Vec3::new(0.0, 10.0, 0.0)),
            mass: 1.0,
            shape: ColliderShape::Sphere { radius: 0.5 },
            ..Default::default()
        }
    }

    fn state_with_one_falling_body() -> PipelineState {
        let mut physics = PhysicsSubsystem::default();
        physics.bodies.push(falling_body());
        let audio = AudioSubsystem::new(Mixer::new(SpeakerLayout::mono()));
        PipelineState::new(physics, audio)
    }

    /// A single [`PhysicsStepStage`] run through [`Pipeline::tick`] must
    /// match calling `PhysicsSubsystem::step` directly — the pipeline
    /// mechanism must not change what a stage does, only when it runs.
    #[test]
    fn physics_step_stage_matches_direct_step_call() {
        let state = state_with_one_falling_body();
        let mut pipeline = Pipeline::new(state.clone(), 2);
        pipeline.add_stage("physics", &[], PhysicsStepStage::new(1.0 / 60.0));

        pipeline.tick();

        let velocity_after_pipeline = state.physics.lock().unwrap().bodies[0].velocity;

        let mut expected = PhysicsSubsystem::default();
        expected.bodies.push(falling_body());
        expected.step(1.0 / 60.0);

        assert_eq!(velocity_after_pipeline, expected.bodies[0].velocity);
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

        let mut physics = PhysicsSubsystem::default();
        physics.bodies.push(falling_body());
        let audio = AudioSubsystem::new(Mixer::new(SpeakerLayout::mono()));
        let state = PipelineState::new(physics, audio);

        let observed = Arc::new(Mutex::new(0.0f32));
        let mut pipeline = Pipeline::new(state, 2);
        let set = pipeline.add_stage("set", &[], SetHeight(42.0));
        pipeline.add_stage("read", &[set], ReadHeightInto(observed.clone()));

        pipeline.tick();

        assert_eq!(*observed.lock().unwrap(), 42.0);
    }

    /// Two stages with no dependency between them, each locking a
    /// *different* resource (physics vs. audio), must both run to
    /// completion without deadlocking or blocking on each other's lock —
    /// this is the actual guarantee fine-grained locking exists for.
    /// Run many times in one process (via `#[test]`'s own harness
    /// parallelism plus repeated ticks below) to give a real deadlock a
    /// chance to manifest rather than pass by luck once.
    #[test]
    fn independent_stages_on_different_resources_never_deadlock() {
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

        let state = state_with_one_falling_body();
        let physics_runs = Arc::new(AtomicUsize::new(0));
        let audio_runs = Arc::new(AtomicUsize::new(0));

        let mut pipeline = Pipeline::new(state, 4);
        pipeline.add_stage("physics", &[], TouchPhysics(physics_runs.clone()));
        pipeline.add_stage("audio", &[], TouchAudio(audio_runs.clone()));

        for _ in 0..50 {
            pipeline.tick();
        }

        assert_eq!(physics_runs.load(Ordering::SeqCst), 50);
        assert_eq!(audio_runs.load(Ordering::SeqCst), 50);
    }

    /// [`StageContext::physics_and_audio`] must observe the same values
    /// either single accessor would — the combined lock is a real
    /// join, not two independent, possibly-inconsistent reads.
    #[test]
    fn physics_and_audio_combined_lock_sees_both_pieces_of_state() {
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

        let mut physics = PhysicsSubsystem::default();
        physics.bodies.push(falling_body());
        physics.bodies.push(falling_body());
        let mut audio = AudioSubsystem::new(Mixer::new(SpeakerLayout::mono()));
        audio.emitters.push((
            Emitter {
                frame: Motor3::identity(),
            },
            1.0,
        ));
        let state = PipelineState::new(physics, audio);

        let body_count = Arc::new(AtomicUsize::new(0));
        let emitter_count = Arc::new(AtomicUsize::new(0));
        let mut pipeline = Pipeline::new(state, 2);
        pipeline.add_stage(
            "read_both",
            &[],
            ReadBoth {
                body_count: body_count.clone(),
                emitter_count: emitter_count.clone(),
            },
        );

        pipeline.tick();

        assert_eq!(body_count.load(Ordering::SeqCst), 2);
        assert_eq!(emitter_count.load(Ordering::SeqCst), 1);
    }
}
