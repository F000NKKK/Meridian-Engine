//! OS abstraction: window, input, filesystem, time, threading and dynamic library loading.

/// An OS window.
#[derive(Debug)]
pub struct Window;

/// Polled keyboard/mouse/gamepad state for one frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct InputState;

/// The engine's monotonic clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct Time {
    pub delta_seconds: f64,
    pub total_seconds: f64,
}

/// A dynamically loaded library (used for hot-reload/plugins).
#[derive(Debug)]
pub struct DynamicLibrary;
