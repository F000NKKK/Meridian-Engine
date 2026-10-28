//! The window, GPU state and frame loop.

use meridian_sdk::ui_core::Size;
use meridian_sdk::ui_scene2d::UiPaint;
use meridian_sdk::{
    AppHandler, Device, GraphicsSession, InputState, Overlay2d, Scene3d, TextureStore, Window,
    compose_frame,
};

use crate::panel::Panel;

pub(crate) struct App {
    tokio_runtime: tokio::runtime::Runtime,
    start: std::time::Instant,
    panel: Panel,
    gpu: Option<GpuState>,
}

struct GpuState {
    base: GraphicsSession,
    paint: UiPaint,
}

impl App {
    pub(crate) fn new() -> Self {
        Self {
            tokio_runtime: tokio::runtime::Runtime::new().expect("failed to start tokio runtime"),
            start: std::time::Instant::now(),
            panel: Panel::new(),
            gpu: None,
        }
    }
}

impl AppHandler for App {
    fn on_ready(&mut self, window: &Window) {
        let (width, height) = (window.width(), window.height());
        let (device, surface) = self
            .tokio_runtime
            .block_on(Device::new_windowed(window.surface_target(), width, height))
            .expect("failed to create windowed GPU device");
        let mut base = GraphicsSession::new(device, surface, width, height);
        let fill = base.render.recorder.upload_solid([255; 4]);
        self.gpu = Some(GpuState {
            base,
            paint: UiPaint { fill },
        });
    }

    fn on_redraw(&mut self, window: &Window, _input: &InputState) {
        let Some(gpu) = &mut self.gpu else {
            return;
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "window sizes are far below 2^24"
        )]
        let size = Size::new(window.width() as f32, window.height() as f32);
        let ui = self
            .panel
            .extract(size, self.start.elapsed().as_secs_f32(), gpu.paint);
        let overlay = Overlay2d {
            scene: ui.scene,
            clips: ui.clips,
        };
        compose_frame(
            &mut gpu.base,
            &Scene3d::default(),
            &[overlay],
            [0.12, 0.2, 0.32, 1.0],
            window,
        );
    }

    fn on_resized(&mut self, width: u32, height: u32) {
        if let Some(gpu) = &mut self.gpu {
            gpu.base.resize(width, height);
        }
    }
}
