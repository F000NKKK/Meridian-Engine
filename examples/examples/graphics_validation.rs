//! Roadmap milestone: validate `meridian-graphics-core`'s driver-independent
//! half — `Camera`'s `Motor3` -> classical view/projection matrix bridge,
//! frustum culling, and render graph pass ordering — against known,
//! hand-computable scenarios. The exhaustive numeric checks live in
//! `meridian-graphics-core`'s own test suite (`cargo test -p
//! meridian-graphics-core`); this is the human-readable version.
//!
//! Run with:
//!   ./build.sh run graphics_validation

use meridian_gac_core::{Motor3, Projection, Vec3};
use meridian_graphics_core::{Aabb, Camera, Frustum, GraphResourceId, RenderGraph, RenderPass};

fn check(label: &str, condition: bool) {
    println!("  [{}] {label}", if condition { "OK" } else { "FAIL" });
    assert!(condition, "{label} failed");
}

fn main() {
    println!("== Camera: Motor3 world frame -> classical view matrix ==");
    // Camera at the world origin, facing world +X (graphics-core's local-
    // forward convention — see LOCAL_TO_VIEW_REMAP's doc comment, chosen to
    // match audio-core's listener convention).
    let camera = Camera {
        frame: Motor3::identity(),
        projection: Projection::perspective(std::f32::consts::FRAC_PI_2, 16.0 / 9.0, 0.1, 100.0),
    };
    let view = camera.view_matrix();
    let forward_point = Vec3::new(10.0, 0.0, 0.0);
    let view_space =
        |m: [[f32; 4]; 4], p: Vec3| -> Vec3 {
            Vec3::new(
                m[0][0] * p.x + m[1][0] * p.y + m[2][0] * p.z + m[3][0],
                m[0][1] * p.x + m[1][1] * p.y + m[2][1] * p.z + m[3][1],
                m[0][2] * p.x + m[1][2] * p.y + m[2][2] * p.z + m[3][2],
            )
        };
    let vp = view_space(view, forward_point);
    println!("  world (10,0,0) -> view space {vp:?}");
    check(
        "world-forward point lands on view space's -Z axis",
        (vp.x.abs() < 1e-4) && (vp.y.abs() < 1e-4) && (vp.z - -10.0).abs() < 1e-4,
    );

    println!("\n== Frustum culling: AABB vs the camera's visible volume ==");
    let frustum = Frustum::from_view_projection(camera.view_projection_matrix());
    let ahead = Aabb {
        min: Vec3::new(9.0, -0.5, -0.5),
        max: Vec3::new(11.0, 0.5, 0.5),
    };
    let behind = Aabb {
        min: Vec3::new(-11.0, -0.5, -0.5),
        max: Vec3::new(-9.0, 0.5, 0.5),
    };
    let beyond_far = Aabb {
        min: Vec3::new(200.0, -0.5, -0.5),
        max: Vec3::new(201.0, 0.5, 0.5),
    };
    check("box directly ahead is visible", frustum.intersects_aabb(ahead));
    check("box behind the camera is culled", !frustum.intersects_aabb(behind));
    check(
        "box beyond the far plane is culled",
        !frustum.intersects_aabb(beyond_far),
    );

    println!("\n== Render graph: pass order derived from resource reads/writes ==");
    let shadow_map = GraphResourceId(0);
    let hdr_color = GraphResourceId(1);
    let ldr_color = GraphResourceId(2);

    let mut graph = RenderGraph::new();
    let lighting_idx = graph.add_pass(
        RenderPass::new("lighting")
            .reading(shadow_map)
            .writing(hdr_color),
    );
    let shadow_idx = graph.add_pass(RenderPass::new("shadow").writing(shadow_map));
    let tonemap_idx = graph.add_pass(
        RenderPass::new("tonemap")
            .reading(hdr_color)
            .writing(ldr_color),
    );

    // Declared in shadow/lighting/tonemap-shuffled order above (lighting
    // added first) to prove the graph derives order from declared
    // reads/writes, not insertion order.
    let order = graph.execution_order().expect("no cycle, no write conflict");
    let names: Vec<&str> = order.iter().map(|&i| graph.passes[i].name).collect();
    println!("  declared order: [lighting, shadow, tonemap] (insertion order)");
    println!("  derived order:  {names:?}");
    let pos = |i: usize| order.iter().position(|&p| p == i).unwrap();
    check(
        "shadow runs before lighting (lighting reads what shadow writes)",
        pos(shadow_idx) < pos(lighting_idx),
    );
    check(
        "lighting runs before tonemap (tonemap reads what lighting writes)",
        pos(lighting_idx) < pos(tonemap_idx),
    );

    println!(
        "\nAll checks passed — Camera/Frustum/RenderGraph behave correctly against known scenarios."
    );
}
