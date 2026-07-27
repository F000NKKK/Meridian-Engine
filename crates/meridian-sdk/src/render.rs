//! The shadow-pass/acquire-frame/submit/present sequence a windowed
//! application needs every redraw ([`render_frame`]), and [`RenderStage`]
//! — the concrete answer to `engine-core::Runtime`'s "a render-presenting
//! `Stage` is exactly what an application implements and registers
//! itself" (see that crate's own module doc): a real [`Stage`] impl
//! wrapping [`GraphicsBase`], so presenting a frame is registered on the
//! *same* `Runtime`/`JobGraph` as physics, audio or compute stages —
//! not a second, separately-driven call an application makes outside
//! `Runtime::tick`. Depend it on whichever stage produces the state its
//! `build_scene` closure reads (typically the physics stage), and
//! `Runtime::tick` guarantees it never runs before that state is ready.

use meridian_engine_core::{Stage, StageContext};
use meridian_graphics_core::{Scene3D, submit_scene3d};
use meridian_platform_core::Window;

use crate::scene::GraphicsBase;

/// Renders one frame of `scene` into `base`'s surface: records the
/// shadow-casting depth pass (`SceneRenderer::render_shadow_pass` — see
/// that method's own doc for why it has to run, as its own pass, before
/// the main color pass opens), acquires the swapchain frame
/// (reconfiguring the surface and skipping the frame on a lost/outdated
/// swapchain instead of the "gray window" failure — logged via
/// `log_warn!`, not a silent skip), submits through `graphics-core`'s
/// submission bridge, applies `base`'s `BloomPass`, then presents. Ends
/// with `window.request_redraw()`, so a caller only needs to call this
/// once per redraw, last. [`RenderStage`] is this function wrapped as a
/// `Stage`, for a caller driving everything through `Runtime::tick`;
/// call this directly instead if an application isn't using `Runtime`
/// at all (e.g. `examples/magic_figures`, which has no physics bodies).
pub fn render_frame(
    base: &mut GraphicsBase,
    scene: &Scene3D,
    clear_color: [f64; 4],
    window: &Window,
) {
    let mut commands = base.device.create_command_buffer();
    let (draw_buffers, light_view_proj, light_direction, shadow_caster) =
        base.renderer.render_shadow_pass(
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
            meridian_foundation::log_warn!(
                "swapchain frame unavailable ({err}); reconfiguring surface"
            );
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
            light_direction,
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

/// A [`Stage`] that owns a [`GraphicsBase`] and presents a frame every
/// tick: `build_scene` reads whatever it needs from the current
/// [`StageContext`] (typically `ctx.physics().bodies` for a
/// physics-driven scene) and returns the [`Scene3D`] to draw — the same
/// split `meridian_engine_core::PhysicsStepStage` makes between "what
/// runs" (this type, generic over any scene-building closure) and "when
/// it runs" (wherever an application registers it via
/// `Runtime::add_stage`, with whatever dependencies it declares).
/// [`Stage::resize`] forwards to [`GraphicsBase::resize`], so
/// `Runtime::resize_all` keeps this stage's swapchain-sized resources
/// correct without an application needing its own separate handle to
/// reach into.
pub struct RenderStage<F> {
    base: GraphicsBase,
    window: Window,
    clear_color: [f64; 4],
    build_scene: F,
}

impl<F> RenderStage<F>
where
    F: FnMut(&StageContext) -> Scene3D + Send + 'static,
{
    pub fn new(base: GraphicsBase, window: Window, clear_color: [f64; 4], build_scene: F) -> Self {
        Self {
            base,
            window,
            clear_color,
            build_scene,
        }
    }
}

impl<F> Stage for RenderStage<F>
where
    F: FnMut(&StageContext) -> Scene3D + Send,
{
    fn run(&mut self, ctx: &StageContext) {
        let scene = (self.build_scene)(ctx);
        render_frame(&mut self.base, &scene, self.clear_color, &self.window);
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.base.resize(width, height);
    }
}
