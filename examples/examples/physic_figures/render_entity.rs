//! An entity as the renderer sees it.

use meridian_sdk::{ColliderShape, MaterialHandle, MeshHandle, Motor3, Vec3};

/// This shape's rendered frame, given its current physics `body_frame`
/// — identical for most shapes, except two disclosed per-shape offsets:
/// the floor mesh is a thin visual quad at `y = 0`, rendered at
/// identity regardless of its (thicker, lower) collider's frame; the
/// pyramid mesh's origin is its own base center, but its `Cuboid`
/// collider is centered on the body, so the mesh renders shifted down
/// by the collider's half-height to keep the base flush with the
/// resting contact point. Takes `mesh_shape`/`collider` directly
/// (rather than a whole `&SceneEntity`) so it works equally for the
/// load-time [`SceneEntity`](crate::SceneEntity) list and the [`RenderEntity`] list the
/// render stage's closure actually reads from.
pub(crate) fn mesh_render_frame(
    mesh_shape: &str,
    collider: ColliderShape,
    body_frame: Motor3,
) -> Motor3 {
    match mesh_shape {
        "ground" => Motor3::identity(),
        "pyramid" => {
            let ColliderShape::Cuboid { half_extents } = collider else {
                panic!("pyramid entity must have a cuboid collider");
            };
            body_frame.compose(Motor3::translation(Vec3::new(0.0, -half_extents.y, 0.0)))
        }
        _ => body_frame,
    }
}

/// One [`SceneEntity`](crate::SceneEntity)'s registered GPU handles plus the bits
/// [`mesh_render_frame`] needs — built once in [`AppHandler::on_ready`],
/// moved into the render stage's `build_scene` closure so it never
/// re-registers a mesh/material or re-reads `scene_entities` per frame.
pub(crate) struct RenderEntity {
    pub(crate) mesh_shape: String,
    pub(crate) collider: ColliderShape,
    pub(crate) mesh: MeshHandle,
    pub(crate) material: MaterialHandle,
}
