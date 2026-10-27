//! Tests of this module.

use meridian_gac_core::{Aabb, Motor3, Projection, Sphere, Vec3, mat4_mul};

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
