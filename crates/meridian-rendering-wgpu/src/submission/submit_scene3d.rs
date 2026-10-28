//! Records and submits one 3-D scene.

use meridian_rendering_domain::ViewUniforms;
use meridian_rendering_domain::scene::Scene3d;

use super::{PreparedScene, SceneRenderer};
use crate::{Device, RenderPass as DriverRenderPass};

/// The main-pass half of a `Scene3d` view's per-frame path: writes the
/// camera/shadow/lighting uniform and records the draws — call after
/// [`SceneRenderer::render_shadow_pass`](super::SceneRenderer::render_shadow_pass) (which supplies the `prepared` scene to pass through here unchanged) and
/// inside the caller's own already-open main render pass. Doesn't build
/// `buffers` itself (unlike the pre-shadow-mapping version of this
/// function) — see [`SceneRenderer::render_shadow_pass`](super::SceneRenderer::render_shadow_pass)'s own doc
/// comment for why baking has to happen once, before either pass opens,
/// not per-pass.
pub fn submit_scene3d(
    device: &Device,
    renderer: &SceneRenderer,
    pass: &mut DriverRenderPass<'_>,
    scene: &Scene3d,
    prepared: &PreparedScene,
) {
    renderer.set_view(device, &ViewUniforms::of_scene(scene, prepared.shadow));
    renderer.draw(pass, prepared);
}
