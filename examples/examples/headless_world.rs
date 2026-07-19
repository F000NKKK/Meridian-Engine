//! Roadmap milestone: a headless integration example tying together every
//! crate implemented so far (steps 1-5) — not one at a time like
//! gac_validation, but cooperating: `ecs-core` entities holding a
//! `gac-core` `Transform`, integrated by a `platform-core` `Clock`;
//! `resource-core` tracking a mesh/material dependency over handles from a
//! `memory-core` `ResourcePool`; a `task-core` job graph running a small
//! dependency-ordered "frame". This is deliberately not a real engine loop
//! (no window, no rendering) — the point is proving these pieces actually
//! fit together before `engine-core` has to wire them into one.
//!
//! Run with:
//!   ./build.sh run headless_world

use std::sync::{Arc, Mutex};

use meridian_ecs_core::{Component, Entity, Transform, World};
use meridian_gac_core::{Motor3, Vec3};
use meridian_memory_core::ResourcePool;
use meridian_platform_core::{Clock, InputState, KeyCode};
use meridian_resource_core::{DependencyGraph, ResourceDependency, ResourceId};
use meridian_task_core::{JobGraph, Scheduler};

#[derive(Debug, Clone, Copy)]
struct Velocity(Vec3);
impl Component for Velocity {}

struct MeshData {
    name: &'static str,
}
struct MaterialData {
    name: &'static str,
}

fn check(label: &str, condition: bool) {
    println!("  [{}] {label}", if condition { "OK" } else { "FAIL" });
    assert!(condition, "{label} failed");
}

fn main() {
    println!("== ecs-core: spawn entities, gac-core Transform + Velocity ==");
    let mut world = World::new();
    let mut entities = Vec::new();
    for i in 0..5 {
        let e = world.spawn();
        world.insert(
            e,
            Transform {
                motor: Motor3::translation(Vec3::new(i as f32, 0.0, 0.0)),
            },
        );
        world.insert(e, Velocity(Vec3::new(1.0, 0.0, 0.0)));
        entities.push(e);
    }
    check(
        "5 entities alive",
        entities.iter().all(|&e| world.is_alive(e)),
    );
    check(
        "all 5 have Transform",
        world.query::<Transform>().count() == 5,
    );

    println!("\n== platform-core: Clock ticks, integrate motion via ecs-core queries ==");
    let mut clock = Clock::new();
    let dt = clock.tick();
    println!(
        "  dt = {:.6}s (first tick, near-zero is expected)",
        dt.delta_seconds
    );

    // No multi-component query yet (see ecs-core's module doc) — collect
    // Velocity first, then look up each entity's Transform individually.
    let velocities: Vec<(Entity, Vec3)> =
        world.query::<Velocity>().map(|(e, v)| (e, v.0)).collect();
    for (entity, velocity) in velocities {
        if let Some(transform) = world.get_mut::<Transform>(entity) {
            let step = Motor3::translation(velocity * dt.delta_seconds as f32);
            transform.motor = transform.motor.compose(step);
        }
    }
    let moved_position = world
        .get::<Transform>(entities[0])
        .unwrap()
        .motor
        .transform_point(Vec3::ZERO);
    println!("  entity 0 world position after one tick: {moved_position:?}");

    println!("\n== memory-core + resource-core: pools, handles, dependency tracking ==");
    let mut meshes: ResourcePool<MeshData> = ResourcePool::new();
    let mesh_handle = meshes.insert(MeshData { name: "cube.mesh" });
    let mesh_id: ResourceId<MeshData> = ResourceId::new(mesh_handle);

    let mut materials: ResourcePool<MaterialData> = ResourcePool::new();
    let material_handle = materials.insert(MaterialData { name: "cube.mat" });
    let material_id: ResourceId<MaterialData> = ResourceId::new(material_handle);

    let mut deps = DependencyGraph::new();
    deps.add_dependency(ResourceDependency::new(material_id, mesh_id));
    println!(
        "  {} depends on {}: {}",
        materials.get(material_handle).unwrap().name,
        meshes.get(mesh_handle).unwrap().name,
        deps.depends_on(material_id.handle, mesh_id.handle)
    );
    check(
        "material -> mesh dependency recorded",
        deps.depends_on(material_id.handle, mesh_id.handle),
    );
    check(
        "mesh -> material would close a cycle",
        deps.would_cycle(mesh_id.handle, material_id.handle),
    );

    println!(
        "\n== task-core: dependency-ordered job graph (Input -> {{Physics, Audio}} -> Render) =="
    );
    let log: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
    let mut graph = JobGraph::new();

    let l = log.clone();
    let input = graph.add_job("input", &[], move || l.lock().unwrap().push("input"));
    let l = log.clone();
    let physics = graph.add_job("physics", &[input], move || {
        l.lock().unwrap().push("physics")
    });
    let l = log.clone();
    let audio = graph.add_job("audio", &[input], move || l.lock().unwrap().push("audio"));
    let l = log.clone();
    graph.add_job("render", &[physics, audio], move || {
        l.lock().unwrap().push("render")
    });

    Scheduler::new(4).run(graph);
    let order = log.lock().unwrap().clone();
    println!("  ran in order: {order:?}");
    check("input ran first", order.first() == Some(&"input"));
    check("render ran last", order.last() == Some(&"render"));
    check(
        "physics and audio both ran (order between them isn't guaranteed)",
        order.len() == 4,
    );

    println!("\n== platform-core: InputState press/held/advance_frame ==");
    let mut input_state = InputState::new();
    input_state.press_key(KeyCode::W);
    check("W held", input_state.is_key_down(KeyCode::W));
    check(
        "W pressed this frame",
        input_state.was_key_pressed(KeyCode::W),
    );
    input_state.advance_frame();
    check(
        "W still held after advance_frame",
        input_state.is_key_down(KeyCode::W),
    );
    check(
        "W no longer 'pressed this frame' after advance_frame",
        !input_state.was_key_pressed(KeyCode::W),
    );

    println!("\nAll checks passed — steps 1-5 cooperate correctly.");
}
