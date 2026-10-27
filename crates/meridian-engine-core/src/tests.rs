//! Tests of this module.

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
    runtime.add_stage(
        "physics",
        &[],
        PhysicsStepStage::new(1.0 / 60.0, PhysicsDispatchMode::Sequential),
    );
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

/// `PhysicsStepStage` in `PhysicsDispatchMode::Batched` must settle a
/// box exactly the way `PhysicsDispatchMode::Sequential`/
/// `PhysicsSubsystem::step` do — same test shape as
/// `physics_subsystem_step_settles_a_box_without_bouncing_or_sinking`,
/// run through the batched-dispatch mode instead of the plain
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
    let physics_id = runtime.add_stage(
        "physics",
        &[],
        PhysicsStepStage::new(1.0 / 60.0, PhysicsDispatchMode::Batched),
    );

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

/// One tick of `PhysicsStepStage` in `PhysicsDispatchMode::Batched`
/// must match one tick in `PhysicsDispatchMode::Sequential`
/// bit-for-bit on a simple falling body — the direct "same
/// algorithm, different dispatch" proof, independent of the longer
/// settling regression above.
#[test]
fn physics_compute_step_stage_matches_physics_step_stage_for_one_tick() {
    let (physics_a, audio_a) = subsystem_pair();
    let mut physics_a = physics_a;
    physics_a.bodies.push(falling_body());
    let mut runtime_a = Runtime::with_worker_count(physics_a, audio_a, 2);
    let stage_a = runtime_a.add_stage(
        "physics",
        &[],
        PhysicsStepStage::new(1.0 / 60.0, PhysicsDispatchMode::Sequential),
    );
    runtime_a.tick_only(&[stage_a]);

    let (physics_b, audio_b) = subsystem_pair();
    let mut physics_b = physics_b;
    physics_b.bodies.push(falling_body());
    let mut runtime_b = Runtime::with_worker_count(physics_b, audio_b, 2);
    let stage_b = runtime_b.add_stage(
        "physics",
        &[],
        PhysicsStepStage::new(1.0 / 60.0, PhysicsDispatchMode::Batched),
    );
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
