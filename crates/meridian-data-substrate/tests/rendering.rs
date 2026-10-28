//! The rendering components register and convert to their file forms.

use meridian_data_substrate::{
    ComponentRegistry, EntityKey, LightForm, MeshReference, NoResolver, register_rendering,
};
use meridian_ecs::World;
use meridian_rendering_domain::{LightKind, LightSource};

fn spot() -> LightSource {
    LightSource {
        kind: LightKind::Spot {
            range: 12.0,
            inner_angle: 0.3,
            outer_angle: 0.5,
        },
        color: [1.0, 0.8, 0.6],
        intensity: 4.0,
    }
}

#[test]
fn a_light_survives_its_file_form() {
    for light in [
        spot(),
        LightSource {
            kind: LightKind::Directional,
            color: [1.0; 3],
            intensity: 1.0,
        },
        LightSource {
            kind: LightKind::Point { range: 5.0 },
            color: [0.2, 0.4, 0.6],
            intensity: 2.5,
        },
    ] {
        assert_eq!(LightSource::from(LightForm::from(light)), light);
    }
}

#[test]
fn light_registers_once_under_its_name() {
    let mut registry = ComponentRegistry::new();
    register_rendering(&mut registry).expect("the first registration succeeds");
    assert!(registry.names().any(|name| name.as_str() == "light"));
    assert!(register_rendering(&mut registry).is_err());
}

const SCENE: &str = r#"{ format: 1, entities: [
    { id: "hero", components: { mesh_renderer: { mesh: "@asset:meshes/hero.obj", material: "@asset:materials/skin.mmat" } } },
] }"#;

#[test]
fn a_mesh_reference_is_read_and_written_as_typed_asset_references() {
    let mut registry = ComponentRegistry::new();
    register_rendering(&mut registry).unwrap();
    let mut world = World::new();
    let scene = registry
        .load_scene(SCENE, &mut world, &mut NoResolver)
        .expect("the scene loads");
    let hero = scene.entity(&EntityKey::new("hero").unwrap()).unwrap();
    let reference = world
        .get::<MeshReference>(hero)
        .expect("the reference is a component");
    assert_eq!(reference.mesh.id.as_str(), "meshes/hero.obj");
    assert_eq!(reference.material.id.as_str(), "materials/skin.mmat");
    assert!(!reference.billboard);

    let saved = registry.save_scene(&world, &scene);
    assert!(saved.contains("@asset:meshes/hero.obj"), "{saved}");
    let mut again = World::new();
    registry
        .load_scene(&saved, &mut again, &mut NoResolver)
        .expect("the saved scene loads again");
}

#[test]
fn a_mesh_reference_to_nothing_is_a_problem_while_reading() {
    struct Nothing;
    impl substrate_data::Resolve for Nothing {
        fn reference(
            &mut self,
            r: &substrate_data::Ref,
            _path: &substrate_data::Path,
        ) -> Result<substrate_data::RefKey, substrate_data::Unresolved> {
            Err(substrate_data::Unresolved::Missing(r.clone()))
        }
    }
    let mut registry = ComponentRegistry::new();
    register_rendering(&mut registry).unwrap();
    let mut world = World::new();
    assert!(
        registry
            .load_scene(SCENE, &mut world, &mut Nothing)
            .is_err()
    );
}
