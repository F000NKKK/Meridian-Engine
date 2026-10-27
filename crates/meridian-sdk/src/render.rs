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
use meridian_graphics_core::{
    BloomInput, PreparedSprites, Scene3d, SceneAssets, SpriteView, batch_scene2d_clipped,
    submit_scene3d,
};
use meridian_graphics_driver::{CommandBuffer, SurfaceFrame};
use meridian_platform_core::Window;

use crate::overlay::Overlay2d;
use crate::overlay_builder::OverlayBuilder;
use crate::scene::GraphicsSession;

/// Renders one frame of `scene` into `base`'s surface; see
/// [`compose_frame`], which this calls with no overlays.
pub fn render_frame(
    base: &mut GraphicsSession,
    scene: &Scene3d,
    clear_color: [f64; 4],
    window: &Window,
) {
    compose_frame(base, scene, &[], clear_color, window);
}

/// Renders one frame of `scene` into `base`'s surface, then draws
/// `overlays` (UI, HUD) over it: records the shadow-casting depth pass
/// (`SceneRenderer::render_shadow_pass` — see that method's own doc for
/// why it has to run, as its own pass, before the main color pass opens),
/// acquires the swapchain frame (reconfiguring the surface and skipping
/// the frame on a lost/outdated swapchain instead of the "gray window"
/// failure — logged via `log_warn!`, not a silent skip), submits through
/// `graphics-core`'s submission bridge, applies `base`'s `BloomPass`,
/// draws the overlays unblurred and without depth in slice order, then
/// presents. Ends with `window.request_redraw()`, so a caller only needs
/// to call this once per redraw, last. [`RenderStage`] is this function
/// wrapped as a `Stage`, for a caller driving everything through
/// `Runtime::tick`; call this directly instead if an application isn't
/// using `Runtime` at all (e.g. `examples/magic_figures`, which has no
/// physics bodies).
pub fn compose_frame(
    base: &mut GraphicsSession,
    scene: &Scene3d,
    overlays: &[Overlay2d],
    clear_color: [f64; 4],
    window: &Window,
) {
    let mut commands = base.device.create_command_buffer();
    let prepared = base.renderer.render_shadow_pass(
        &base.device,
        &mut commands,
        scene,
        &SceneAssets {
            meshes: &base.meshes,
            materials: &base.materials,
            textures: &base.textures,
        },
    );
    let viewport = [window.width(), window.height()];
    let overlay_draws = prepare_overlays(base, overlays, viewport);

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
        submit_scene3d(&base.device, &base.renderer, &mut pass, scene, &prepared);
    }
    base.bloom.apply(
        &base.device,
        &mut commands,
        &BloomInput {
            renderer: &base.renderer,
            draw_buffers: &prepared.buffers,
            target_frame: &frame,
        },
    );
    draw_overlays(base, &mut commands, &frame, &overlay_draws);
    commands.submit();
    frame.present(&base.device);

    window.request_redraw();
}

/// Batches and uploads each overlay; must run before any pass opens.
fn prepare_overlays(
    base: &GraphicsSession,
    overlays: &[Overlay2d],
    viewport: [u32; 2],
) -> Vec<PreparedSprites> {
    overlays
        .iter()
        .map(|overlay| {
            let batches = batch_scene2d_clipped(&overlay.scene, &overlay.clips);
            let view = SpriteView {
                camera: overlay.scene.camera,
                viewport,
            };
            base.sprites
                .prepare(&base.device, view, &batches, &base.textures)
        })
        .collect()
}

/// Records every prepared overlay into one pass that keeps the 3D frame
/// underneath.
fn draw_overlays(
    base: &GraphicsSession,
    commands: &mut CommandBuffer<'_>,
    frame: &SurfaceFrame,
    overlays: &[PreparedSprites],
) {
    if overlays.is_empty() {
        return;
    }
    let mut pass = commands.begin_render_pass_loaded(frame.view());
    for prepared in overlays {
        base.sprites.draw(&mut pass, prepared);
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
    overlays: Option<OverlayBuilder>,
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
            overlays: None,
        }
    }

    /// Also draws what `build` returns over the 3D scene each tick (UI,
    /// HUD); see [`compose_frame`].
    #[must_use]
    pub fn with_overlays(
        mut self,
        build: impl FnMut(&StageContext) -> Vec<Overlay2d> + Send + 'static,
    ) -> Self {
        self.overlays = Some(OverlayBuilder::new(build));
        self
    }
}

impl<F> Stage for RenderStage<F>
where
    F: FnMut(&StageContext) -> Scene3d + Send,
{
    fn run(&mut self, ctx: &StageContext) {
        let scene = (self.build_scene)(ctx);
        let overlays = self
            .overlays
            .as_mut()
            .map_or_else(Vec::new, |builder| builder.build(ctx));
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
