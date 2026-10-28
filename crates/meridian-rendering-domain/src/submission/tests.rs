//! Tests of this module.

use meridian_gac::Vec3;

use super::*;
use crate::scene::Material;

fn triangle() -> MeshSource {
    MeshSource {
        positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        normals: vec![[0.0, 0.0, 1.0]; 3],
        uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        indices: vec![0, 1, 2],
    }
}

/// `m * vec4(p, 1)`, `m` column-major (`m[col][row]`) — the same
/// convention every matrix in this workspace uses (see
/// `Frustum::from_view_projection`'s identical row-extraction).
fn transform_point_via_matrix(m: [[f32; 4]; 4], p: Vec3) -> [f32; 4] {
    let v = [p.x, p.y, p.z, 1.0];
    let mut out = [0.0f32; 4];
    for (row, slot) in out.iter_mut().enumerate() {
        *slot = (0..4).map(|col| m[col][row] * v[col]).sum();
    }
    out
}

#[test]
fn directional_shadow_camera_orders_depth_by_distance_along_light_direction() {
    // Light shines straight down: a point above `scene_center` is
    // closer to the light than one below it.
    let light_direction = Vec3::new(0.0, -1.0, 0.0);
    let camera = directional_shadow_camera(light_direction, Vec3::ZERO);
    let vp = camera.view_projection_matrix();

    let near_clip = transform_point_via_matrix(vp, Vec3::new(0.0, 5.0, 0.0));
    let far_clip = transform_point_via_matrix(vp, Vec3::new(0.0, -5.0, 0.0));
    let near_depth = near_clip[2] / near_clip[3];
    let far_depth = far_clip[2] / far_clip[3];

    assert!(
        near_depth < far_depth,
        "a point closer to the light along its direction must have a smaller \
         depth (near {near_depth}, far {far_depth}) — matches the shadow \
         pipeline's CompareFunction::Less depth test"
    );
}

#[test]
fn directional_shadow_camera_centers_scene_center_in_clip_space() {
    let light_direction = Vec3::new(0.3, -1.0, 0.2);
    let scene_center = Vec3::new(1.0, 2.0, 3.0);
    let camera = directional_shadow_camera(light_direction, scene_center);
    let vp = camera.view_projection_matrix();

    let clip = transform_point_via_matrix(vp, scene_center);
    let ndc_x = clip[0] / clip[3];
    let ndc_y = clip[1] / clip[3];
    assert!(
        ndc_x.abs() < 1e-4,
        "scene_center must map to clip-space x=0, got {ndc_x}"
    );
    assert!(
        ndc_y.abs() < 1e-4,
        "scene_center must map to clip-space y=0, got {ndc_y}"
    );
}

#[test]
fn mesh_registry_rejects_non_triangle_index_counts() {
    let mut registry = MeshRegistry::new();
    let mut source = triangle();
    source.indices = vec![0, 1]; // not a multiple of 3
    assert_eq!(
        registry.register(source),
        Err(MeshRegistryError::NotATriangleList { index_count: 2 })
    );
}

#[test]
fn mesh_registry_takes_meshes_beyond_the_u16_vertex_range() {
    let mut registry = MeshRegistry::new();
    let count = u16::MAX as usize + 2;
    let source = MeshSource {
        positions: vec![[0.0, 0.0, 0.0]; count],
        normals: vec![[0.0, 0.0, 1.0]; count],
        uvs: vec![[0.0, 0.0]; count],
        indices: vec![0, 1, u32::try_from(count - 1).unwrap()],
    };
    assert!(registry.register(source).is_ok());
}

#[test]
fn mesh_registry_rejects_an_index_past_the_vertices() {
    let mut registry = MeshRegistry::new();
    let mut source = triangle();
    source.indices = vec![0, 1, 3];
    assert_eq!(
        registry.register(source),
        Err(MeshRegistryError::IndexOutOfRange {
            index: 3,
            vertex_count: 3
        })
    );
}

#[test]
fn mesh_registry_rejects_uv_count_mismatch() {
    let mut registry = MeshRegistry::new();
    let mut source = triangle();
    source.uvs.pop();
    assert_eq!(
        registry.register(source),
        Err(MeshRegistryError::UvCountMismatch {
            positions: 3,
            uvs: 2
        })
    );
}

#[test]
fn mesh_registry_rejects_normal_count_mismatch() {
    let mut registry = MeshRegistry::new();
    let mut source = triangle();
    source.normals.pop();
    assert_eq!(
        registry.register(source),
        Err(MeshRegistryError::NormalCountMismatch {
            positions: 3,
            normals: 2
        })
    );
}

#[test]
fn mesh_registry_round_trips_a_valid_mesh() {
    let mut registry = MeshRegistry::new();
    let handle = registry.register(triangle()).unwrap();
    assert_eq!(registry.get(handle).unwrap().indices, vec![0, 1, 2]);
    assert!(registry.unregister(handle).is_some());
    assert!(registry.get(handle).is_none());
}

