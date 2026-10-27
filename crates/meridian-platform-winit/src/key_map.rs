//! Translation of `winit` keys and mouse buttons into this crate's own.

use winit::keyboard::KeyCode as Wk;

use crate::input::{KeyCode, MouseButton};

/// Every `winit` key this crate names.
const KEY_TABLE: [(Wk, KeyCode); 50] = [
    (Wk::KeyA, KeyCode::A),
    (Wk::KeyB, KeyCode::B),
    (Wk::KeyC, KeyCode::C),
    (Wk::KeyD, KeyCode::D),
    (Wk::KeyE, KeyCode::E),
    (Wk::KeyF, KeyCode::F),
    (Wk::KeyG, KeyCode::G),
    (Wk::KeyH, KeyCode::H),
    (Wk::KeyI, KeyCode::I),
    (Wk::KeyJ, KeyCode::J),
    (Wk::KeyK, KeyCode::K),
    (Wk::KeyL, KeyCode::L),
    (Wk::KeyM, KeyCode::M),
    (Wk::KeyN, KeyCode::N),
    (Wk::KeyO, KeyCode::O),
    (Wk::KeyP, KeyCode::P),
    (Wk::KeyQ, KeyCode::Q),
    (Wk::KeyR, KeyCode::R),
    (Wk::KeyS, KeyCode::S),
    (Wk::KeyT, KeyCode::T),
    (Wk::KeyU, KeyCode::U),
    (Wk::KeyV, KeyCode::V),
    (Wk::KeyW, KeyCode::W),
    (Wk::KeyX, KeyCode::X),
    (Wk::KeyY, KeyCode::Y),
    (Wk::KeyZ, KeyCode::Z),
    (Wk::Digit0, KeyCode::Digit0),
    (Wk::Digit1, KeyCode::Digit1),
    (Wk::Digit2, KeyCode::Digit2),
    (Wk::Digit3, KeyCode::Digit3),
    (Wk::Digit4, KeyCode::Digit4),
    (Wk::Digit5, KeyCode::Digit5),
    (Wk::Digit6, KeyCode::Digit6),
    (Wk::Digit7, KeyCode::Digit7),
    (Wk::Digit8, KeyCode::Digit8),
    (Wk::Digit9, KeyCode::Digit9),
    (Wk::Space, KeyCode::Space),
    (Wk::Enter, KeyCode::Enter),
    (Wk::Escape, KeyCode::Escape),
    (Wk::Tab, KeyCode::Tab),
    (Wk::Backspace, KeyCode::Backspace),
    (Wk::ArrowUp, KeyCode::ArrowUp),
    (Wk::ArrowDown, KeyCode::ArrowDown),
    (Wk::ArrowLeft, KeyCode::ArrowLeft),
    (Wk::ArrowRight, KeyCode::ArrowRight),
    (Wk::ShiftLeft, KeyCode::ShiftLeft),
    (Wk::ShiftRight, KeyCode::ShiftRight),
    (Wk::ControlLeft, KeyCode::ControlLeft),
    (Wk::ControlRight, KeyCode::ControlRight),
    (Wk::AltLeft, KeyCode::Alt),
];

/// Translates a `winit` physical key into this crate's own [`KeyCode`].
/// Deliberately partial (`None` for keys [`KeyCode`] doesn't name yet,
/// e.g. function keys, numpad, punctuation) — extending [`KeyCode`] to
/// cover more of them is additive, not a breaking change to this
/// function's shape.
pub(crate) fn keycode_from_winit(code: Wk) -> Option<KeyCode> {
    KEY_TABLE
        .iter()
        .find(|(winit_code, _)| *winit_code == code)
        .map(|(_, key)| *key)
}

/// Translates a `winit` mouse button into this crate's own
/// [`MouseButton`] — a direct, total mapping: both enums were already
/// designed with the same shape (`Left`/`Right`/`Middle`/`Back`/
/// `Forward`/`Other(u16)`), matching how X11/Windows/`winit` report
/// buttons past the first five.
pub(crate) fn mouse_button_from_winit(button: winit::event::MouseButton) -> MouseButton {
    match button {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Back => MouseButton::Back,
        winit::event::MouseButton::Forward => MouseButton::Forward,
        winit::event::MouseButton::Other(n) => MouseButton::Other(n),
    }
}
