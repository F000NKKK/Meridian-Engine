//! Which events reach an editor.

use meridian_editor_app::Subscriptions;
use meridian_editor_domain::{
    Event, EventTopic, HostMessage, LogLevel, PlayState, Request, Response,
};

use crate::rig::Rig;

fn log_line() -> Event {
    Event::Log {
        level: LogLevel::Info,
        message: "hello".into(),
    }
}

#[test]
fn nothing_is_sent_before_a_subscription() {
    let rig = Rig::greeted(&[]);
    assert_eq!(rig.session.announce(log_line()), None);
}

#[test]
fn a_topic_gets_its_events_and_only_those() {
    let rig = Rig::greeted(&[EventTopic::Log]);
    assert_eq!(
        rig.session.announce(log_line()),
        Some(HostMessage::Event { event: log_line() })
    );
    let other = Event::PlayStateChanged {
        state: PlayState::Playing,
    };
    assert_eq!(rig.session.announce(other), None);
}

#[test]
fn unsubscribing_stops_the_events() {
    let mut rig = Rig::greeted(&[EventTopic::Log]);
    let answer = rig.respond(Request::Unsubscribe {
        topic: EventTopic::Log,
    });
    assert_eq!(answer, Response::Done);
    assert_eq!(rig.session.announce(log_line()), None);
}

#[test]
fn unsubscribing_from_a_topic_never_joined_is_harmless() {
    let mut rig = Rig::greeted(&[]);
    let answer = rig.respond(Request::Unsubscribe {
        topic: EventTopic::Entities,
    });
    assert_eq!(answer, Response::Done);
}

#[test]
fn subscribing_twice_is_the_same_as_once() {
    let mut subscriptions = Subscriptions::new();
    subscriptions.subscribe(EventTopic::Components);
    subscriptions.subscribe(EventTopic::Components);
    subscriptions.unsubscribe(EventTopic::Components);
    assert!(!subscriptions.contains(EventTopic::Components));
}

#[test]
fn every_event_belongs_to_the_topic_it_documents() {
    use meridian_editor_domain::EntityRef;
    use substrate_data::Value;
    let entity = EntityRef {
        index: 0,
        generation: 0,
    };
    let component = meridian_editor_domain::ComponentRef::new("health").unwrap();
    let cases = [
        (Event::EntityAdded { entity }, EventTopic::Entities),
        (Event::EntityRemoved { entity }, EventTopic::Entities),
        (
            Event::ComponentChanged {
                entity,
                component,
                value: Value::Int(1),
            },
            EventTopic::Components,
        ),
        (log_line(), EventTopic::Log),
        (
            Event::PlayStateChanged {
                state: PlayState::Paused,
            },
            EventTopic::PlayState,
        ),
    ];
    for (event, topic) in cases {
        let mut subscriptions = Subscriptions::new();
        assert!(!subscriptions.wants(&event), "{event:?}");
        subscriptions.subscribe(topic);
        assert!(subscriptions.wants(&event), "{event:?}");
    }
}
