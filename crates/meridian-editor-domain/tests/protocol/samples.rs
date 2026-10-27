//! One value of every variant of every message enum.

use meridian_editor_domain::{
    ComponentRef, EntityRef, Event, EventTopic, HostMessage, LogLevel, PlayState, ProtocolVersion,
    Request, RequestError, RequestId, Response,
};
use substrate_data::{Diagnostic, Value};

pub fn entity() -> EntityRef {
    EntityRef {
        index: 7,
        generation: 2,
    }
}

pub fn component() -> ComponentRef {
    ComponentRef::new("rigid_body").expect("a valid name")
}

fn value() -> Value {
    Value::Int(42)
}

/// Every `Request` variant, in declaration order.
pub fn requests() -> Vec<Request> {
    vec![
        Request::Hello {
            version: ProtocolVersion::CURRENT,
        },
        Request::Describe,
        Request::ListEntities,
        Request::GetComponent {
            entity: entity(),
            component: component(),
        },
        Request::SetComponent {
            entity: entity(),
            component: component(),
            value: value(),
        },
        Request::SpawnEntity,
        Request::DespawnEntity { entity: entity() },
        Request::Play,
        Request::Pause,
        Request::Step,
        Request::Subscribe {
            topic: EventTopic::Log,
        },
        Request::Unsubscribe {
            topic: EventTopic::Log,
        },
    ]
}

/// Every `RequestError` variant, in declaration order.
pub fn errors() -> Vec<RequestError> {
    vec![
        RequestError::VersionMismatch {
            host: ProtocolVersion::CURRENT,
            client: ProtocolVersion::new(9),
        },
        RequestError::NotGreeted,
        RequestError::NoSuchEntity { entity: entity() },
        RequestError::NoSuchComponent {
            component: component(),
        },
        RequestError::ComponentMissing {
            entity: entity(),
            component: component(),
        },
        RequestError::InvalidValue {
            diagnostics: vec![Diagnostic::from("not a number")],
        },
        RequestError::Unsupported,
    ]
}

/// Every `Response` variant, in declaration order.
pub fn responses() -> Vec<Response> {
    vec![
        Response::Welcome {
            version: ProtocolVersion::CURRENT,
        },
        Response::Described {
            components: Vec::new(),
        },
        Response::Entities {
            entities: vec![entity()],
        },
        Response::Component { value: value() },
        Response::Spawned { entity: entity() },
        Response::Done,
        Response::Failed {
            error: RequestError::NotGreeted,
        },
    ]
}

/// Every `Event` variant, in declaration order.
pub fn events() -> Vec<Event> {
    vec![
        Event::EntityAdded { entity: entity() },
        Event::EntityRemoved { entity: entity() },
        Event::ComponentChanged {
            entity: entity(),
            component: component(),
            value: value(),
        },
        Event::Log {
            level: LogLevel::Warning,
            message: "low on memory".to_owned(),
        },
        Event::PlayStateChanged {
            state: PlayState::Paused,
        },
    ]
}

/// Every `HostMessage` variant, in declaration order.
pub fn host_messages() -> Vec<HostMessage> {
    vec![
        HostMessage::Response {
            id: RequestId::new(3),
            response: Response::Done,
        },
        HostMessage::Event {
            event: Event::EntityAdded { entity: entity() },
        },
    ]
}

pub const TOPICS: [EventTopic; 4] = [
    EventTopic::Entities,
    EventTopic::Components,
    EventTopic::Log,
    EventTopic::PlayState,
];

pub const LEVELS: [LogLevel; 4] = [
    LogLevel::Debug,
    LogLevel::Info,
    LogLevel::Warning,
    LogLevel::Error,
];

pub const STATES: [PlayState; 2] = [PlayState::Playing, PlayState::Paused];
