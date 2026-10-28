//! Tests of this module.

use meridian_ecs::{GlobalTransform, Transform, World};
use meridian_gac::{Aabb, Motor3, Projection, Sphere, Vec3, mat4_mul};

use super::*;

fn transform_via_matrix(m: [[f32; 4]; 4], p: Vec3) -> Vec3 {
    Vec3::new(
        m[0][0] * p.x + m[1][0] * p.y + m[2][0] * p.z + m[3][0],
        m[0][1] * p.x + m[1][1] * p.y + m[2][1] * p.z + m[3][1],
        m[0][2] * p.x + m[1][2] * p.y + m[2][2] * p.z + m[3][2],
    )
}

fn assert_vec3_approx(a: Vec3, b: Vec3) {
    assert!((a - b).length() < 1e-4, "expected {b:?}, got {a:?}");
}

#[test]
fn identity_camera_puts_forward_point_on_negative_view_z() {
    let camera = Camera {
        frame: Motor3::identity(),
        projection: Projection::default(),
    };
    // World +X is this camera's local forward (see LOCAL_TO_VIEW_REMAP);
    // view space must see it straight ahead, on -Z, with x = y = 0.
    let view_point = transform_via_matrix(camera.view_matrix(), Vec3::new(5.0, 0.0, 0.0));
    assert_vec3_approx(view_point, Vec3::new(0.0, 0.0, -5.0));
}

#[test]
fn translated_camera_sees_world_origin_at_its_own_distance() {
    let camera = Camera {
        frame: Motor3::translation(Vec3::new(10.0, 0.0, 0.0)),
        projection: Projection::default(),
    };
    // Camera sits at world (10,0,0) facing world +X (local forward);
    // the world origin is now directly behind it, i.e. +Z in view space.
    let view_point = transform_via_matrix(camera.view_matrix(), Vec3::ZERO);
    assert_vec3_approx(view_point, Vec3::new(0.0, 0.0, 10.0));
}

#[test]
fn view_projection_matches_perspective_projection_of_view_space() {
    let camera = Camera {
        frame: Motor3::identity(),
        projection: Projection::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0),
    };
    let world_point = Vec3::new(5.0, 0.0, 0.0);
    let vp = camera.view_projection_matrix();
    let direct = mat4_mul(camera.projection.0, camera.view_matrix());
    assert_eq!(vp, direct);

    // A point straight ahead on the forward axis must land at clip
    // x = y = 0 (dead center of the screen) regardless of depth.
    let m = vp;
    let p = world_point;
    let clip = [
        m[0][0] * p.x + m[1][0] * p.y + m[2][0] * p.z + m[3][0],
        m[0][1] * p.x + m[1][1] * p.y + m[2][1] * p.z + m[3][1],
    ];
    assert!(clip[0].abs() < 1e-4 && clip[1].abs() < 1e-4);
}

fn test_frustum() -> Frustum {
    // Identity camera facing world +X, 90-degree vertical FOV, square
    // aspect, near=0.1, far=100 — an easy frustum to reason about by
    // hand: at depth d in front of the camera, the visible half-width
    // and half-height are both exactly d (tan(45 deg) == 1).
    let camera = Camera {
        frame: Motor3::identity(),
        projection: Projection::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0),
    };
    Frustum::from_view_projection(camera.view_projection_matrix())
}

#[test]
fn frustum_contains_a_box_directly_ahead() {
    let frustum = test_frustum();
    let aabb = Aabb {
        min: Vec3::new(9.0, -0.5, -0.5),
        max: Vec3::new(11.0, 0.5, 0.5),
    };
    assert!(frustum.intersects(&aabb));
}

#[test]
fn frustum_intersects_works_for_any_shape_not_just_aabb() {
    let frustum = test_frustum();
    assert!(frustum.intersects(&Sphere {
        center: Vec3::new(10.0, 0.0, 0.0),
        radius: 0.5,
    }));
    assert!(!frustum.intersects(&Sphere {
        center: Vec3::new(-10.0, 0.0, 0.0),
        radius: 0.5,
    }));
}

#[test]
fn frustum_rejects_a_box_behind_the_camera() {
    let frustum = test_frustum();
    let aabb = Aabb {
        min: Vec3::new(-11.0, -0.5, -0.5),
        max: Vec3::new(-9.0, 0.5, 0.5),
    };
    assert!(!frustum.intersects(&aabb));
}

