//! OS abstraction: window, input, filesystem, time, threading and dynamic library loading.
//!
//! `Time`/`Clock` and `InputState` are implemented and tested below — pure
//! state machines with no OS dependency, so they need no external crate.
//! `Window` is real now, backed by `winit` (see
//! [ADR 010](../../../docs/adr/010-windowing-via-winit.md) for why: correct
//! cross-platform windowing turned out to be the same class of
//! multi-month, many-independent-bug-classes undertaking that justified
//! accepting `wgpu` over hand-written GPU FFI). [`run_windowed_app`] owns
//! the OS event loop (`winit` requires this — window creation and event
//! delivery only happen inside its callback-driven model, not on demand)
//! and translates `winit`'s own event types into this crate's existing,
//! `winit`-independent [`KeyCode`]/[`MouseButton`]/[`InputState`]
//! vocabulary — callers implement [`AppHandler`] and never see a `winit`
//! type. `DynamicLibrary` is unaffected by that decision and is still a
//! stub, deliberately deferred: hand-written unsafe FFI (`dlopen`/
//! `LoadLibrary`) is still the plan for it — see docs/roadmap.md "Not yet
//! decided".

use std::collections::HashSet;
use std::sync::Arc;
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
        Self {
            threads: detect_cpu_threads(),
        }
    }
}

/// GPU-side capability info. `device_name` is real, populated by
/// `graphics-driver::Device` (the first `*-driver` crate with an actual
/// GPU backend — see that crate's module doc for the `wgpu` details);
/// `compute-driver`/`physics-driver`/`audio-driver` still report `gpu:
/// None` since none of them dispatch to a GPU yet. More fields (VRAM, max
/// workgroup size, ...) land here once a backend needs to report them —
/// additive, not a redesign of the `Option`-based shape in
/// [`BackendCapabilities`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GpuCapabilities {
    pub device_name: String,
}

/// Detects the current machine's CPU thread count. Called by
/// [`CpuCapabilities::detect`]; exposed directly too in case a caller
/// needs the thread count without a full `CpuCapabilities`.
pub fn detect_cpu_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// An OS window, backed by a real `winit::window::Window`. Only
/// constructible by [`run_windowed_app`] (`winit` only creates windows
/// inside its own event-loop callback, not on demand — see the module
/// doc), and always `Arc`-wrapped: the same handle is what
/// `graphics-driver::Device::new_windowed` needs as its surface target
/// (see [`Window::surface_target`]), so cloning a `Window` to keep
/// alongside a `Device`/`Surface` pair is cheap and correct, not a
/// separate ownership story.
#[derive(Debug, Clone)]
pub struct Window {
    inner: Arc<winit::window::Window>,
}

impl Window {
    /// Current client-area width in physical pixels.
    pub fn width(&self) -> u32 {
        self.inner.inner_size().width
    }

    /// Current client-area height in physical pixels.
    pub fn height(&self) -> u32 {
        self.inner.inner_size().height
    }

    /// Requests that the OS deliver another `RedrawRequested` event soon
    /// — [`run_windowed_app`] calls this automatically after every
    /// [`AppHandler::on_redraw`], so a continuously-rendering app (a game
    /// loop, not an idle GUI) doesn't need to call it itself.
    pub fn request_redraw(&self) {
        self.inner.request_redraw();
    }

    /// The handle `graphics-driver::Device::new_windowed` consumes to
    /// build a real `wgpu::Surface`. Returns `Arc<winit::window::Window>`
    /// directly rather than a `platform-core`-specific wrapper: `wgpu`
    /// already accepts `Arc<T>` for any `T` implementing the
    /// `raw-window-handle` traits (which `winit::window::Window` does),
    /// so `graphics-driver` never needs to depend on `winit` itself or
    /// learn about this crate's own `Window` type — see
    /// [ADR 010](../../../docs/adr/010-windowing-via-winit.md)'s
    /// "graphics-driver stays winit-agnostic" section.
    pub fn surface_target(&self) -> Arc<winit::window::Window> {
        self.inner.clone()
    }

