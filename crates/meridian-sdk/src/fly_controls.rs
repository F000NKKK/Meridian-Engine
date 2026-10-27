//! Maps the window's input state onto a free-fly camera's controls.

use meridian_platform_winit::{InputState, KeyCode};
use meridian_rendering_domain::FlyControls;

/// This frame's [`FlyControls`] from `input`: WASD to move, Space and
/// Ctrl for up and down, Shift to boost, and the device-relative mouse
/// motion to look (`raw_mouse_delta` keeps working once the caller grabs the
/// cursor, which stops position deltas entirely).
#[must_use]
pub fn fly_controls(input: &InputState) -> FlyControls {
    FlyControls {
        forward: input.is_key_down(KeyCode::W),
        backward: input.is_key_down(KeyCode::S),
        left: input.is_key_down(KeyCode::A),
        right: input.is_key_down(KeyCode::D),
        up: input.is_key_down(KeyCode::Space),
        down: input.is_key_down(KeyCode::ControlLeft),
        boost: input.is_key_down(KeyCode::ShiftLeft),
        look: input.raw_mouse_delta(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_keys_to_controls() {
        let mut input = InputState::new();
        input.press_key(KeyCode::W);
        input.press_key(KeyCode::ShiftLeft);

        let controls = fly_controls(&input);

        assert!(controls.forward && controls.boost);
        assert!(!controls.backward && !controls.left && !controls.right);
        assert!(!controls.up && !controls.down);
    }

    #[test]
    fn maps_mouse_motion_to_look() {
        let mut input = InputState::new();
        input.accumulate_mouse_motion(3.0, -2.0);

        assert_eq!(fly_controls(&input).look, (3.0, -2.0));
    }
}
