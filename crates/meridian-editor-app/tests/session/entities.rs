//! Listing, spawning and despawning.

use meridian_editor_domain::{
    EntityRef, Event, EventTopic, HostMessage, Request, RequestError, Response,
};

use crate::rig::Rig;

#[test]
fn an_empty_world_lists_nothing() {
    let mut rig = Rig::greeted(&[]);
    assert_eq!(rig.entities(), Vec::new());
}

#[test]
fn spawned_entities_are_listed_in_index_order() {
    let mut rig = Rig::greeted(&[]);
    let first = rig.spawn();
    let second = rig.spawn();
    let third = rig.spawn();
    assert_eq!(rig.entities(), vec![first, second, third]);
}

#[test]
fn despawning_removes_the_entity() {
    let mut rig = Rig::greeted(&[]);
    let first = rig.spawn();
    let second = rig.spawn();
    assert_eq!(
        rig.respond(Request::DespawnEntity { entity: first }),
        Response::Done
    );
    assert_eq!(rig.entities(), vec![second]);
}

#[test]
fn despawning_a_dead_entity_is_no_such_entity() {
    let mut rig = Rig::greeted(&[]);
    let entity = rig.spawn();
    rig.respond(Request::DespawnEntity { entity });
    assert_eq!(
        rig.respond(Request::DespawnEntity { entity }),
        Response::Failed {
            error: RequestError::NoSuchEntity { entity }
        }
    );
}

#[test]
fn spawn_and_despawn_announce_to_the_entities_topic() {
    let mut rig = Rig::greeted(&[EventTopic::Entities]);
    let spawned = rig.send(Request::SpawnEntity);
    let [HostMessage::Response { .. }, HostMessage::Event { event }] = spawned.as_slice() else {
        panic!("expected a response then an event, got {spawned:?}");
    };
    let Event::EntityAdded { entity } = *event else {
        panic!("expected EntityAdded, got {event:?}");
    };
    let despawned = rig.send(Request::DespawnEntity { entity });
    assert!(matches!(
        despawned.as_slice(),
        [HostMessage::Response { .. }, HostMessage::Event { event: Event::EntityRemoved { entity: gone } }]
            if *gone == entity
    ));
}

#[test]
fn a_failed_despawn_announces_nothing() {
    let mut rig = Rig::greeted(&[EventTopic::Entities]);
    let ghost = EntityRef {
        index: 9,
        generation: 9,
    };
    let sent = rig.send(Request::DespawnEntity { entity: ghost });
    assert_eq!(sent.len(), 1);
}
