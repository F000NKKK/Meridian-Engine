//! Streams that end early or carry nonsense, over in-memory pipes.

use meridian_editor_domain::{
    FrameError, HostMessage, ProtocolVersion, Request, RequestError, RequestId, Response,
};
use meridian_editor_tcp::{ServeError, serve_stream};
use substrate_data::Value;
use tokio::io::{AsyncWriteExt, DuplexStream, duplex};

use crate::host::{Client, TestHost};

/// Serves `server` while `editor` runs on the other end; the editor's end
/// is dropped when it finishes, as a hung-up socket is.
async fn serve<F, Fut>(editor: F) -> Result<(), ServeError>
where
    F: FnOnce(DuplexStream) -> Fut,
    Fut: Future<Output = ()>,
{
    let (near, far) = duplex(64 * 1024);
    let mut host = TestHost::new();
    let (served, ()) = tokio::join!(serve_stream(far, &mut host), editor(near));
    served
}

fn framed_text(text: &str) -> Vec<u8> {
    let value = substrate_data::text::parse_value(text).expect("test text parses");
    let body = substrate_data::to_bytes(&value);
    let mut frame = u32::try_from(body.len())
        .expect("a small frame")
        .to_le_bytes()
        .to_vec();
    frame.extend_from_slice(&body);
    frame
}

#[tokio::test]
async fn an_editor_that_hangs_up_between_frames_ends_cleanly() {
    let served = serve(|_stream| async {}).await;
    assert!(served.is_ok());
}

#[tokio::test]
async fn a_stream_cut_inside_the_length_is_truncated() {
    let served = serve(|mut stream| async move {
        stream.write_all(&[7, 0]).await.unwrap();
    })
    .await;
    assert!(matches!(
        served,
        Err(ServeError::Frame(FrameError::Truncated))
    ));
}

#[tokio::test]
async fn a_stream_cut_inside_the_body_is_truncated() {
    let served = serve(|mut stream| async move {
        let mut frame = framed_text(r#"{ id: 1, request: { type: "play" } }"#);
        frame.truncate(frame.len() - 1);
        stream.write_all(&frame).await.unwrap();
    })
    .await;
    assert!(matches!(
        served,
        Err(ServeError::Frame(FrameError::Truncated))
    ));
}

#[tokio::test]
async fn a_length_over_the_limit_is_too_long() {
    let served = serve(|mut stream| async move {
        stream.write_all(&u32::MAX.to_le_bytes()).await.unwrap();
    })
    .await;
    assert!(matches!(
        served,
        Err(ServeError::Frame(FrameError::TooLong))
    ));
}

#[tokio::test]
async fn bytes_that_are_not_a_message_are_malformed() {
    let served = serve(|mut stream| async move {
        let mut frame = 5u32.to_le_bytes().to_vec();
        frame.extend_from_slice(b"\xff\xfe\xfd\xfc\xfb");
        stream.write_all(&frame).await.unwrap();
    })
    .await;
    assert!(matches!(
        served,
        Err(ServeError::Frame(FrameError::Malformed(_)))
    ));
}

#[tokio::test]
async fn a_frame_with_no_request_id_is_a_broken_peer() {
    let served = serve(|mut stream| async move {
        let frame = framed_text(r#"{ request: { type: "play" } }"#);
        stream.write_all(&frame).await.unwrap();
    })
    .await;
    assert!(matches!(served, Err(ServeError::BrokenPeer(_))));
}

#[tokio::test]
async fn an_unknown_request_is_answered_unsupported_and_the_session_goes_on() {
    let served = serve(|stream| async move {
        let unknown = framed_text(r#"{ id: 41, request: { type: "warp_drive", speed: 9 } }"#);
        let mut stream = stream;
        stream.write_all(&unknown).await.unwrap();
        let mut client = Client::new(stream);
        let answer = client.next_message().await;
        assert_eq!(
            answer,
            HostMessage::Response {
                id: RequestId::new(41),
                response: Response::Failed {
                    error: RequestError::Unsupported
                }
            }
        );
        let (welcome, _) = client
            .call(Request::Hello {
                version: ProtocolVersion::CURRENT,
            })
            .await;
        assert!(matches!(welcome, Response::Welcome { .. }));
    })
    .await;
    assert!(served.is_ok());
}

#[tokio::test]
async fn messages_round_trip_through_the_async_twins() {
    let (mut near, mut far) = duplex(1024);
    let value = Value::Int(7);
    meridian_editor_tcp::write_message(&mut near, &value)
        .await
        .unwrap();
    let read: Option<Value> = meridian_editor_tcp::read_message(&mut far).await.unwrap();
    assert_eq!(read, Some(value));
}
