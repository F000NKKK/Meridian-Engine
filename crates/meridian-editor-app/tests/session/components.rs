//! `GetComponent` and `SetComponent`, and what they announce.

use meridian_editor_domain::{
    ComponentRef, EntityRef, Event, EventTopic, HostMessage, Request, RequestError, Response,
};
use substrate_data::{Code, Map, Problem, Schema, Value};

use crate::rig::{Health, Rig, health_name};

fn health(points: i32) -> Value {
    Health { points }.write()
}

fn set(rig: &mut Rig, entity: EntityRef, value: Value) -> Response {
    rig.respond(Request::SetComponent {
        entity,
        component: health_name(),
        value,
    })
}

fn failed(error: RequestError) -> Response {
    Response::Failed { error }
}

#[test]
fn a_set_component_is_read_back() {
    let mut rig = Rig::greeted(&[]);
    let entity = rig.spawn();
    assert_eq!(set(&mut rig, entity, health(7)), Response::Done);
    let got = rig.respond(Request::GetComponent {
        entity,
        component: health_name(),
    });
    assert_eq!(got, Response::Component { value: health(7) });
    let target = meridian_ecs::Entity::from(entity);
    assert_eq!(rig.world.get::<Health>(target), Some(&Health { points: 7 }));
}

#[test]
fn setting_again_replaces_the_value() {
    let mut rig = Rig::greeted(&[]);
    let entity = rig.spawn();
    set(&mut rig, entity, health(1));
    set(&mut rig, entity, health(2));
    let target = meridian_ecs::Entity::from(entity);
    assert_eq!(rig.world.get::<Health>(target), Some(&Health { points: 2 }));
}

#[test]
fn an_invalid_value_is_refused_with_diagnostics_and_changes_nothing() {
    let mut rig = Rig::greeted(&[EventTopic::Components]);
    let entity = rig.spawn();
    let mut wrong = Map::new();
    wrong.insert("points", Value::Str("lots".into()));
    let sent = rig.send(Request::SetComponent {
        entity,
        component: health_name(),
        value: Value::Map(wrong),
    });
    assert_eq!(sent.len(), 1, "no event for a refused edit");
    let HostMessage::Response {
        response:
            Response::Failed {
                error: RequestError::InvalidValue { diagnostics },
            },
        ..
    } = &sent[0]
    else {
        panic!("expected InvalidValue, got {sent:?}");
    };
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics
            .iter()
            .any(|d| d.path.to_string().contains("points")),
        "{diagnostics:?}"
    );
    let target = meridian_ecs::Entity::from(entity);
    assert!(rig.world.get::<Health>(target).is_none());
}

#[test]
fn a_value_missing_a_field_says_which() {
    let mut rig = Rig::greeted(&[]);
    let entity = rig.spawn();
    let answer = set(&mut rig, entity, Value::Map(Map::new()));
    let Response::Failed {
        error: RequestError::InvalidValue { diagnostics },
    } = answer
    else {
        panic!("expected InvalidValue, got {answer:?}");
    };
    assert!(
        diagnostics
            .iter()
            .any(|d| d.code == Some(Code::Data(Problem::MissingField)))
    );
}

#[test]
fn an_unknown_component_is_no_such_component() {
    let mut rig = Rig::greeted(&[]);
    let entity = rig.spawn();
    let component = ComponentRef::new("warp_core").unwrap();
    let expected = failed(RequestError::NoSuchComponent {
        component: component.clone(),
    });
    let get = Request::GetComponent {
        entity,
        component: component.clone(),
    };
    assert_eq!(rig.respond(get), expected);
    let set = Request::SetComponent {
        entity,
        component,
        value: health(1),
    };
    assert_eq!(rig.respond(set), expected);
}

#[test]
fn an_entity_without_the_component_is_component_missing() {
    let mut rig = Rig::greeted(&[]);
    let entity = rig.spawn();
    let got = rig.respond(Request::GetComponent {
        entity,
        component: health_name(),
    });
    assert_eq!(
        got,
        failed(RequestError::ComponentMissing {
            entity,
            component: health_name()
        })
    );
}

#[test]
fn a_dead_entity_is_no_such_entity_for_get_and_set() {
    let mut rig = Rig::greeted(&[]);
    let entity = rig.spawn();
    rig.respond(Request::DespawnEntity { entity });
    let expected = failed(RequestError::NoSuchEntity { entity });
    let get = Request::GetComponent {
        entity,
        component: health_name(),
    };
    assert_eq!(rig.respond(get), expected);
    assert_eq!(set(&mut rig, entity, health(1)), expected);
}

#[test]
fn a_set_announces_the_stored_value_to_the_components_topic() {
    let mut rig = Rig::greeted(&[EventTopic::Components]);
    let entity = rig.spawn();
    let sent = rig.send(Request::SetComponent {
        entity,
        component: health_name(),
        value: health(3),
    });
    assert_eq!(sent.len(), 2);
    assert_eq!(
        sent[1],
        HostMessage::Event {
            event: Event::ComponentChanged {
                entity,
                component: health_name(),
                value: health(3),
            }
        }
    );
}