    /// Locks the OS cursor to the window and hides it (`true`), or
    /// releases/shows it again (`false`) — the standard "free-look
    /// camera" input mode (mouse movement steers the view instead of
    /// moving a visible cursor), used by `examples::FlyCamera`. Tries
    /// `CursorGrabMode::Locked` (the cursor stays fixed in place, which
    /// is what a free-look camera actually wants) first, falling back to
    /// `Confined` (cursor can move but stays inside the window) since
    /// `Locked` isn't supported on every platform winit runs on — either
    /// way the caller only sees `Ok`/`Err`, never a `winit` type, per
    /// this crate's winit-agnostic public API (see
    /// [ADR 010](../../../docs/adr/010-windowing-via-winit.md)).
    pub fn set_cursor_grabbed(&self, grabbed: bool) {
        let mode = if grabbed {
            winit::window::CursorGrabMode::Locked
        } else {
            winit::window::CursorGrabMode::None
        };
        if self.inner.set_cursor_grab(mode).is_err() && grabbed {
            let _ = self
                .inner
                .set_cursor_grab(winit::window::CursorGrabMode::Confined);
        }
        self.inner.set_cursor_visible(!grabbed);
    }
}

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
    raw_mouse_delta: (f32, f32),
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
        self.mouse_delta = (0.0, 0.0);
        self.raw_mouse_delta = (0.0, 0.0);
    }
}

/// What a caller of [`run_windowed_app`] implements to receive OS/window
/// events, in this crate's own vocabulary — never a `winit` type. Every
/// method has a default no-op body except [`on_redraw`](Self::on_redraw):
/// that's the one callback an app genuinely can't do anything useful
/// without.
pub trait AppHandler {
    /// Called once, right after the OS window is created and before the
    /// first [`on_redraw`](Self::on_redraw) — the natural place to build
    /// GPU resources that need the window (a
    /// `graphics-driver::Device::new_windowed` surface, for instance).
    fn on_ready(&mut self, window: &Window) {
        let _ = window;
    }

    /// Called whenever the OS says it's a good time to draw a frame.
    /// [`run_windowed_app`] requests another redraw immediately after
    /// this returns and clears `input`'s per-frame transitions
    /// (equivalent to it calling [`InputState::advance_frame`] for the
    /// caller) — so this is effectively "once per frame" for a
    /// continuously-rendering app, not an occasional GUI repaint hook.
    fn on_redraw(&mut self, window: &Window, input: &InputState);

    /// Called when the window's client-area size changed (new size in
    /// physical pixels) — the point at which a caller should reconfigure
    /// its `graphics-driver::Surface`.
    fn on_resized(&mut self, width: u32, height: u32) {
        let _ = (width, height);
    }

    /// Called when the OS asked the window to close (close button,
    /// Alt+F4, ...). Returning `true` (the default) lets
    /// [`run_windowed_app`] exit; returning `false` ignores the request
    /// and keeps running.
    fn on_close_requested(&mut self) -> bool {
        true
    }
}

/// Why [`run_windowed_app`] failed.
#[derive(Debug)]
pub enum WindowError {
    /// The OS-level event loop itself couldn't start or exited abnormally.
    EventLoop(winit::error::EventLoopError),
    /// Window creation failed (e.g. no display server reachable).
    Os(winit::error::OsError),
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowError::EventLoop(e) => write!(f, "windowing event loop failed: {e}"),
            WindowError::Os(e) => write!(f, "failed to create OS window: {e}"),
        }
    }
}

impl std::error::Error for WindowError {}

