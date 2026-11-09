//! The `winit` application handler behind [`run_windowed_app`](crate::run_windowed_app).

use winit::event::{ElementState, WindowEvent};

use crate::input::InputState;
use crate::key_map::{keycode_from_winit, mouse_button_from_winit};
use crate::window::{AppHandler, Window, WindowError};

/// Drives an [`AppHandler`] from `winit`'s callbacks, translating events
/// into this crate's input vocabulary.
pub(crate) struct EventRunner<'a, A: AppHandler> {
    pub(crate) title: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) app: &'a mut A,
    pub(crate) window: Option<Window>,
    pub(crate) input: InputState,
    pub(crate) error: Option<WindowError>,
    pub(crate) closing: bool,
}

impl<A: AppHandler> EventRunner<'_, A> {
    fn request_close(&mut self) -> bool {
        if !self.closing {
            self.closing = self.app.on_close_requested();
        }
        self.closing && self.app.ready_to_close()
    }

    fn handle_key(&mut self, event: &winit::event::KeyEvent) {
        if event.state == ElementState::Pressed
            && let Some(text) = &event.text
        {
            self.input.push_text(text);
        }
        if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key
            && let Some(key) = keycode_from_winit(code)
        {
            match event.state {
                ElementState::Pressed => self.input.press_key(key),
                ElementState::Released => self.input.release_key(key),
            }
        }
    }

    fn handle_ime(&mut self, event: &winit::event::Ime) {
        if let winit::event::Ime::Commit(text) = event {
            self.input.push_text(text);
        }
    }

    fn handle_mouse_button(&mut self, state: ElementState, button: winit::event::MouseButton) {
        let button = mouse_button_from_winit(button);
        match state {
            ElementState::Pressed => self.input.press_mouse_button(button),
            ElementState::Released => self.input.release_mouse_button(button),
        }
    }

    fn redraw(&mut self, window: &Window) {
        self.app.on_redraw(window, &self.input);
        self.input.advance_frame();
        window.request_redraw();
    }
}

impl<A: AppHandler> winit::application::ApplicationHandler for EventRunner<'_, A> {
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
            inner: std::sync::Arc::new(raw_window),
        };
        meridian_diagnostics::log_info!(
            "window created: \"{}\" ({}x{})",
            self.title,
            self.width,
            self.height
        );
        self.app.on_ready(&window);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.clone() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => {
                if self.request_close() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => self.app.on_resized(size.width, size.height),
            WindowEvent::KeyboardInput { event, .. } => self.handle_key(&event),
            WindowEvent::Ime(event) => self.handle_ime(&event),
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_button(state, button);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input
                    .set_mouse_position(position.x as f32, position.y as f32);
            }
            WindowEvent::RedrawRequested if !self.closing => self.redraw(&window),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.closing {
            if self.app.ready_to_close() {
                event_loop.exit();
            } else {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                    std::time::Instant::now() + std::time::Duration::from_millis(10),
                ));
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestApp;

    impl AppHandler for TestApp {
        fn on_redraw(&mut self, _window: &Window, _input: &InputState) {}
    }

    #[test]
    fn committed_ime_text_reaches_the_input_state() {
        let mut app = TestApp;
        let mut runner = EventRunner {
            title: "test".into(),
            width: 1,
            height: 1,
            app: &mut app,
            window: None,
            input: InputState::new(),
            error: None,
            closing: false,
        };
        runner.handle_ime(&winit::event::Ime::Commit("Привет 🌍".into()));
        assert_eq!(runner.input.text(), "Привет 🌍");
    }
}

#[cfg(test)]
mod shutdown_tests {
    use super::*;
    struct ClosingApp {
        accept: bool,
        ready: bool,
        requests: usize,
    }
    impl AppHandler for ClosingApp {
        fn on_redraw(&mut self, _: &Window, _: &InputState) {}
        fn on_close_requested(&mut self) -> bool {
            self.requests += 1;
            self.accept
        }
        fn ready_to_close(&self) -> bool {
            self.ready
        }
    }
    #[test]
    fn accepted_close_waits_for_workers_and_is_idempotent() {
        let mut app = ClosingApp {
            accept: true,
            ready: false,
            requests: 0,
        };
        let mut runner = EventRunner {
            title: String::new(),
            width: 1,
            height: 1,
            app: &mut app,
            window: None,
            input: InputState::new(),
            error: None,
            closing: false,
        };
        assert!(!runner.request_close());
        assert!(runner.closing);
        assert!(!runner.request_close());
        assert_eq!(runner.app.requests, 1);
        runner.app.ready = true;
        assert!(runner.request_close());
        assert_eq!(runner.app.requests, 1);
    }
    #[test]
    fn refused_close_keeps_processing_the_application() {
        let mut app = ClosingApp {
            accept: false,
            ready: true,
            requests: 0,
        };
        let mut runner = EventRunner {
            title: String::new(),
            width: 1,
            height: 1,
            app: &mut app,
            window: None,
            input: InputState::new(),
            error: None,
            closing: false,
        };
        assert!(!runner.request_close());
        assert!(!runner.closing);
        runner.app.accept = true;
        assert!(runner.request_close());
    }
}
