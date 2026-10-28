//! Tests of the contact sound stage and the physics stage's contact events.

use std::sync::Arc;

use meridian_audio_app::AudioScene;
use meridian_audio_domain::{Emitter, MemorySource, Mixer, Repeat, SpeakerLayout, Voice};
use meridian_gac::{Motor3, Vec3};
use meridian_physics_app::{ContactEvent, ContactWatcher, PhysicsSubsystem, SequentialStepper};
use meridian_physics_domain::{ColliderShape, RigidBody};

use crate::testing::InlineExecutor;
use crate::*;

/// A stand-in for the physics step: publishes one canned contact.
struct StubPhysics {
    events: Shared<EventSystem>,
}

impl Stage for StubPhysics {
    fn run(&mut self) {
        self.events.lock().publish(ContactEvent {
            a: 0,
            b: 1,
            point: Vec3::new(1.0, 2.0, 3.0),
            normal: Vec3::new(1.0, 0.0, 0.0),
            impact_speed: 4.0,
        });
    }
}

fn scene() -> Shared<AudioScene> {
    Shared::new(AudioScene::new(Mixer::new(
        SpeakerLayout::stereo_headphones(),
    )))
}

fn thud(at: Vec3) -> Voice {
    Voice::new(
        MemorySource::new(vec![1.0; 16], Repeat::Once),
        Emitter {
            frame: Motor3::translation(at),
        },
    )
}

#[test]
fn a_contact_published_by_the_physics_stage_becomes_a_voice_at_the_contact_point() {
    let events = Shared::new(EventSystem::new());
    let scene = scene();
    let mut runtime = Runtime::new(Arc::new(InlineExecutor));
    let physics = runtime.add_stage(
        "physics",
        &[],
        StubPhysics {
            events: events.clone(),
        },
    );
    let heard_at = Shared::new(Vec::new());
    let sink = heard_at.clone();
    runtime.add_stage(
        "contact-sound",
        &[physics],
        ContactSoundStage::new(events, scene.clone(), move |contact: &ContactEvent| {
            sink.lock().push(contact.impact_speed);
            Some(thud(contact.point))
        }),
    );

    runtime.tick();

    assert_eq!(scene.lock().voice_count(), 1);
    assert_eq!(*heard_at.lock(), vec![4.0]);
}

#[test]
fn contacts_the_application_does_not_voice_add_nothing() {
    let events = Shared::new(EventSystem::new());
    let scene = scene();
    let mut runtime = Runtime::new(Arc::new(InlineExecutor));
    let physics = runtime.add_stage(
        "physics",
        &[],
        StubPhysics {
            events: events.clone(),
        },
    );
    runtime.add_stage(
        "contact-sound",
        &[physics],
        ContactSoundStage::new(events, scene.clone(), |_: &ContactEvent| None),
    );

    runtime.tick();

    assert_eq!(scene.lock().voice_count(), 0);
}

#[test]
fn a_tick_without_contacts_leaves_the_scene_alone() {
    let events = Shared::new(EventSystem::new());
    let scene = scene();
    let mut runtime = Runtime::new(Arc::new(InlineExecutor));
    runtime.add_stage(
        "contact-sound",
        &[],
        ContactSoundStage::new(events, scene.clone(), |c: &ContactEvent| {
            Some(thud(c.point))
        }),
    );

    runtime.tick();

    assert_eq!(scene.lock().voice_count(), 0);
}

fn sphere(at: Vec3, velocity: Vec3) -> RigidBody {
    RigidBody {
        frame: Motor3::translation(at),
        velocity,
        mass: 1.0,
        shape: ColliderShape::Sphere { radius: 0.5 },
        ..Default::default()
    }
}

#[test]
fn the_physics_stage_publishes_contacts_and_the_sound_stage_voices_them() {
    let physics = Shared::new(PhysicsSubsystem::default());
    physics
        .lock()
        .bodies
        .push(sphere(Vec3::new(-2.0, 0.0, 0.0), Vec3::new(5.0, 0.0, 0.0)));
    physics
        .lock()
        .bodies
        .push(sphere(Vec3::new(2.0, 0.0, 0.0), Vec3::new(-5.0, 0.0, 0.0)));
    let events = Shared::new(EventSystem::new());
    let scene = scene();
    let mut runtime = Runtime::new(Arc::new(InlineExecutor));
    let step = runtime.add_stage(
        "physics",
        &[],
        PhysicsStepStage::new(1.0 / 60.0, SequentialStepper, physics)
            .with_contact_events(ContactWatcher::new(1.0), events.clone()),
    );
    runtime.add_stage(
        "contact-sound",
        &[step],
        ContactSoundStage::new(events, scene.clone(), |c: &ContactEvent| {
            Some(thud(c.point))
        }),
    );

    for _ in 0..120 {
        runtime.tick();
    }

    assert_eq!(scene.lock().voice_count(), 1, "one collision, one thud");
}
