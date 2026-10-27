//! [`Outcome`]: what handling one request produced.

use meridian_editor_protocol::{Event, RequestError, Response};

/// The answer to one request and the events it caused, before the session
/// filters them by subscription.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Outcome {
    pub(crate) response: Response,
    pub(crate) events: Vec<Event>,
}

impl Outcome {
    /// `response`, with no events.
    pub(crate) fn answer(response: Response) -> Self {
        Outcome {
            response,
            events: Vec::new(),
        }
    }

    /// A failed request.
    pub(crate) fn failed(error: RequestError) -> Self {
        Outcome::answer(Response::Failed { error })
    }

    /// `Done`, with no events.
    pub(crate) fn done() -> Self {
        Outcome::answer(Response::Done)
    }

    /// This outcome and `event`.
    pub(crate) fn with_event(mut self, event: Event) -> Self {
        self.events.push(event);
        self
    }
}
