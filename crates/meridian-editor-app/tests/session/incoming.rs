//! Frames the host cannot read as a request.

use meridian_editor_app::{Incoming, unsupported};
use meridian_editor_domain::{
    HostMessage, Request, RequestEnvelope, RequestError, RequestId, Response,
};
use substrate_data::Value;

fn frame(text: &str) -> Value {
    substrate_data::text::parse_value(text).expect("test text parses")
}

#[test]
fn a_known_request_is_a_request() {
    let incoming = Incoming::classify(&frame(r#"{ id: 5, request: { type: "play" } }"#));
    assert_eq!(
        incoming,
        Incoming::Request(RequestEnvelope {
            id: RequestId::new(5),
            request: Request::Play
        })
    );
}

#[test]
fn an_unknown_request_keeps_its_id_and_is_answered_unsupported() {
    let incoming = Incoming::classify(&frame(
        r#"{ id: 41, request: { type: "warp_drive", speed: 9 } }"#,
    ));
    assert_eq!(incoming, Incoming::Unsupported(RequestId::new(41)));
    assert_eq!(
        unsupported(RequestId::new(41)),
        HostMessage::Response {
            id: RequestId::new(41),
            response: Response::Failed {
                error: RequestError::Unsupported
            }
        }
    );
}

#[test]
fn a_frame_without_a_readable_id_is_unidentifiable() {
    for text in [
        r#"{ request: { type: "play" } }"#,
        r#"{ id: "seven", request: { type: "warp_drive" } }"#,
        "[1, 2, 3]",
    ] {
        let incoming = Incoming::classify(&frame(text));
        assert!(matches!(incoming, Incoming::Unidentifiable(_)), "{text}");
    }
}