#[test]
fn material_registry_round_trips() {
    let mut registry = MaterialRegistry::new();
    let material = Material {
        base_color_factor: [1.0, 0.0, 0.0, 1.0],
        ..Default::default()
    };
    let handle = registry.register(material);
    assert_eq!(
        registry.get(handle).unwrap().base_color_factor,
        [1.0, 0.0, 0.0, 1.0]
    );
    assert!(registry.unregister(handle).is_some());
    assert!(registry.get(handle).is_none());
}

fn material(blend: crate::scene::BlendMode, always_on_top: bool) -> Material {
    Material {
        always_on_top,
        blend,
        ..Material::default()
    }
}

#[test]
fn materials_pick_their_draw_state() {
    use crate::scene::BlendMode::{AlphaBlend, Opaque};
    assert_eq!(DrawState::of(&material(Opaque, false)), DrawState::Opaque);
    assert_eq!(
        DrawState::of(&material(AlphaBlend, false)),
        DrawState::Transparent
    );
    assert_eq!(
        DrawState::of(&material(Opaque, true)),
        DrawState::OnTopOpaque
    );
    assert_eq!(
        DrawState::of(&material(AlphaBlend, true)),
        DrawState::OnTopTransparent
    );
}

#[test]
fn transparent_draws_neither_write_depth_nor_hide_what_is_behind() {
    assert!(DrawState::Opaque.writes_depth());
    assert!(DrawState::Transparent.tests_depth() && !DrawState::Transparent.writes_depth());
    assert!(!DrawState::OnTopOpaque.tests_depth());
    assert!(!DrawState::OnTopTransparent.tests_depth());
}

#[test]
fn draws_go_opaque_then_far_to_near_translucent_then_on_top() {
    let draw = |state, distance_squared| OrderedDraw {
        state,
        distance_squared,
    };
    let order = order_draws(&[
        draw(DrawState::OnTopOpaque, 1.0),
        draw(DrawState::Transparent, 4.0),
        draw(DrawState::Opaque, 9.0),
        draw(DrawState::Transparent, 16.0),
        draw(DrawState::Opaque, 1.0),
        draw(DrawState::OnTopTransparent, 2.0),
    ]);
    // Opaque keep submission order, translucent run farthest first.
    assert_eq!(order, vec![2, 4, 3, 1, 0, 5]);
}

#[test]
fn equal_distances_keep_submission_order() {
    let draw = OrderedDraw {
        state: DrawState::Transparent,
        distance_squared: 5.0,
    };
    assert_eq!(order_draws(&[draw, draw, draw]), vec![0, 1, 2]);
}

#[test]
fn nothing_to_order_gives_an_empty_order() {
    assert!(order_draws(&[]).is_empty());
}

fn key(mesh: crate::MeshHandle, state: DrawState) -> BatchKey {
    BatchKey {
        mesh,
        albedo: None,
        metallic_roughness: None,
        lit: true,
        state,
        casts_shadow: true,
    }
}

fn mesh_handles() -> (crate::MeshHandle, crate::MeshHandle) {
    let mut registry = MeshRegistry::new();
    let a = registry.register(triangle()).unwrap();
    let b = registry.register(triangle()).unwrap();
    (a, b)
}

#[test]
fn opaque_draws_of_one_mesh_share_a_batch() {
    let (a, b) = mesh_handles();
    let item = |mesh, distance_squared| DrawItem {
        key: key(mesh, DrawState::Opaque),
        distance_squared,
    };
    let batches = plan_batches(&[item(a, 1.0), item(b, 2.0), item(a, 3.0), item(a, 4.0)]);
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].members, vec![0, 2, 3]);
    assert_eq!(batches[1].members, vec![1]);
}

#[test]
fn a_different_flag_splits_the_batch() {
    let (a, _) = mesh_handles();
    let base = key(a, DrawState::Opaque);
    let item = |key| DrawItem {
        key,
        distance_squared: 1.0,
    };
    let batches = plan_batches(&[
        item(base),
        item(BatchKey { lit: false, ..base }),
        item(BatchKey {
            casts_shadow: false,
            ..base
        }),
    ]);
    assert_eq!(batches.len(), 3);
}

#[test]
fn blended_draws_never_merge_and_sort_far_to_near() {
    let (a, _) = mesh_handles();
    let item = |distance_squared| DrawItem {
        key: key(a, DrawState::Transparent),
        distance_squared,
    };
    let batches = plan_batches(&[item(1.0), item(9.0), item(4.0)]);
    let order: Vec<usize> = batches.iter().map(|b| b.members[0]).collect();
    assert_eq!(order, vec![1, 2, 0]);
}

#[test]
fn batches_follow_the_draw_state_order() {
    let (a, b) = mesh_handles();
    let item = |mesh, state| DrawItem {
        key: key(mesh, state),
        distance_squared: 1.0,
    };
    let batches = plan_batches(&[
        item(a, DrawState::OnTopOpaque),
        item(b, DrawState::Transparent),
        item(a, DrawState::Opaque),
    ]);
    let states: Vec<DrawState> = batches.iter().map(|batch| batch.key.state).collect();
    assert_eq!(
        states,
        vec![
            DrawState::Opaque,
            DrawState::Transparent,
            DrawState::OnTopOpaque
        ]
    );
}

#[test]
fn no_items_plan_no_batches() {
    assert!(plan_batches(&[]).is_empty());
}