#[test]
fn frustum_rejects_a_box_beyond_the_far_plane() {
    let frustum = test_frustum();
    let aabb = Aabb {
        min: Vec3::new(200.0, -0.5, -0.5),
        max: Vec3::new(201.0, 0.5, 0.5),
    };
    assert!(!frustum.intersects(&aabb));
}

#[test]
fn frustum_rejects_a_box_far_outside_the_side_planes() {
    let frustum = test_frustum();
    // At depth 10 the half-width is exactly 10 (90-degree FOV), so a
    // box sitting entirely beyond z=15 (local right, see
    // LOCAL_TO_VIEW_REMAP) is well outside the right plane.
    let aabb = Aabb {
        min: Vec3::new(9.0, -0.5, 15.0),
        max: Vec3::new(11.0, 0.5, 16.0),
    };
    assert!(!frustum.intersects(&aabb));
}

#[test]
fn frustum_accepts_a_box_straddling_the_right_edge() {
    let frustum = test_frustum();
    // Half-width at depth 10 is 10, so a box straddling x=10 (world
    // right) only partially crosses the boundary — its near corner
    // (in local +Z / right terms, x=9..11 here maps to depth 9..11,
    // but the straddling axis is local Z which becomes view X) must
    // still be reported visible (conservative AABB test).
    let aabb = Aabb {
        min: Vec3::new(9.0, -0.5, 8.0),
        max: Vec3::new(11.0, 0.5, 12.0),
    };
    assert!(frustum.intersects(&aabb));
}

#[test]
fn render_graph_orders_linear_producer_consumer_chain() {
    let shadow_map = GraphResourceId(0);
    let hdr_color = GraphResourceId(1);
    let ldr_color = GraphResourceId(2);

    let mut graph = RenderGraph::new();
    let lighting = graph.add_pass(
        RenderPass::new("lighting")
            .reading(shadow_map)
            .writing(hdr_color),
    );
    let shadow = graph.add_pass(RenderPass::new("shadow").writing(shadow_map));
    let tonemap = graph.add_pass(
        RenderPass::new("tonemap")
            .reading(hdr_color)
            .writing(ldr_color),
    );

    let order = graph.execution_order().unwrap();
    assert_eq!(order.len(), 3);
    let pos = |i: usize| order.iter().position(|&p| p == i).unwrap();
    assert!(
        pos(shadow) < pos(lighting),
        "shadow must run before lighting"
    );
    assert!(
        pos(lighting) < pos(tonemap),
        "lighting must run before tonemap"
    );
}

#[test]
fn render_graph_leaves_independent_passes_unordered_between_each_other() {
    let a_out = GraphResourceId(0);
    let b_out = GraphResourceId(1);

    let mut graph = RenderGraph::new();
    graph.add_pass(RenderPass::new("a").writing(a_out));
    graph.add_pass(RenderPass::new("b").writing(b_out));

    // Neither pass reads what the other writes, so any order derived
    // is valid — the graph must still terminate and include both.
    let order = graph.execution_order().unwrap();
    assert_eq!(order.len(), 2);
}

#[test]
fn render_graph_treats_externally_supplied_resources_as_available() {
    // "backbuffer" has no writer inside this graph (e.g. the
    // swapchain image) — a pass reading it must not be blocked.
    let backbuffer = GraphResourceId(0);
    let mut graph = RenderGraph::new();
    graph.add_pass(RenderPass::new("present").reading(backbuffer));
    assert_eq!(graph.execution_order().unwrap(), vec![0]);
}

#[test]
fn render_graph_rejects_two_passes_writing_the_same_resource() {
    let target = GraphResourceId(0);
    let mut graph = RenderGraph::new();
    graph.add_pass(RenderPass::new("a").writing(target));
    graph.add_pass(RenderPass::new("b").writing(target));

    let err = graph.execution_order().unwrap_err();
    assert_eq!(
        err,
        RenderGraphError::MultipleWriters {
            resource: target,
            first_writer: 0,
            second_writer: 1,
        }
    );
}

#[test]
fn render_graph_rejects_a_read_write_cycle() {
    let a_res = GraphResourceId(0);
    let b_res = GraphResourceId(1);
    let mut graph = RenderGraph::new();
    graph.add_pass(RenderPass::new("a").reading(b_res).writing(a_res));
    graph.add_pass(RenderPass::new("b").reading(a_res).writing(b_res));

    assert_eq!(
        graph.execution_order().unwrap_err(),
        RenderGraphError::Cycle
    );
}

