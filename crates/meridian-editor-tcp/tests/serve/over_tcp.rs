//! A whole session over a real socket on loopback.

use meridian_ecs_core::Entity;
use meridian_editor_protocol::{
    EntityRef, Event, EventTopic, HostMessage, ProtocolVersion, Request, RequestError, Response,
};
use meridian_editor_tcp::{EditorListener, ServeError};
use substrate_data::{Schema, Value};
use tokio::net::TcpStream;

use crate::host::{Client, Health, TestHost, health_name};

async fn hello(client: &mut Client<TcpStream>) {
    let (welcome, _) = client
        .call(Request::Hello {
            version: ProtocolVersion::CURRENT,
        })
        .await;
    let expected = Response::Welcome {
        version: ProtocolVersion::CURRENT,
    };
    assert_eq!(welcome, expected);
}

async fn describe_and_list(client: &mut Client<TcpStream>, entity: EntityRef) {
    let (described, _) = client.call(Request::Describe).await;
    let names: Vec<_> = match described {
        Response::Described { components } => components.into_iter().map(|c| c.name).collect(),
        _ => Vec::new(),
    };
    assert_eq!(names, vec![health_name()]);

    let (listed, _) = client.call(Request::ListEntities).await;
    let expected = Response::Entities {
        entities: vec![entity],
    };
    assert_eq!(listed, expected);
}

async fn set_and_observe(client: &mut Client<TcpStream>, entity: EntityRef, value: Value) {
    let topic = EventTopic::Components;
    let (subscribed, _) = client.call(Request::Subscribe { topic }).await;
    assert_eq!(subscribed, Response::Done);
    let set = Request::SetComponent {
        entity,
        component: health_name(),
        value: value.clone(),
    };
    let id = client.send(set).await;
    let answer = client.next_message().await;
    let done = HostMessage::Response {
        id,
        response: Response::Done,
    };
    assert_eq!(answer, done);
    let event = client.next_message().await;
    let changed = HostMessage::Event {
        event: Event::ComponentChanged {
            entity,
            component: health_name(),
            value,
        },
    };
    assert_eq!(event, changed);
}

#[tokio::test]
async fn hello_describe_list_set_and_the_event() {
    let mut host = TestHost::new();
    let entity = host.world.spawn();
    let listener = EditorListener::bind(0).await.unwrap();
    let address = listener.address().unwrap();
    assert!(address.ip().is_loopback());

    let reference = EntityRef::from(entity);
    let value = Health { points: 12 }.write();
    let editor = async {
        let mut client = Client::new(TcpStream::connect(address).await.unwrap());
        hello(&mut client).await;
        describe_and_list(&mut client, reference).await;
        set_and_observe(&mut client, reference, value).await;
    };
    let (served, ()) = tokio::join!(listener.serve_one(&mut host), editor);
    served.expect("a clean end when the editor hangs up");
    let stored = host.world.get::<Health>(Entity::from(reference));
    assert_eq!(stored, Some(&Health { points: 12 }));
}

#[tokio::test]
async fn a_refused_edit_travels_as_typed_diagnostics() {
    let mut host = TestHost::new();
    let entity = EntityRef::from(host.world.spawn());
    let listener = EditorListener::bind(0).await.unwrap();
    let address = listener.address().unwrap();
    let editor = async {
        let mut client = Client::new(TcpStream::connect(address).await.unwrap());
        hello(&mut client).await;
        let set = Request::SetComponent {
            entity,
            component: health_name(),
            value: Value::Bool(true),
        };
        let (response, events) = client.call(set).await;
        assert!(events.is_empty());
        assert!(matches!(
            response,
            Response::Failed {
                error: RequestError::InvalidValue { ref diagnostics }
            } if !diagnostics.is_empty()
        ));
    };
    let (served, ()) = tokio::join!(listener.serve_one(&mut host), editor);
    served.unwrap();
}

#[tokio::test]
async fn binding_a_taken_port_is_a_listen_error() {
    let first = EditorListener::bind(0).await.unwrap();
    let port = first.address().unwrap().port();
    let second = EditorListener::bind(port).await;
    assert!(matches!(second, Err(ServeError::Listen(_))));
}
