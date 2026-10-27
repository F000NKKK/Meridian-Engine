//! [`Session`]: one editor connection's state, and the dispatch of its requests.

use meridian_editor_domain::{
    Event, EventTopic, HostMessage, ProtocolVersion, Request, RequestEnvelope, RequestError,
    Response,
};

use crate::handlers::{components, entities, playback};
use crate::outcome::Outcome;
use crate::{Game, Subscriptions};

/// The state of one editor connection: the version negotiated by `Hello`
/// (none before it) and the event topics subscribed to. A session owns no
/// game state: each request borrows a [`Game`], so the host keeps the
/// `World` and decides when requests run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    host_version: ProtocolVersion,
    negotiated: Option<ProtocolVersion>,
    subscriptions: Subscriptions,
}

impl Session {
    /// A session that speaks [`ProtocolVersion::CURRENT`] and has not been
    /// greeted.
    #[must_use]
    pub fn new() -> Self {
        Session::speaking(ProtocolVersion::CURRENT)
    }

    /// A session of a host that speaks `host_version` (an older host, in
    /// tests).
    #[must_use]
    pub fn speaking(host_version: ProtocolVersion) -> Self {
        Session {
            host_version,
            negotiated: None,
            subscriptions: Subscriptions::new(),
        }
    }

    /// The version negotiated by `Hello`; `None` before it.
    #[must_use]
    pub fn negotiated(&self) -> Option<ProtocolVersion> {
        self.negotiated
    }

    /// The topics the editor subscribed to.
    #[must_use]
    pub fn subscriptions(&self) -> &Subscriptions {
        &self.subscriptions
    }

    /// Answers `envelope` against `game`: the response first, then the
    /// events the request caused that the editor subscribed to.
    pub fn handle(&mut self, envelope: RequestEnvelope, game: &mut Game<'_>) -> Vec<HostMessage> {
        let RequestEnvelope { id, request } = envelope;
        let outcome = self.dispatch(request, game);
        let response = HostMessage::Response {
            id,
            response: outcome.response,
        };
        let events = outcome.events.into_iter().filter_map(|e| self.announce(e));
        std::iter::once(response).chain(events).collect()
    }

    /// `event` as a message, when the editor subscribed to its topic. The
    /// way to announce what the host did on its own (a log line, the
    /// simulation stopping), as opposed to what a request caused.
    #[must_use]
    pub fn announce(&self, event: Event) -> Option<HostMessage> {
        self.subscriptions
            .wants(&event)
            .then_some(HostMessage::Event { event })
    }

    fn dispatch(&mut self, request: Request, game: &mut Game<'_>) -> Outcome {
        match request {
            Request::Hello { version } => self.greet(version),
            _ if self.negotiated.is_none() => Outcome::failed(RequestError::NotGreeted),
            Request::Describe => Outcome::answer(Response::Described {
                components: game.registry.describe(),
            }),
            Request::ListEntities => entities::list(game),
            Request::SpawnEntity => entities::spawn(game),
            Request::DespawnEntity { entity } => entities::despawn(game, entity),
            Request::GetComponent { entity, component } => {
                components::get(game, entity, &component)
            }
            Request::SetComponent {
                entity,
                component,
                value,
            } => components::set(game, entity, component, &value),
            Request::Play => playback::play(game),
            Request::Pause => playback::pause(game),
            Request::Step => playback::step(game),
            Request::Subscribe { topic } => self.subscription(topic, Subscriptions::subscribe),
            Request::Unsubscribe { topic } => self.subscription(topic, Subscriptions::unsubscribe),
        }
    }

    /// `Hello`: negotiates, or refuses an editor newer than the host.
    fn greet(&mut self, client: ProtocolVersion) -> Outcome {
        match self.host_version.negotiate(client) {
            Ok(version) => {
                self.negotiated = Some(version);
                Outcome::answer(Response::Welcome { version })
            }
            Err(error) => Outcome::failed(error),
        }
    }

    fn subscription(
        &mut self,
        topic: EventTopic,
        change: fn(&mut Subscriptions, EventTopic),
    ) -> Outcome {
        change(&mut self.subscriptions, topic);
        Outcome::done()
    }
}

impl Default for Session {
    fn default() -> Self {
        Session::new()
    }
}
