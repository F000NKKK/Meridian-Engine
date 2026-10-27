//! The append-only rule: declaration order is pinned, so a reordered or
//! removed variant fails here.

use meridian_editor_domain::{
    Event, EventTopic, HostMessage, LogLevel, PlayState, Request, RequestError, Response,
};
use substrate_data::{Schema, Shape};

fn variants<T: Schema>() -> Vec<&'static str> {
    match T::shape() {
        Shape::Enum(shape) => shape.variants.iter().map(|v| v.name).collect(),
        other => panic!("not an enum: {other:?}"),
    }
}

#[test]
fn request_variants_are_pinned() {
    assert_eq!(
        variants::<Request>(),
        [
            "hello",
            "describe",
            "list_entities",
            "get_component",
            "set_component",
            "spawn_entity",
            "despawn_entity",
            "play",
            "pause",
            "step",
            "subscribe",
            "unsubscribe",
        ]
    );
}

#[test]
fn response_and_error_variants_are_pinned() {
    assert_eq!(
        variants::<Response>(),
        [
            "welcome",
            "described",
            "entities",
            "component",
            "spawned",
            "done",
            "failed"
        ]
    );
    assert_eq!(
        variants::<RequestError>(),
        [
            "version_mismatch",
            "not_greeted",
            "no_such_entity",
            "no_such_component",
            "component_missing",
            "invalid_value",
            "unsupported"
        ]
    );
}

#[test]
fn event_and_host_message_variants_are_pinned() {
    assert_eq!(
        variants::<Event>(),
        [
            "entity_added",
            "entity_removed",
            "component_changed",
            "log",
            "play_state_changed"
        ]
    );
    assert_eq!(variants::<HostMessage>(), ["response", "event"]);
}

#[test]
fn unit_enum_variants_are_pinned() {
    assert_eq!(
        variants::<EventTopic>(),
        ["entities", "components", "log", "play_state"]
    );
    assert_eq!(
        variants::<LogLevel>(),
        ["debug", "info", "warning", "error"]
    );
    assert_eq!(variants::<PlayState>(), ["playing", "paused"]);
}
