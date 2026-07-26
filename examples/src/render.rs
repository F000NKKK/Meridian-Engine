//! The acquire-frame/submit/present sequence every windowed example
//! repeats identically each redraw — the only thing that differs
//! between examples is the `Scene3D` contents and the clear color.

use meridian_sdk::{DrawBuffers, GraphicsBase, Scene3D, Window, submit_scene3d};

/// Renders one frame of `scene` into `base`'s surface: acquires the
/// swapchain frame (reconfiguring the surface and skipping the frame on
/// a lost/outdated swapchain instead of the "gray window" failure —
/// logged via `log_warn!`, not a silent skip), submits through
/// `graphics-core`'s submission bridge, applies `base`'s `BloomPass`,
/// then presents. Ends with `window.request_redraw()`, so a caller's
/// `on_redraw` only needs to call this once, last.
pub fn render_frame(
    base: &mut GraphicsBase,
    scene: &Scene3D,
    clear_color: [f64; 4],
    window: &Window,
) {
    let frame = match base.surface.acquire_frame() {
        Ok(frame) => frame,
        Err(err) => {
            meridian_sdk::log_warn!("swapchain frame unavailable ({err}); reconfiguring surface");
            base.resize(window.width(), window.height());
            return;
        }
    };

    let mut commands = base.device.create_command_buffer();
    let draw_buffers: Vec<DrawBuffers>;
    {
        let mut pass = commands.begin_render_pass(frame.view(), clear_color, Some(&base.depth));
        draw_buffers = submit_scene3d(
            &base.device,
            &base.renderer,
            &mut pass,
            scene,
            &base.meshes,
            &base.materials,
            &base.textures,
        );
    }
    base.bloom.apply(
        &base.device,
        &mut commands,
        &base.renderer,
        &draw_buffers,
        &frame,
    );
    commands.submit();
    frame.present(&base.device);

    window.request_redraw();
}
