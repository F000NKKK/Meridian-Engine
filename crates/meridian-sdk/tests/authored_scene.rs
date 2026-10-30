//! A downstream game loads authored rendering components using only the SDK facade.

use meridian_sdk::persistence::{
    ComponentName, ComponentRegistry, NoResolver, Transform, TransformCodec, World,
    register_rendering,
};
use meridian_sdk::{EditorCamera, extract_scene3d};

#[test]
fn a_game_registers_and_loads_its_startup_scene_through_the_sdk() {
    let mut registry = ComponentRegistry::new();
    registry
        .register_with::<Transform, TransformCodec>(ComponentName::new("transform").unwrap())
        .unwrap();
    register_rendering(&mut registry).unwrap();
    let mut world = World::new();
    registry
        .load_scene(
            r#"{format:1,entities:[{id:"cube",components:{
                transform:{motor:{rotor:[1,0,0,0]}},
                mesh_renderer:{mesh:"@asset:builtin/cube",material:"@asset:builtin/default"}
            }}]}"#,
            &mut world,
            &mut NoResolver,
        )
        .unwrap();
    assert_eq!(world.entities().count(), 1);
    let scene = extract_scene3d(&world, EditorCamera::default().camera(1.0));
    // Unbound asset references remain undrawn until the host binds them to GPU resources.
    assert!(scene.renderables.is_empty());
}
