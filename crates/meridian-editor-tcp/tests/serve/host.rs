//! A host for the tests: a world, a registry, a simulation and a session
//! behind the `RequestHandler` port, and the client's half of the wire.

use meridian_data_substrate::ComponentRegistry;
use meridian_ecs::{Component, World};
use meridian_editor_app::{Game, ManualSimulation, RequestHandler, Session};
use meridian_editor_domain::{
    ComponentRef, HostMessage, Request, RequestEnvelope, RequestId, Response,
};
use substrate_data::Schema;
use tokio::io::{AsyncRead, AsyncWrite};

/// A component with one field.
#[derive(Debug, Clone, PartialEq, Schema)]
pub(crate) struct Health {
    pub(crate) points: i32,
}
impl Component for Health {}

pub(crate) fn health_name() -> ComponentRef {
    ComponentRef::new("health").expect("a component name")
}

pub(crate) struct TestHost {
    pub(crate) world: World,
    registry: ComponentRegistry,
    simulation: ManualSimulation,
    session: Session,
}

impl TestHost {
    pub(crate) fn new() -> Self {
        let mut registry = ComponentRegistry::new();
        registry
            .register::<Health>(health_name())
            .expect("registers");
        TestHost {
            world: World::new(),
            registry,
            simulation: ManualSimulation::new(),
            session: Session::new(),
        }
    }
}

impl RequestHandler for TestHost {
    fn handle(&mut self, request: RequestEnvelope) -> Vec<HostMessage> {
        let mut game = Game::new(&mut self.world, &self.registry, &mut self.simulation);
        self.session.handle(request, &mut game)
    }
}

/// An editor: numbers its requests and matches answers to them.
pub(crate) struct Client<S> {
    stream: S,
    next: RequestId,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Client<S> {
    pub(crate) fn new(stream: S) -> Self {
        Client {
            stream,
            next: RequestId::new(1),
        }
    }

    pub(crate) async fn send(&mut self, request: Request) -> RequestId {
        let id = self.next;
        self.next = id.next();
        let envelope = RequestEnvelope { id, request };
        meridian_editor_tcp::write_message(&mut self.stream, &envelope)
            .await
            .expect("the request is written");
        id
    }

    pub(crate) async fn next_message(&mut self) -> HostMessage {
        meridian_editor_tcp::read_message(&mut self.stream)
            .await
            .expect("a readable message")
            .expect("the host closed the stream")
    }

    /// Sends `request` and returns its response and the events that came with it.
    pub(crate) async fn call(&mut self, request: Request) -> (Response, Vec<HostMessage>) {
        let id = self.send(request).await;
        let mut events = Vec::new();
        loop {
            match self.next_message().await {
                HostMessage::Response { id: got, response } if got == id => {
                    return (response, events);
                }
                other => events.push(other),
            }
        }
    }
}
