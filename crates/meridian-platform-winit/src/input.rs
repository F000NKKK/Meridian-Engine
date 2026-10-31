//! Engine-owned input vocabulary: keyboard/mouse codes and the
//! per-frame [`InputState`] state machine. Deliberately independent of
//! any windowing backend — `window` translates OS events into calls on
//! this API.

use std::collections::HashSet;

/// A keyboard key. Not an exhaustive enumeration of every possible key —
/// extending it is additive (new variants), not breaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// The A key.
    A,
    /// The B key.
    B,
    /// The C key.
    C,
    /// The D key.
    D,
    /// The E key.
    E,
    /// The F key.
    F,
    /// The G key.
    G,
    /// The H key.
    H,
    /// The I key.
    I,
    /// The J key.
    J,
    /// The K key.
    K,
    /// The L key.
    L,
    /// The M key.
    M,
    /// The N key.
    N,
    /// The O key.
    O,
    /// The P key.
    P,
    /// The Q key.
    Q,
    /// The R key.
    R,
    /// The S key.
    S,
    /// The T key.
    T,
    /// The U key.
    U,
    /// The V key.
    V,
    /// The W key.
    W,
    /// The X key.
    X,
    /// The Y key.
    Y,
    /// The Z key.
    Z,
    /// The top-row 0 key.
    Digit0,
    /// The top-row 1 key.
    Digit1,
    /// The top-row 2 key.
    Digit2,
    /// The top-row 3 key.
    Digit3,
    /// The top-row 4 key.
    Digit4,
    /// The top-row 5 key.
    Digit5,
    /// The top-row 6 key.
    Digit6,
    /// The top-row 7 key.
    Digit7,
    /// The top-row 8 key.
    Digit8,
    /// The top-row 9 key.
    Digit9,
    /// The space bar.
    Space,
    /// The Enter key.
    Enter,
    /// The Escape key.
    Escape,
    /// The Tab key.
    Tab,
    /// The Backspace key.
    Backspace,
    /// The up arrow key.
    ArrowUp,
    /// The down arrow key.
    ArrowDown,
    /// The left arrow key.
    ArrowLeft,
    /// The right arrow key.
    ArrowRight,
    /// The left Shift key.
    ShiftLeft,
    /// The right Shift key.
    ShiftRight,
    /// The left Control key.
    ControlLeft,
    /// The right Control key.
    ControlRight,
    /// The Alt key.
    Alt,
    /// The F3 diagnostics key.
    F3,
}

/// A mouse button. `Left`/`Right`/`Middle` plus the common `Back`/`Forward`
/// side buttons are named; anything beyond that (extra numbered buttons
/// some mice expose) is `Other(n)`, matching how X11/Windows/winit report
/// them — a raw button index, not a name, because there's no universal
/// naming past the first five.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// The primary mouse button.
    Left,
    /// The secondary mouse button.
    Right,
    /// The middle mouse button or wheel press.
    Middle,
    /// The browser-back side button.
    Back,
    /// The browser-forward side button.
    Forward,
    /// An additional platform-specific button index.
    Other(u16),
}

/// Polled keyboard/mouse state for one frame. Decoupled from any actual
/// event source: a future OS backend feeds this by calling
/// `press_key`/`release_key`/`set_mouse_position` per event and
/// `advance_frame` once per frame, but nothing here assumes how those
/// events arrive.
#[derive(Debug, Clone, Default)]
pub struct InputState {
    keys_down: HashSet<KeyCode>,
    keys_pressed_this_frame: HashSet<KeyCode>,
    keys_released_this_frame: HashSet<KeyCode>,
    mouse_buttons_down: HashSet<MouseButton>,
    mouse_buttons_pressed_this_frame: HashSet<MouseButton>,
    mouse_position: (f32, f32),
    mouse_delta: (f32, f32),
    raw_mouse_delta: (f32, f32),
    text: String,
}

impl InputState {
    /// Creates input state with no held buttons or pending transitions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `key` as held. A no-op (no transition recorded) if it was
    /// already down — matches OS key-repeat events not re-triggering
    /// `was_key_pressed`.
    pub fn press_key(&mut self, key: KeyCode) {
        if self.keys_down.insert(key) {
            self.keys_pressed_this_frame.insert(key);
        }
    }

