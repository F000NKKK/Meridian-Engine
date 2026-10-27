//! The UI's own input vocabulary. A later adapter above `platform-core`
//! translates OS events into [`InputEvent`]s; this crate never sees a
//! window or a backend.

mod event;
mod key;
mod modifiers;
mod outcome;
mod pointer_button;
mod ui_event;

pub use event::InputEvent;
pub use key::Key;
pub use modifiers::Modifiers;
pub use outcome::InputOutcome;
pub use pointer_button::PointerButton;
pub use ui_event::{UiEvent, UiEventKind};