#[test]
fn the_default_tonemap_is_aces_at_unit_exposure() {
    let config = TonemapConfig::default();
    assert_eq!(config.exposure, 1.0);
    assert_eq!(config.operator, TonemapOperator::Aces);
}

#[test]
fn tonemap_operators_have_distinct_shader_indices() {
    use crate::TonemapOperator::{Aces, Clamp, Reinhard};
    assert_eq!(
        [
            Clamp.shader_index(),
            Reinhard.shader_index(),
            Aces.shader_index()
        ],
        [0.0, 1.0, 2.0]
    );
}

#[test]
fn an_upright_look_at_never_rolls() {
    for (yaw, height) in [(0.0_f32, 0.5), (1.0, 0.7), (2.5, 0.2), (4.0, -0.4)] {
        let eye = Vec3::new(10.0 * yaw.sin(), 10.0 * height, 10.0 * yaw.cos());
        let rotor = look_at_upright(eye, Vec3::ZERO);
        let forward = rotor.transform_vector(Vec3::X);
        let right = rotor.transform_vector(Vec3::Z);
        let toward_target = (Vec3::ZERO - eye).normalize();
        assert_vec3_approx(forward, toward_target);
        assert!(right.y.abs() < 1e-4, "right axis leans: {right:?}");
    }
}

#[test]
fn light_sources_are_extracted_as_lights_at_their_entity_frame() {
    use meridian_ecs::{Transform, World};

    let mut world = World::new();
    let sun = world.spawn();
    world.insert(
        sun,
        LightSource {
            kind: LightKind::Directional,
            color: [1.0, 0.9, 0.8],
            intensity: 2.0,
        },
    );
    let lamp = world.spawn();
    world.insert(
        lamp,
        Transform::from_motor(Motor3::translation(Vec3::new(0.0, 3.0, 0.0))),
    );
    world.insert(
        lamp,
        LightSource {
            kind: LightKind::Point { range: 8.0 },
            color: [1.0; 3],
            intensity: 1.0,
        },
    );
    let lights = extract_lights(&world);
    assert_eq!(lights.len(), 2);
    let positions: Vec<f32> = lights
        .iter()
        .filter_map(|light| match light {
            Light::Point {
                position, range, ..
            } if (*range - 8.0).abs() < 1e-5 => Some(position.transform_point(Vec3::ZERO).y),
            _ => None,
        })
        .collect();
    assert_eq!(positions.len(), 1);
    assert!((positions[0] - 3.0).abs() < 1e-5);
    assert!(
        lights
            .iter()
            .any(|light| matches!(light, Light::Directional { .. }))
    );
}

#[test]
fn world_space_bounds_scales_before_placing() {
    let local = Aabb {
        min: Vec3::new(-1.0, -1.0, -1.0),
        max: Vec3::new(1.0, 1.0, 1.0),
    };
    let frame = Motor3::translation(Vec3::new(10.0, 0.0, 0.0));
    let world = world_space_bounds(&local, &frame, Vec3::new(2.0, -1.0, 0.5));
    assert_eq!(world.min, Vec3::new(8.0, -1.0, -0.5));
    assert_eq!(world.max, Vec3::new(12.0, 1.0, 0.5));
}

#[test]
fn extract_scene3d_carries_the_global_scale() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(
        entity,
        Transform {
            scale: Vec3::new(2.0, 2.0, 2.0),
            ..Transform::default()
        },
    );
    world.insert(
        entity,
        MeshRenderer {
            mesh: MeshHandle::new(meridian_memory::Handle::default()),
            material: MaterialHandle::new(meridian_memory::Handle::default()),
            billboard: false,
        },
    );
    let alone = extract_scene3d(&world, Camera::default());
    assert_eq!(alone.renderables[0].scale, Vec3::new(2.0, 2.0, 2.0));

    world.insert(
        entity,
        GlobalTransform {
            scale: Vec3::new(6.0, 6.0, 6.0),
            ..GlobalTransform::default()
        },
    );
    let resolved = extract_scene3d(&world, Camera::default());
    assert_eq!(resolved.renderables[0].scale, Vec3::new(6.0, 6.0, 6.0));
}