    /// Records a key release and its one-frame transition.
    pub fn release_key(&mut self, key: KeyCode) {
        if self.keys_down.remove(&key) {
            self.keys_released_this_frame.insert(key);
        }
    }

    /// Currently held, regardless of which frame it was pressed in.
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    /// Pressed since the last [`advance_frame`](Self::advance_frame) call.
    pub fn was_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed_this_frame.contains(&key)
    }

    /// Released since the last [`advance_frame`](Self::advance_frame) call.
    pub fn was_key_released(&self, key: KeyCode) -> bool {
        self.keys_released_this_frame.contains(&key)
    }

    /// Records a mouse button as held and tracks its initial press.
    pub fn press_mouse_button(&mut self, button: MouseButton) {
        if self.mouse_buttons_down.insert(button) {
            self.mouse_buttons_pressed_this_frame.insert(button);
        }
    }

    /// Records that a mouse button is no longer held.
    pub fn release_mouse_button(&mut self, button: MouseButton) {
        self.mouse_buttons_down.remove(&button);
    }

    /// Returns whether a mouse button is currently held.
    pub fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        self.mouse_buttons_down.contains(&button)
    }

    /// Pressed since the last [`advance_frame`](Self::advance_frame) call.
    pub fn was_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed_this_frame.contains(&button)
    }

    /// Appends committed text from the OS keyboard or input method.
    pub fn push_text(&mut self, text: &str) {
        self.text.push_str(text);
    }

    /// Text committed since the last [`advance_frame`](Self::advance_frame) call.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Sets the absolute mouse position, accumulating the movement into
    /// this frame's delta.
    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_delta.0 += x - self.mouse_position.0;
        self.mouse_delta.1 += y - self.mouse_position.1;
        self.mouse_position = (x, y);
    }

    /// Returns the latest absolute cursor position.
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    /// Movement accumulated since the last
    /// [`advance_frame`](Self::advance_frame) call, derived from
    /// consecutive absolute [`set_mouse_position`](Self::set_mouse_position)
    /// calls. Not what a free-look camera wants once the cursor is
    /// grabbed/locked (a locked cursor stops moving, so this goes to
    /// zero) — see [`raw_mouse_delta`](Self::raw_mouse_delta) for that.
    pub fn mouse_delta(&self) -> (f32, f32) {
        self.mouse_delta
    }

    /// Accumulates a relative mouse-motion sample (OS-reported raw
    /// device delta, independent of cursor position — keeps working
    /// under `Window::set_cursor_grabbed(true)`, unlike
    /// [`mouse_delta`](Self::mouse_delta)). A future OS backend calls
    /// this once per raw motion event, separately from
    /// [`set_mouse_position`](Self::set_mouse_position).
    pub fn accumulate_mouse_motion(&mut self, dx: f32, dy: f32) {
        self.raw_mouse_delta.0 += dx;
        self.raw_mouse_delta.1 += dy;
    }

    /// Raw relative mouse movement accumulated since the last
    /// [`advance_frame`](Self::advance_frame) call — see
    /// [`accumulate_mouse_motion`](Self::accumulate_mouse_motion). What
    /// a free-look camera (`examples::FlyCamera`) should read.
    pub fn raw_mouse_delta(&self) -> (f32, f32) {
        self.raw_mouse_delta
    }

    /// Call once per frame after systems have read this frame's input:
    /// clears the "pressed this frame"/"released this frame" transition
    /// sets and both mouse deltas. `is_key_down`/mouse-button-down state
    /// (what's currently held) is untouched — it persists until the
    /// matching release event.
    pub fn advance_frame(&mut self) {
        self.keys_pressed_this_frame.clear();
        self.keys_released_this_frame.clear();
        self.mouse_buttons_pressed_this_frame.clear();
        self.mouse_delta = (0.0, 0.0);
        self.raw_mouse_delta = (0.0, 0.0);
        self.text.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn press_key_sets_down_and_pressed_this_frame() {
        let mut input = InputState::new();
        input.press_key(KeyCode::Space);
        assert!(input.is_key_down(KeyCode::Space));
        assert!(input.was_key_pressed(KeyCode::Space));
        assert!(!input.was_key_released(KeyCode::Space));
    }
    #[test]
    fn key_repeat_does_not_retrigger_pressed_this_frame() {
        let mut input = InputState::new();
        input.press_key(KeyCode::A);
        input.advance_frame();
        input.press_key(KeyCode::A); // still held, OS "repeat" style event
        assert!(input.is_key_down(KeyCode::A));
        assert!(!input.was_key_pressed(KeyCode::A));
    }
    #[test]
    fn advance_frame_clears_transitions_but_keeps_held_state() {
        let mut input = InputState::new();
        input.press_key(KeyCode::W);
        input.advance_frame();
        assert!(
            input.is_key_down(KeyCode::W),
            "held state must survive advance_frame"
        );
        assert!(
            !input.was_key_pressed(KeyCode::W),
            "transition must be cleared"
        );
    }
    #[test]
    fn release_key_clears_down_and_sets_released_this_frame() {
        let mut input = InputState::new();
        input.press_key(KeyCode::Enter);
        input.advance_frame();
        input.release_key(KeyCode::Enter);
        assert!(!input.is_key_down(KeyCode::Enter));
        assert!(input.was_key_released(KeyCode::Enter));
    }
    #[test]
    fn releasing_a_key_that_is_not_down_is_a_no_op() {
        let mut input = InputState::new();
        input.release_key(KeyCode::Escape);
        assert!(!input.was_key_released(KeyCode::Escape));
    }
    #[test]
    fn mouse_position_and_delta_track_movement() {
        let mut input = InputState::new();
        input.set_mouse_position(10.0, 10.0);
        assert_eq!(input.mouse_position(), (10.0, 10.0));
        // First move from the default (0,0) origin counts as a delta too.
        assert_eq!(input.mouse_delta(), (10.0, 10.0));

        input.advance_frame();
        input.set_mouse_position(15.0, 8.0);
        assert_eq!(input.mouse_position(), (15.0, 8.0));
        assert_eq!(input.mouse_delta(), (5.0, -2.0));
    }
    #[test]
    fn mouse_button_down_state() {
        let mut input = InputState::new();
        assert!(!input.is_mouse_button_down(MouseButton::Left));
        input.press_mouse_button(MouseButton::Left);
        assert!(input.is_mouse_button_down(MouseButton::Left));
        input.release_mouse_button(MouseButton::Left);
        assert!(!input.is_mouse_button_down(MouseButton::Left));
    }
    #[test]
    fn text_and_mouse_press_transitions_last_one_frame() {
        let mut input = InputState::new();
        input.push_text("Привет");
        input.press_mouse_button(MouseButton::Right);
        assert_eq!(input.text(), "Привет");
        assert!(input.was_mouse_button_pressed(MouseButton::Right));
        input.advance_frame();
        assert_eq!(input.text(), "");
        assert!(!input.was_mouse_button_pressed(MouseButton::Right));
        assert!(input.is_mouse_button_down(MouseButton::Right));
    }
    #[test]
    fn extra_numbered_mouse_buttons_are_tracked_independently() {
        let mut input = InputState::new();
        input.press_mouse_button(MouseButton::Back);
        input.press_mouse_button(MouseButton::Other(6));

        assert!(input.is_mouse_button_down(MouseButton::Back));
        assert!(input.is_mouse_button_down(MouseButton::Other(6)));
        assert!(!input.is_mouse_button_down(MouseButton::Forward));
        assert!(
            !input.is_mouse_button_down(MouseButton::Other(7)),
            "different Other indices must not alias"
        );

        input.release_mouse_button(MouseButton::Other(6));
        assert!(!input.is_mouse_button_down(MouseButton::Other(6)));
        assert!(
            input.is_mouse_button_down(MouseButton::Back),
            "releasing Other(6) must not affect Back"
        );
    }
}
