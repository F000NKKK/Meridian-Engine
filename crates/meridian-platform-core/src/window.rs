//! OS windowing and the event loop, backed by `winit` (ADR 010):
//! [`Window`], [`AppHandler`] and [`run_windowed_app`], which translates
//! `winit` events into the engine's own [`input`](crate::input)
//! vocabulary — callers never see a `winit` type.

use std::sync::Arc;

use crate::input::{InputState, KeyCode, MouseButton};

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

impl meridian_foundation::EngineError for WindowError {}

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
