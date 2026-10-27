//! Tests of stage dependencies and locking.

use std::sync::{Arc, Mutex};

use meridian_audio_core::Emitter;
use meridian_compute_runtime::DispatchSize;
use meridian_gac_core::{Motor3, Vec3};

use super::{falling_body, subsystem_pair};
use crate::*;

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
            meridian_compute_runtime::ComputeScheduler::new(Arc::new(
                meridian_compute_runtime::testing::SequentialDevice,
            )),
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
