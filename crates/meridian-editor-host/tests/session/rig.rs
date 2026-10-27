//! A host for the tests: a world, a registry, a simulation and a session.

use meridian_ecs_core::{Component, World};
use meridian_editor_host::{Game, ManualSimulation, Session};
use meridian_editor_protocol::{
    ComponentRef, EntityRef, EventTopic, HostMessage, ProtocolVersion, Request, RequestEnvelope,
    RequestId, Response,
};
use meridian_scene::ComponentRegistry;
use substrate_data::Schema;

/// A component with one field.
#[derive(Debug, Clone, PartialEq, Schema)]
pub(crate) struct Health {
    pub(crate) points: i32,
}
impl Component for Health {}

pub(crate) fn health_name() -> ComponentRef {
    ComponentRef::new("health").expect("a component name")
}

pub(crate) struct Rig {
    pub(crate) world: World,
    pub(crate) registry: ComponentRegistry,
    pub(crate) simulation: ManualSimulation,
    pub(crate) session: Session,
    next: RequestId,
}

impl Rig {
    /// A rig whose session has not said `Hello`.
    pub(crate) fn new() -> Self {
        let mut registry = ComponentRegistry::new();
        registry
            .register::<Health>(health_name())
            .expect("registers");
        Rig {
            world: World::new(),
            registry,
            simulation: ManualSimulation::new(),
            session: Session::new(),
            next: RequestId::new(1),
        }
    }

    /// A rig whose session has said `Hello` and subscribed to `topics`.
    pub(crate) fn greeted(topics: &[EventTopic]) -> Self {
        let mut rig = Rig::new();
        let welcome = rig.respond(Request::Hello {
            version: ProtocolVersion::CURRENT,
        });
        assert_eq!(
            welcome,
            Response::Welcome {
                version: ProtocolVersion::CURRENT
            }
        );
        for &topic in topics {
            assert_eq!(rig.respond(Request::Subscribe { topic }), Response::Done);
        }
        rig
    }

    /// Sends `request` and returns everything the host sent back.
    pub(crate) fn send(&mut self, request: Request) -> Vec<HostMessage> {
        let id = self.next;
        self.next = id.next();
        let mut game = Game::new(&mut self.world, &self.registry, &mut self.simulation);
        self.session
            .handle(RequestEnvelope { id, request }, &mut game)
    }

    /// Sends `request` and returns its response, ignoring events.
    pub(crate) fn respond(&mut self, request: Request) -> Response {
        let first = self.send(request).into_iter().next();
        match first {
            Some(HostMessage::Response { response, .. }) => Some(response),
            _ => None,
        }
        .expect("the first message is the response")
    }

    /// Spawns an entity through the protocol.
    pub(crate) fn spawn(&mut self) -> EntityRef {
        match self.respond(Request::SpawnEntity) {
            Response::Spawned { entity } => Some(entity),
            _ => None,
        }
        .expect("SpawnEntity answers Spawned")
    }

    /// Lists entities through the protocol.
    pub(crate) fn entities(&mut self) -> Vec<EntityRef> {
        match self.respond(Request::ListEntities) {
            Response::Entities { entities } => Some(entities),
            _ => None,
        }
        .expect("ListEntities answers Entities")
    }
}
