//! The `winit` application handler behind [`run_windowed_app`](crate::run_windowed_app).

use crate::input::InputState;
use crate::key_map::{keycode_from_winit, mouse_button_from_winit};
use crate::window::{AppHandler, Window, WindowError};
use winit::event::{ElementState, WindowEvent};

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
}

impl<A: AppHandler> EventRunner<'_, A> {
    fn handle_key(&mut self, event: &winit::event::KeyEvent) {
        if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key
            && let Some(key) = keycode_from_winit(code)
        {
            match event.state {
                ElementState::Pressed => self.input.press_key(key),
                ElementState::Released => self.input.release_key(key),
            }
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
        meridian_foundation::log_info!(
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
            WindowEvent::CloseRequested if self.app.on_close_requested() => event_loop.exit(),
            WindowEvent::Resized(size) => self.app.on_resized(size.width, size.height),
            WindowEvent::KeyboardInput { event, .. } => self.handle_key(&event),
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_button(state, button);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input
                    .set_mouse_position(position.x as f32, position.y as f32);
            }
            WindowEvent::RedrawRequested => self.redraw(&window),
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