/// Translates a `winit` physical key into this crate's own [`KeyCode`].
/// Deliberately partial (`None` for keys [`KeyCode`] doesn't name yet,
/// e.g. function keys, numpad, punctuation) — extending [`KeyCode`] to
/// cover more of them is additive, not a breaking change to this
/// function's shape.
fn keycode_from_winit(code: winit::keyboard::KeyCode) -> Option<KeyCode> {
    use winit::keyboard::KeyCode as Wk;
    Some(match code {
        Wk::KeyA => KeyCode::A,
        Wk::KeyB => KeyCode::B,
        Wk::KeyC => KeyCode::C,
        Wk::KeyD => KeyCode::D,
        Wk::KeyE => KeyCode::E,
        Wk::KeyF => KeyCode::F,
        Wk::KeyG => KeyCode::G,
        Wk::KeyH => KeyCode::H,
        Wk::KeyI => KeyCode::I,
        Wk::KeyJ => KeyCode::J,
        Wk::KeyK => KeyCode::K,
        Wk::KeyL => KeyCode::L,
        Wk::KeyM => KeyCode::M,
        Wk::KeyN => KeyCode::N,
        Wk::KeyO => KeyCode::O,
        Wk::KeyP => KeyCode::P,
        Wk::KeyQ => KeyCode::Q,
        Wk::KeyR => KeyCode::R,
        Wk::KeyS => KeyCode::S,
        Wk::KeyT => KeyCode::T,
        Wk::KeyU => KeyCode::U,
        Wk::KeyV => KeyCode::V,
        Wk::KeyW => KeyCode::W,
        Wk::KeyX => KeyCode::X,
        Wk::KeyY => KeyCode::Y,
        Wk::KeyZ => KeyCode::Z,
        Wk::Digit0 => KeyCode::Digit0,
        Wk::Digit1 => KeyCode::Digit1,
        Wk::Digit2 => KeyCode::Digit2,
        Wk::Digit3 => KeyCode::Digit3,
        Wk::Digit4 => KeyCode::Digit4,
        Wk::Digit5 => KeyCode::Digit5,
        Wk::Digit6 => KeyCode::Digit6,
        Wk::Digit7 => KeyCode::Digit7,
        Wk::Digit8 => KeyCode::Digit8,
        Wk::Digit9 => KeyCode::Digit9,
        Wk::Space => KeyCode::Space,
        Wk::Enter => KeyCode::Enter,
        Wk::Escape => KeyCode::Escape,
        Wk::Tab => KeyCode::Tab,
        Wk::Backspace => KeyCode::Backspace,
        Wk::ArrowUp => KeyCode::ArrowUp,
        Wk::ArrowDown => KeyCode::ArrowDown,
        Wk::ArrowLeft => KeyCode::ArrowLeft,
        Wk::ArrowRight => KeyCode::ArrowRight,
        Wk::ShiftLeft => KeyCode::ShiftLeft,
        Wk::ShiftRight => KeyCode::ShiftRight,
        Wk::ControlLeft => KeyCode::ControlLeft,
        Wk::ControlRight => KeyCode::ControlRight,
        Wk::AltLeft => KeyCode::Alt,
        _ => return None,
    })
}

/// Translates a `winit` mouse button into this crate's own
/// [`MouseButton`] — a direct, total mapping: both enums were already
/// designed with the same shape (`Left`/`Right`/`Middle`/`Back`/
/// `Forward`/`Other(u16)`), matching how X11/Windows/`winit` report
/// buttons past the first five.
fn mouse_button_from_winit(button: winit::event::MouseButton) -> MouseButton {
    match button {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Back => MouseButton::Back,
        winit::event::MouseButton::Forward => MouseButton::Forward,
        winit::event::MouseButton::Other(n) => MouseButton::Other(n),
    }
}

