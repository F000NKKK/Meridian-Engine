//! Real rigid-body physics: a sphere, a cube and a pyramid dropped above
//! a textured floor and stepped every frame through a real
//! `meridian_sdk::Runtime` (`engine-core`'s single `JobGraph`-based
//! frame-work entry point — see that crate's own module doc). Physics
//! *and* rendering both go through this one `Runtime`: a registered
//! `meridian_sdk::PhysicsStepStage` in `PhysicsDispatchMode::Batched`
//! (not `Sequential` — see that type's own doc comment for why this is
//! a proven drop-in swap, and docs/roadmap.md's "Scaling beyond a tech
//! demo" item 4 for the motivation) driven by the fixed-timestep
//! accumulator (see
//! [`PhysicsRig::step`]) and a registered `meridian_sdk::RenderStage`
//! (driven once per redraw, depending on the physics stage) — no raw,
//! runtime-bypassing pipeline call for either. See
//! `Runtime::tick_only`'s own doc comment for why physics and render
//! need *selective* ticks (different multiplicities per real display
//! frame) rather than one `Runtime::tick()` running both together every
//! time.
//!
//! Design notes: docs/notes/meridian-examples.md (`examples/physic_figures`).

mod app;
mod physics_rig;
mod render_entity;
mod scene_entity;
mod sun;

use app::App;
use meridian_sdk::run_windowed_app;
pub(crate) use physics_rig::PhysicsRig;
pub(crate) use render_entity::{RenderEntity, mesh_render_frame};
pub(crate) use scene_entity::{SceneEntity, load_scene, mesh_source_for};
pub(crate) use sun::{SUN_DIRECTION, sun_renderable};

fn main() {
    meridian_examples::app_main::install_diagnostics("physic_figures");
    run_windowed_app("Meridian Engine — Physic Figures", 1024, 768, App::new())
        .expect("windowed app exited with an error");
}
