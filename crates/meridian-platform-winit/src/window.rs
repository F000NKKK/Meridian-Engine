//! OS windowing and the event loop, backed by `winit` (ADR 010):
//! [`Window`], [`AppHandler`] and [`run_windowed_app`], which translates
//! `winit` events into the engine's own [`input`](crate::input)
//! vocabulary — callers never see a `winit` type.

use std::sync::Arc;

use crate::event_runner::EventRunner;
use crate::input::InputState;

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
    pub(crate) inner: Arc<winit::window::Window>,
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
            // Platforms without a locked mode get the confined one; with
            // neither, the cursor stays free and the window still works.
            if let Err(err) = self
                .inner
                .set_cursor_grab(winit::window::CursorGrabMode::Confined)
            {
                meridian_diagnostics::log_warn!("cursor grab unavailable: {err}");
            }
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
    fn on_resized(&mut self, _width: u32, _height: u32) {}

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

impl meridian_diagnostics::EngineError for WindowError {}

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
    let event_loop = winit::event_loop::EventLoop::new().map_err(WindowError::EventLoop)?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let mut runner = EventRunner {
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