/// Creates an OS window titled `title` at `width`x`height` physical
/// pixels and runs the OS event loop until the window closes, dispatching
/// every event to `app` (implementing [`AppHandler`]) in this crate's own
/// vocabulary. Blocks the calling thread for as long as the app runs —
/// `winit` owns the thread its event loop runs on, the same reason a GUI
/// toolkit's `run()` call blocks; this is not a genuine-I/O `async fn`
/// (see [ADR 009](../../../docs/adr/009-async-io-via-tokio.md)'s "async
/// only on genuine I/O" scoping) because it isn't waiting on one
/// unbounded external event, it's *driving* a whole event loop's worth of
/// them.
pub fn run_windowed_app<A: AppHandler>(
    title: &str,
    width: u32,
    height: u32,
    mut app: A,
) -> Result<(), WindowError> {
    struct Runner<'a, A: AppHandler> {
        title: String,
        width: u32,
        height: u32,
        app: &'a mut A,
        window: Option<Window>,
        input: InputState,
        error: Option<WindowError>,
    }

    impl<A: AppHandler> winit::application::ApplicationHandler for Runner<'_, A> {
        fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let attributes = winit::window::WindowAttributes::default()
                .with_title(self.title.clone())
                .with_inner_size(winit::dpi::PhysicalSize::new(self.width, self.height));
            let raw_window = match event_loop.create_window(attributes) {
                Ok(w) => w,
                Err(e) => {
                    self.error = Some(WindowError::Os(e));
                    event_loop.exit();
                    return;
                }
            };
            let window = Window {
                inner: Arc::new(raw_window),
            };
            self.app.on_ready(&window);
            self.window = Some(window);
        }

        fn window_event(
            &mut self,
            event_loop: &winit::event_loop::ActiveEventLoop,
            _window_id: winit::window::WindowId,
            event: winit::event::WindowEvent,
        ) {
            let Some(window) = self.window.clone() else {
                return;
            };
            match event {
                winit::event::WindowEvent::CloseRequested => {
                    if self.app.on_close_requested() {
                        event_loop.exit();
                    }
                }
                winit::event::WindowEvent::Resized(size) => {
                    self.app.on_resized(size.width, size.height);
                }
                winit::event::WindowEvent::KeyboardInput { event, .. } => {
                    if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key
                        && let Some(key) = keycode_from_winit(code)
                    {
                        match event.state {
                            winit::event::ElementState::Pressed => self.input.press_key(key),
                            winit::event::ElementState::Released => self.input.release_key(key),
                        }
                    }
                }
                winit::event::WindowEvent::MouseInput { state, button, .. } => {
                    let button = mouse_button_from_winit(button);
                    match state {
                        winit::event::ElementState::Pressed => {
                            self.input.press_mouse_button(button)
                        }
                        winit::event::ElementState::Released => {
                            self.input.release_mouse_button(button)
                        }
                    }
                }
                winit::event::WindowEvent::CursorMoved { position, .. } => {
                    self.input
                        .set_mouse_position(position.x as f32, position.y as f32);
                }
                winit::event::WindowEvent::RedrawRequested => {
                    self.app.on_redraw(&window, &self.input);
                    self.input.advance_frame();
                    window.request_redraw();
                }
                _ => {}
            }
        }

        /// Raw device motion, not tied to cursor position — the only
        /// source `InputState::raw_mouse_delta` still gets fed from once
        /// the cursor is grabbed/locked (`Window::set_cursor_grabbed`),
        /// since a locked cursor stops generating `CursorMoved` deltas.
        fn device_event(
            &mut self,
            _event_loop: &winit::event_loop::ActiveEventLoop,
            _device_id: winit::event::DeviceId,
            event: winit::event::DeviceEvent,
        ) {
            if let winit::event::DeviceEvent::MouseMotion { delta } = event {
                self.input
                    .accumulate_mouse_motion(delta.0 as f32, delta.1 as f32);
            }
        }
    }

    let event_loop = winit::event_loop::EventLoop::new().map_err(WindowError::EventLoop)?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut runner = Runner {
        title: title.to_string(),
        width,
        height,
        app: &mut app,
        window: None,
        input: InputState::new(),
        error: None,
    };
    event_loop
        .run_app(&mut runner)
        .map_err(WindowError::EventLoop)?;
    match runner.error {
        Some(err) => Err(err),
        None => Ok(()),
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
