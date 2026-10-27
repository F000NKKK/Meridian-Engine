//! The shadow-pass/acquire-frame/submit/present sequence a windowed
//! application needs every redraw ([`render_frame`]), and [`RenderStage`]
//! — the concrete answer to `engine-core::Runtime`'s "a render-presenting
//! `Stage` is exactly what an application implements and registers
//! itself" (see that crate's own module doc): a real [`Stage`] impl
//! wrapping [`GraphicsSession`], so presenting a frame is registered on the
//! *same* `Runtime`/`JobGraph` as physics, audio or compute stages —
//! not a second, separately-driven call an application makes outside
//! `Runtime::tick`. Depend it on whichever stage produces the state its
//! `build_scene` closure reads (typically the physics stage), and
//! `Runtime::tick` guarantees it never runs before that state is ready.

use meridian_engine_core::{Stage, StageContext};
use meridian_graphics_core::{FrameOutcome, FrameRecorder, FrameRequest, Scene3d};
use meridian_platform_core::Window;

use crate::scene::GraphicsSession;

/// Renders one frame of `scene` into `base`'s surface through its
/// [`FrameRecorder`]: shadow pass, main pass, bloom, present (see
/// `GpuFrameRecorder::record_frame`). A lost swapchain reconfigures the surface
/// to `window`'s size and skips the frame (the recorder logs it); a presented
/// frame ends with `window.request_redraw()`, so a caller only needs to call this
/// once per redraw, last. [`RenderStage`] is this function wrapped as a `Stage`,
/// for a caller driving everything through `Runtime::tick`; call this directly
/// instead if an application isn't using `Runtime` at all (e.g.
/// `examples/magic_figures`, which has no physics bodies).
pub fn render_frame(
    base: &mut GraphicsSession,
    scene: &Scene3d,
    clear_color: [f64; 4],
    window: &Window,
) {
    let request = FrameRequest {
        scene,
        meshes: &base.meshes,
        materials: &base.materials,
        clear_color,
    };
    match base.gpu.record_frame(&request) {
        FrameOutcome::Presented => window.request_redraw(),
        FrameOutcome::TargetUnavailable => base.resize(window.width(), window.height()),
        _ => {}
    }
}

/// A [`Stage`] that owns a [`GraphicsSession`] and presents a frame every
/// tick: `build_scene` reads whatever it needs from the current
/// [`StageContext`] (typically `ctx.physics().bodies` for a
/// physics-driven scene) and returns the [`Scene3d`] to draw — the same
/// split `meridian_engine_core::PhysicsStepStage` makes between "what
/// runs" (this type, generic over any scene-building closure) and "when
/// it runs" (wherever an application registers it via
/// `Runtime::add_stage`, with whatever dependencies it declares).
/// [`Stage::resize`] forwards to [`GraphicsSession::resize`], so
/// `Runtime::resize_all` keeps this stage's swapchain-sized resources
/// correct without an application needing its own separate handle to
/// reach into.
#[derive(Debug)]
pub struct RenderStage<F> {
    base: GraphicsSession,
    window: Window,
    clear_color: [f64; 4],
    build_scene: F,
}

impl<F> RenderStage<F>
where
    F: FnMut(&StageContext) -> Scene3d + Send + 'static,
{
    pub fn new(
        base: GraphicsSession,
        window: Window,
        clear_color: [f64; 4],
        build_scene: F,
    ) -> Self {
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
    F: FnMut(&StageContext) -> Scene3d + Send,
{
    fn run(&mut self, ctx: &StageContext) {
        let scene = (self.build_scene)(ctx);
        render_frame(&mut self.base, &scene, self.clear_color, &self.window);
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.base.resize(width, height);
    }
}
