//! Broken streams give typed errors, never panics.

use meridian_editor_protocol::{FrameError, Request, RequestEnvelope, read_message, write_message};
use substrate_data::{Code, Problem};

use crate::samples;

fn framed(bytes: &[u8]) -> Vec<u8> {
    let len = u32::try_from(bytes.len()).expect("a small frame");
    let mut frame = len.to_le_bytes().to_vec();
    frame.extend_from_slice(bytes);
    frame
}

fn framed_text(text: &str) -> Vec<u8> {
    let value = substrate_data::text::parse_value(text).expect("test text parses");
    framed(&substrate_data::to_bytes(&value))
}

fn read_request(stream: &[u8]) -> Result<Option<RequestEnvelope>, FrameError> {
    read_message(&mut &stream[..])
}

#[test]
fn a_frame_cut_inside_its_body_is_truncated() {
    let mut stream = Vec::new();
    let request = samples::requests().remove(3);
    let id = meridian_editor_protocol::RequestId::new(1);
    write_message(&mut stream, &RequestEnvelope { id, request }).expect("writes");
    stream.truncate(stream.len() - 1);
    assert!(matches!(read_request(&stream), Err(FrameError::Truncated)));
}

#[test]
fn bytes_that_are_not_a_message_are_malformed() {
    let stream = framed(b"\xff\xfe garbage");
    assert!(matches!(read_request(&stream), Err(FrameError::Malformed(d)) if !d.is_empty()));
}

#[test]
fn a_frame_of_another_type_is_malformed() {
    let stream = framed_text("[1, 2, 3]");
    assert!(matches!(
        read_request(&stream),
        Err(FrameError::Malformed(_))
    ));
}

#[test]
fn an_unknown_variant_is_malformed_and_says_where() {
    let stream = framed_text(r#"{ id: 1, request: { type: "warp_drive" } }"#);
    let Err(FrameError::Malformed(problems)) = read_request(&stream) else {
        panic!("an unknown variant must be rejected");
    };
    assert!(
        problems
            .iter()
            .any(|p| p.path.to_string().contains("request"))
    );
}

#[test]
fn a_missing_field_is_malformed() {
    let stream = framed_text(r#"{ id: 1, request: { type: "get_component" } }"#);
    let Err(FrameError::Malformed(problems)) = read_request(&stream) else {
        panic!("a missing field must be rejected");
    };
    assert!(
        problems
            .iter()
            .any(|p| p.code == Some(Code::Data(Problem::MissingField)))
    );
}

#[test]
fn a_length_over_the_limit_is_too_long() {
    let stream = u32::MAX.to_le_bytes();
    assert!(matches!(read_request(&stream), Err(FrameError::TooLong)));
}

#[test]
fn an_empty_stream_ends_cleanly() {
    assert!(matches!(read_request(&[]), Ok(None)));
}

#[test]
fn a_valid_frame_still_reads_after_the_checks() {
    let stream = framed_text(r#"{ id: 1, request: { type: "play" } }"#);
    let read = read_request(&stream).expect("valid").expect("a message");
    assert_eq!(read.request, Request::Play);
}

#[test]
fn a_field_of_a_newer_peer_is_reported_not_skipped() {
    // The named binary form refuses what it doesn't know: this is why a peer
    // never sends past the other's `ProtocolVersion` (docs/editor-protocol.md).
    let stream = framed_text(r#"{ id: 1, request: { type: "play", speed: 2 } }"#);
    let Err(FrameError::Malformed(problems)) = read_request(&stream) else {
        panic!("an unknown field must be rejected");
    };
    assert!(
        problems
            .iter()
            .any(|p| p.code == Some(Code::Data(Problem::UnknownField)))
    );
}
