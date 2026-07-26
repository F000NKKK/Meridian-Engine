//! The shadow-pass/acquire-frame/submit/present sequence every windowed
//! example repeats identically each redraw — the only thing that
//! differs between examples is the `Scene3D` contents and the clear
//! color.

use meridian_sdk::{GraphicsBase, Scene3D, Window, submit_scene3d};

/// Renders one frame of `scene` into `base`'s surface: records the
/// shadow-casting depth pass (`SceneRenderer::render_shadow_pass` — see
/// that method's own doc for why it has to run, as its own pass, before
/// the main color pass opens), acquires the swapchain frame
/// (reconfiguring the surface and skipping the frame on a lost/outdated
/// swapchain instead of the "gray window" failure — logged via
/// `log_warn!`, not a silent skip), submits through `graphics-core`'s
/// submission bridge, applies `base`'s `BloomPass`, then presents. Ends
/// with `window.request_redraw()`, so a caller's `on_redraw` only needs
/// to call this once, last.
pub fn render_frame(
    base: &mut GraphicsBase,
    scene: &Scene3D,
    clear_color: [f64; 4],
    window: &Window,
) {
    let mut commands = base.device.create_command_buffer();
    let (draw_buffers, light_view_proj, shadow_caster) = base.renderer.render_shadow_pass(
        &base.device,
        &mut commands,
        scene,
        &base.meshes,
        &base.materials,
        &base.textures,
    );

    let frame = match base.surface.acquire_frame() {
        Ok(frame) => frame,
        Err(err) => {
            meridian_sdk::log_warn!("swapchain frame unavailable ({err}); reconfiguring surface");
            base.resize(window.width(), window.height());
            return;
        }
    };

    {
        let mut pass = commands.begin_render_pass(frame.view(), clear_color, Some(&base.depth));
        submit_scene3d(
            &base.device,
            &base.renderer,
            &mut pass,
            scene,
            light_view_proj,
            shadow_caster,
            &draw_buffers,
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
