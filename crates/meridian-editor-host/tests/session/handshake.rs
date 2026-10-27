//! `Hello`, versions, and requests before `Hello`.

use meridian_editor_host::Session;
use meridian_editor_protocol::{
    EventTopic, HostMessage, ProtocolVersion, Request, RequestError, RequestId, Response,
};

use crate::rig::Rig;

fn failed(error: RequestError) -> Response {
    Response::Failed { error }
}

#[test]
fn hello_is_welcomed_with_the_negotiated_version() {
    let mut rig = Rig::new();
    let sent = rig.send(Request::Hello {
        version: ProtocolVersion::CURRENT,
    });
    assert_eq!(
        sent,
        vec![HostMessage::Response {
            id: RequestId::new(1),
            response: Response::Welcome {
                version: ProtocolVersion::CURRENT
            }
        }]
    );
    assert_eq!(rig.session.negotiated(), Some(ProtocolVersion::CURRENT));
}

#[test]
fn an_older_editor_gets_its_own_version() {
    let mut rig = Rig::new();
    rig.session = Session::speaking(ProtocolVersion::new(3));
    let welcome = rig.respond(Request::Hello {
        version: ProtocolVersion::new(2),
    });
    assert_eq!(
        welcome,
        Response::Welcome {
            version: ProtocolVersion::new(2)
        }
    );
    assert_eq!(rig.session.negotiated(), Some(ProtocolVersion::new(2)));
}

#[test]
fn a_newer_editor_is_refused_and_stays_ungreeted() {
    let mut rig = Rig::new();
    let newer = ProtocolVersion::new(ProtocolVersion::CURRENT.number() + 1);
    let answer = rig.respond(Request::Hello { version: newer });
    assert_eq!(
        answer,
        failed(RequestError::VersionMismatch {
            host: ProtocolVersion::CURRENT,
            client: newer
        })
    );
    assert_eq!(rig.session.negotiated(), None);
    assert_eq!(
        rig.respond(Request::Describe),
        failed(RequestError::NotGreeted)
    );
}

#[test]
fn every_request_but_hello_needs_a_hello_first() {
    let requests = [
        Request::Describe,
        Request::ListEntities,
        Request::SpawnEntity,
        Request::Play,
        Request::Pause,
        Request::Step,
        Request::Subscribe {
            topic: EventTopic::Log,
        },
        Request::Unsubscribe {
            topic: EventTopic::Log,
        },
    ];
    for request in requests {
        let mut rig = Rig::new();
        let sent = rig.send(request.clone());
        assert_eq!(sent.len(), 1, "{request:?}");
        assert_eq!(
            rig.respond(request.clone()),
            failed(RequestError::NotGreeted),
            "{request:?}"
        );
    }
    let rig = Rig::new();
    assert!(!rig.session.subscriptions().contains(EventTopic::Log));
}

#[test]
fn a_request_before_hello_changes_nothing() {
    let mut rig = Rig::new();
    rig.respond(Request::SpawnEntity);
    assert_eq!(rig.world.entities().count(), 0);
    rig.respond(Request::Play);
    assert_eq!(
        meridian_editor_host::SimulationControl::state(&rig.simulation),
        meridian_editor_protocol::PlayState::Paused
    );
}

#[test]
fn describe_is_the_registrys_own_description() {
    let mut rig = Rig::greeted(&[]);
    let expected = rig.registry.describe();
    assert_eq!(
        rig.respond(Request::Describe),
        Response::Described {
            components: expected
        }
    );
}

#[test]
fn a_second_hello_negotiates_again() {
    let mut rig = Rig::greeted(&[]);
    let again = rig.respond(Request::Hello {
        version: ProtocolVersion::new(1),
    });
    assert!(matches!(again, Response::Welcome { .. }));
}
