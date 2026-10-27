//! [`Handled`]: whether an event handler consumed the event.

/// The answer of a handler that takes part in routing. `Yes` stops the
/// event from reaching the handlers further along its route (WinUI's
/// `e.Handled = true`, as a typed return value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Handled {
    /// Keep routing.
    #[default]
    No,
    /// The event is consumed.
    Yes,
}
