//! OS abstraction: window, input, filesystem, time, threading and dynamic library loading.
//!
//! `Time`/`Clock` and `InputState` are implemented and tested below — pure
//! state machines with no OS dependency, so they need no external crate.
//! `Window` and `DynamicLibrary` are still stubs, deliberately deferred
//! until `graphics-driver` (step 8) actually needs them — see
//! docs/roadmap.md "Not yet decided". Decision already made for when that
//! happens: hand-written unsafe FFI (`dlopen`/`LoadLibrary`, per-platform
//! window creation), not an external crate like `winit`/`libloading` — the
//! workspace stays at zero external dependencies.

use std::collections::HashSet;
use std::time::Instant;

/// Backend capability reporting shared by every `*-driver` crate's own
/// capability type (`compute-driver::ComputeCapabilities`,
/// `physics-driver::PhysicsBackend`, and future `graphics-driver`/
/// `audio-driver` equivalents). CPU and GPU are deliberately separate
/// capability shapes, not one bag of fields with a `gpu: bool` flag: CPU
/// capability is "how many threads" (a number, always present); GPU
/// capability is a completely different shape (device name, VRAM,
/// workgroup limits, ...) that either doesn't exist yet (no backend) or
/// exists with real detected fields (once one does) — `Option<GpuCapabilities>`
/// models that directly instead of a bool that would need a parallel
/// struct bolted on later. When a real GPU backend lands (planned:
/// `wgpu`, not hand-written FFI, unlike `Window`/`DynamicLibrary` — see
/// docs/roadmap.md), it fills in [`GpuCapabilities`]'s fields; no
/// restructuring of this trait or [`CpuCapabilities`] is needed.
pub trait BackendCapabilities {
    fn cpu(&self) -> CpuCapabilities;
    fn gpu(&self) -> Option<GpuCapabilities>;

    fn gpu_available(&self) -> bool {
        self.gpu().is_some()
    }
}

/// CPU-side capability info every `*-driver` backend reports. Each
/// driver's own capability type embeds this (`pub cpu: CpuCapabilities`)
/// instead of redeclaring `threads` itself, and adds its own
/// domain-specific fields (SIMD width, audio latency, ...) alongside it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CpuCapabilities {
    pub threads: usize,
}

impl CpuCapabilities {
    pub fn detect() -> Self {
        Self { threads: detect_cpu_threads() }
    }
}

/// GPU-side capability info. Empty for now — nothing in the workspace
/// constructs a `Some(GpuCapabilities { .. })` anywhere yet, since no
/// `*-driver` crate has a real GPU backend (see `compute-driver`'s module
/// doc). Real fields (device name, VRAM, max workgroup size, ...) land
/// here once a backend exists to report them; adding fields to this
/// struct then is additive, not a redesign of the `Option`-based shape
/// above it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GpuCapabilities {}

/// Detects the current machine's CPU thread count. Called by
/// [`CpuCapabilities::detect`]; exposed directly too in case a caller
/// needs the thread count without a full `CpuCapabilities`.
pub fn detect_cpu_threads() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

/// An OS window. Not yet implemented — see the module doc.
#[derive(Debug)]
pub struct Window;

/// A keyboard key. Not an exhaustive enumeration of every possible key —
/// extending it is additive (new variants), not breaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Space,
    Enter,
    Escape,
    Tab,
    Backspace,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    Alt,
}

/// A mouse button. `Left`/`Right`/`Middle` plus the common `Back`/`Forward`
/// side buttons are named; anything beyond that (extra numbered buttons
/// some mice expose) is `Other(n)`, matching how X11/Windows/winit report
/// them — a raw button index, not a name, because there's no universal
/// naming past the first five.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
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
    mouse_position: (f32, f32),
    mouse_delta: (f32, f32),
}

impl InputState {
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

    pub fn press_mouse_button(&mut self, button: MouseButton) {
        self.mouse_buttons_down.insert(button);
    }

    pub fn release_mouse_button(&mut self, button: MouseButton) {
        self.mouse_buttons_down.remove(&button);
    }

    pub fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        self.mouse_buttons_down.contains(&button)
    }

    /// Sets the absolute mouse position, accumulating the movement into
    /// this frame's delta.
    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_delta.0 += x - self.mouse_position.0;
        self.mouse_delta.1 += y - self.mouse_position.1;
        self.mouse_position = (x, y);
    }

    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    /// Movement accumulated since the last
    /// [`advance_frame`](Self::advance_frame) call.
    pub fn mouse_delta(&self) -> (f32, f32) {
        self.mouse_delta
    }

    /// Call once per frame after systems have read this frame's input:
    /// clears the "pressed this frame"/"released this frame" transition
    /// sets and the mouse delta. `is_key_down`/mouse-button-down state
    /// (what's currently held) is untouched — it persists until the
    /// matching release event.
    pub fn advance_frame(&mut self) {
        self.keys_pressed_this_frame.clear();
        self.keys_released_this_frame.clear();
        self.mouse_delta = (0.0, 0.0);
    }
}

/// A snapshot of the engine's monotonic clock for one frame: how long the
/// previous frame took, and how long the clock has been running in total.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Time {
    pub delta_seconds: f64,
    pub total_seconds: f64,
}

/// Produces [`Time`] snapshots from `std::time::Instant`. `delta_seconds`
/// is wall-clock time since the previous [`tick`](Self::tick), not a fixed
/// step — a fixed-step accumulator for deterministic simulation is a
/// separate concern, tracked in docs/roadmap.md, not this type's job.
#[derive(Debug)]
pub struct Clock {
    start: Instant,
    last_tick: Instant,
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start: now,
            last_tick: now,
        }
    }

    /// Advances the clock to "now" and returns the elapsed/total time.
    pub fn tick(&mut self) -> Time {
        let now = Instant::now();
        let delta_seconds = now.duration_since(self.last_tick).as_secs_f64();
        let total_seconds = now.duration_since(self.start).as_secs_f64();
        self.last_tick = now;
        Time {
            delta_seconds,
            total_seconds,
        }
    }
}

/// A dynamically loaded library (used for hot-reload/plugins). Not yet
/// implemented — see the module doc.
#[derive(Debug)]
pub struct DynamicLibrary;

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn detect_cpu_threads_reports_at_least_one() {
        assert!(detect_cpu_threads() >= 1);
    }

    #[test]
    fn cpu_capabilities_detect_reports_at_least_one_thread() {
        let caps = CpuCapabilities::detect();
        assert!(caps.threads >= 1);
    }

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

    #[test]
    fn clock_delta_and_total_advance_monotonically() {
        let mut clock = Clock::new();
        sleep(Duration::from_millis(10));
        let t1 = clock.tick();
        assert!(
            t1.delta_seconds >= 0.005,
            "delta should reflect the sleep, got {}",
            t1.delta_seconds
        );
        assert!(t1.total_seconds >= t1.delta_seconds);

        sleep(Duration::from_millis(10));
        let t2 = clock.tick();
        assert!(t2.delta_seconds >= 0.005);
        assert!(
            t2.total_seconds > t1.total_seconds,
            "total must keep accumulating"
        );
    }
}
