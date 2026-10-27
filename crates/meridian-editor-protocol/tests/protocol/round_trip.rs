//! Every variant survives a trip through a stream.

use meridian_editor_protocol::{
    ProtocolVersion, RequestEnvelope, RequestId, read_message, write_message,
};
use substrate_data::Schema;

use crate::samples;

fn round_trip<T: Schema + PartialEq + std::fmt::Debug>(message: &T) {
    let mut stream = Vec::new();
    write_message(&mut stream, message).expect("a Vec accepts writes");
    let read = read_message::<T>(&mut stream.as_slice()).expect("reads back");
    assert_eq!(read.as_ref(), Some(message));
}

#[test]
fn every_request_round_trips() {
    for (i, request) in samples::requests().into_iter().enumerate() {
        let id = RequestId::new(u64::try_from(i).expect("a small index"));
        round_trip(&RequestEnvelope { id, request });
    }
}

#[test]
fn every_response_error_event_and_host_message_round_trips() {
    samples::responses().iter().for_each(round_trip);
    samples::errors().iter().for_each(round_trip);
    samples::events().iter().for_each(round_trip);
    samples::host_messages().iter().for_each(round_trip);
}

#[test]
fn every_topic_level_and_state_round_trips() {
    samples::TOPICS.iter().for_each(round_trip);
    samples::LEVELS.iter().for_each(round_trip);
    samples::STATES.iter().for_each(round_trip);
}

#[test]
fn messages_follow_each_other_and_the_stream_ends_cleanly() {
    let mut stream = Vec::new();
    for host in samples::host_messages() {
        write_message(&mut stream, &host).expect("a Vec accepts writes");
    }
    let mut input = stream.as_slice();
    for host in samples::host_messages() {
        assert_eq!(read_message(&mut input).expect("reads"), Some(host));
    }
    assert_eq!(
        read_message::<meridian_editor_protocol::HostMessage>(&mut input).expect("clean end"),
        None
    );
}

#[test]
fn hello_negotiates_the_lower_version() {
    let host = ProtocolVersion::new(3);
    assert_eq!(
        host.negotiate(ProtocolVersion::new(1)),
        Ok(ProtocolVersion::new(1))
    );
    assert_eq!(host.negotiate(host), Ok(host));
    assert_eq!(
        host.negotiate(ProtocolVersion::new(4)),
        Err(meridian_editor_protocol::RequestError::VersionMismatch {
            host,
            client: ProtocolVersion::new(4)
        })
    );
    assert_eq!(ProtocolVersion::CURRENT.number(), 1);
}

#[test]
fn request_ids_count_up_and_entities_convert() {
    assert_eq!(RequestId::new(4).next(), RequestId::new(5));
    assert_eq!(RequestId::new(u64::MAX).next(), RequestId::new(0));
    let entity = meridian_ecs_core::Entity {
        index: 3,
        generation: 1,
    };
    let back: meridian_ecs_core::Entity = meridian_editor_protocol::EntityRef::from(entity).into();
    assert_eq!(back, entity);
}
