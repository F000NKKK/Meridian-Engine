//! The UI's own input vocabulary. A later adapter above `platform-winit`
//! translates OS events into [`InputEvent`]s; this crate never sees a
//! window or a backend.

mod event;
mod handled;
mod into_handled;
mod key;
mod modifiers;
mod outcome;
mod pointer_button;
mod ui_event;
mod ui_event_tag;

pub use event::InputEvent;
pub use handled::Handled;
pub use into_handled::IntoHandled;
pub use key::Key;
pub use modifiers::Modifiers;
pub use outcome::InputOutcome;
pub use pointer_button::PointerButton;
pub use ui_event::{UiEvent, UiEventKind};
pub use ui_event_tag::UiEventTag;
