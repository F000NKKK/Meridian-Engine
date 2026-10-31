//! Records and submits one 3-D scene.

use super::{PreparedScene, SceneRenderer};
use crate::RenderPass as DriverRenderPass;

/// Records a prepared scene inside the caller's already-open main render pass.
/// Its camera and lighting bindings belong to `prepared`, so another view
/// queued into the same command buffer cannot replace them.
pub fn submit_scene3d(
    renderer: &SceneRenderer,
    pass: &mut DriverRenderPass<'_>,
    prepared: &PreparedScene,
) {
    renderer.draw(pass, prepared);
}
