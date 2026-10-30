//! The shadow-pass/acquire-frame/submit/present sequence a windowed
//! application needs every redraw ([`render_frame`]), and [`RenderStage`]
//! — the concrete answer to `runtime-app::Runtime`'s "a render-presenting
//! `Stage` is exactly what an application implements and registers
//! itself" (see that crate's own module doc): a real [`Stage`] impl
//! wrapping [`GraphicsSession`], so presenting a frame is registered on the
//! *same* `Runtime`/`JobGraph` as physics, audio or compute stages —
//! not a second, separately-driven call an application makes outside
//! `Runtime::tick`. Depend it on whichever stage produces the state its
//! `build_scene` closure reads (typically the physics stage), and
//! `Runtime::tick` guarantees it never runs before that state is ready.

use meridian_platform_winit::Window;
use meridian_rendering_app::FrameOutcome;
use meridian_rendering_domain::{Overlay2d, Scene3d, Viewport3d};
use meridian_runtime_app::Stage;

use crate::overlay_builder::OverlayBuilder;
use crate::scene::GraphicsSession;

/// Renders one frame of `scene` into `base`'s surface through its
/// `RenderSession`: shadow pass, main pass, bloom, present. A lost swapchain
/// reconfigures the surface to `window`'s size and skips the frame; a presented
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
    compose_frame(base, scene, &[], clear_color, window);
}

/// [`render_frame`] with `overlays` (UI, HUD) drawn over the 3D frame: the 3D
/// scene gets shadows and bloom first, then each [`Overlay2d`] is drawn
/// unblurred and without depth, in slice order, over the result.
pub fn compose_frame(
    base: &mut GraphicsSession,
    scene: &Scene3d,
    overlays: &[Overlay2d],
    clear_color: [f64; 4],
    window: &Window,
) {
    let size = (window.width(), window.height());
    let outcome = base
        .render
        .render_with_overlays(scene, overlays, clear_color, size);
    if outcome == FrameOutcome::Presented {
        window.request_redraw();
    }
}

/// [`compose_frame`] with independent 3D scenes clipped to UI rectangles.
pub fn compose_frame_with_viewports(
    base: &mut GraphicsSession,
    scene: &Scene3d,
    overlays: &[Overlay2d],
    viewports: &[Viewport3d],
    clear_color: [f64; 4],
    window: &Window,
) {
    let size = (window.width(), window.height());
    let outcome = base
        .render
        .render_with_viewports(scene, overlays, viewports, clear_color, size);
    if outcome == FrameOutcome::Presented {
        window.request_redraw();
    }
}

/// A [`Stage`] that owns a [`GraphicsSession`] and presents a frame every
/// tick: `build_scene` reads whatever it needs from the
/// [`Shared`](meridian_runtime_app::Shared) handles it captured (typically
/// the physics bodies of a physics-driven scene) and returns the
/// [`Scene3d`] to draw — the same
/// split `meridian_runtime_app::PhysicsStepStage` makes between "what
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
    overlays: Option<OverlayBuilder>,
}

impl<F> RenderStage<F>
where
    F: FnMut() -> Scene3d + Send + 'static,
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
            overlays: None,
        }
    }

    /// Also draws what `build` returns over the 3D scene each tick (UI,
    /// HUD); see [`compose_frame`].
    #[must_use]
    pub fn with_overlays(mut self, build: impl FnMut() -> Vec<Overlay2d> + Send + 'static) -> Self {
        self.overlays = Some(OverlayBuilder::new(build));
        self
    }
}

impl<F> Stage for RenderStage<F>
where
    F: FnMut() -> Scene3d + Send,
{
    fn run(&mut self) {
        let scene = (self.build_scene)();
        let overlays = self
            .overlays
            .as_mut()
            .map_or_else(Vec::new, OverlayBuilder::build);
        compose_frame(
            &mut self.base,
            &scene,
            &overlays,
            self.clear_color,
            &self.window,
        );
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.base.resize(width, height);
    }
}
