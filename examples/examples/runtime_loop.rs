//! Roadmap milestone: `meridian-engine-core`'s `Runtime` ties the
//! driver-independent subsystems (ecs-core, physics-core, audio-core)
//! into one real per-frame loop — `SubsystemManager` owns real instances,
//! `Runtime::tick` advances physics then recomputes audio gains from the
//! result, and `EventSystem` decouples that from anything downstream that
//! wants to react to a completed frame. `graphics-core` isn't part of this
//! loop yet: rendering has nothing to submit to without a real
//! `graphics-driver` backend (blocked on the `wgpu` decision — see
//! docs/roadmap.md). The exhaustive numeric checks live in
//! `meridian-engine-core`'s own test suite (`cargo test -p
//! meridian-engine-core`); this is the human-readable version.
//!
//! Run with:
//!   ./build.sh run runtime_loop

use std::thread::sleep;
use std::time::Duration;

use meridian_audio_core::{Channel, Emitter, Listener, Mixer, SpeakerLayout};
use meridian_ecs_core::{Component, Transform, World};
use meridian_engine_core::{FrameCompleted, Runtime, SubsystemManager};
use meridian_gac_core::{Motor3, Vec3};
use meridian_physics_core::{ColliderShape, RigidBody};

#[derive(Debug, Clone, Copy)]
struct Name(&'static str);
impl Component for Name {}

fn check(label: &str, condition: bool) {
    println!("  [{}] {label}", if condition { "OK" } else { "FAIL" });
    assert!(condition, "{label} failed");
}

fn main() {
    println!("== SubsystemManager: real ecs-core + physics-core + audio-core instances ==");
    // No attenuation: this section checks *direction* only (see
    // audio_spatialization.rs's same no_attenuation pattern) — the
    // default AttenuationModel's reference_distance of 1.0 would
    // otherwise scale the gain down at the emitter's distance of 5 and
    // make a directional check ambiguous.
    let mixer = Mixer::new(SpeakerLayout::stereo_headphones()).with_attenuation(
        meridian_audio_core::AttenuationModel {
            reference_distance: 1000.0,
            rolloff: 1.0,
            max_distance: 1000.0,
        },
    );
    let mut subsystems = SubsystemManager::new(mixer);

    // The ecs-core World is available for application-level entities —
    // engine-core doesn't invent a sync between it and physics-core's own
    // RigidBody list (see the crate's module doc for why).
    let mut world = World::new();
    let ball_entity = world.spawn();
    world.insert(
        ball_entity,
        Transform {
            motor: Motor3::translation(Vec3::new(0.0, 10.0, 0.0)),
        },
    );
    world.insert(ball_entity, Name("ball"));
    subsystems.world = world;
    let ball_name = subsystems.world.query::<Name>().next().map(|(_, n)| n.0);
    check(
        "ecs-core entity spawned independently of physics bodies",
        ball_name == Some("ball"),
    );

    // A falling ball above a static floor — the same scenario
    // physics-core's own tests use, driven through Runtime this time.
    subsystems.bodies.push(RigidBody {
        frame: Motor3::translation(Vec3::new(0.0, -50.0, 0.0)),
        mass: 0.0, // static floor
        shape: ColliderShape::Sphere { radius: 50.0 },
        ..Default::default()
    });
    subsystems.bodies.push(RigidBody {
        frame: Motor3::translation(Vec3::new(0.0, 2.0, 0.0)),
        mass: 1.0,
        shape: ColliderShape::Sphere { radius: 0.5 },
        ..Default::default()
    });

    subsystems.listener = Listener {
        frame: Motor3::identity(),
    };
    subsystems.emitters.push((
        Emitter {
            frame: Motor3::translation(Vec3::new(0.0, 0.0, 5.0)), // local +Z: to the right
        },
        1.0,
    ));

    println!("\n== Runtime: tick physics + audio together, drain events ==");
    let mut runtime = Runtime::new(subsystems);

    // Runtime::tick uses real wall-clock time (see the crate's module
    // doc: no fixed-step accumulator yet), the same as a real game calling
    // tick() once per rendered frame — so this sleeps between ticks to
    // simulate a real ~60fps frame cadence. Without it, 60 ticks fired
    // back-to-back would accumulate only microseconds of simulated time,
    // nowhere near enough for the ball to actually fall.
    const TICK_COUNT: usize = 60;
    for _ in 0..TICK_COUNT {
        sleep(Duration::from_millis(16));
        runtime.tick();
    }

    let completed = runtime.events.drain::<FrameCompleted>();
    check(
        "one FrameCompleted event published per tick",
        completed.len() == TICK_COUNT,
    );
    check(
        "frame indices increase monotonically",
        completed
            .windows(2)
            .all(|w| w[1].frame_index == w[0].frame_index + 1),
    );
    check(
        "draining again returns nothing (mailbox, not a log)",
        runtime.events.drain::<FrameCompleted>().is_empty(),
    );

    let resting_height = runtime.subsystems.bodies[1].position().y;
    println!("  ball settled at y={resting_height:.3} after {TICK_COUNT} ticks");
    check(
        "ball fell and came to rest near the floor surface (y=0.5)",
        (resting_height - 0.5).abs() < 0.5,
    );

    let gains = runtime.subsystems.mix_audio();
    let gain_of = |channel: Channel| {
        gains
            .iter()
            .find(|(c, _)| *c == channel)
            .map(|(_, g)| *g)
            .unwrap_or(0.0)
    };
    println!("  audio gains after physics-driven ticks: {gains:?}");
    check(
        "audio still reflects the (physics-independent) emitter's position",
        gain_of(Channel::Right) > 0.99 && gain_of(Channel::Left) < 1e-3,
    );

    println!(
        "\nAll checks passed — Runtime ties ecs-core/physics-core/audio-core into one real per-frame loop."
    );
}
