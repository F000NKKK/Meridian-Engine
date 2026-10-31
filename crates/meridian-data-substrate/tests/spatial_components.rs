//! Spatial values are independent registry entries, not special scene syntax.
use meridian_data_substrate::{ComponentName, ComponentRegistry, NoResolver, PositionCodec};
use meridian_ecs::{Position, World};
use meridian_gac::{Motor3, Rotor, Vec3};
use substrate_data::Schema;

#[test]
fn independent_spatial_components_round_trip_through_the_component_registry() {
    let mut registry = ComponentRegistry::new();
    let name = |value| ComponentName::new(value).expect("valid component name");
    registry
        .register::<Rotor>(name("rotor"))
        .expect("rotor registration");
    registry
        .register::<Motor3>(name("motor"))
        .expect("motor registration");
    registry
        .register_with::<Position, PositionCodec>(name("position"))
        .expect("position registration");
    let mut world = World::new();
    let loaded = registry
        .load_scene(
            r#"{format:1,entities:[
      {id:"split",components:{position:[1,2,3],rotor:[1,0,0,0]}},
      {id:"rigid",components:{motor:{translation:[4,5,6],rotor:[1,0,0,0]}}}
    ]}"#,
            &mut world,
            &mut NoResolver,
        )
        .expect("independent components load");
    let split = loaded.entities()[0].entity;
    let rigid = loaded.entities()[1].entity;
    assert_eq!(
        world.get::<Position>(split),
        Some(&Position(Vec3::new(1.0, 2.0, 3.0)))
    );
    assert_eq!(world.get::<Rotor>(split), Some(&Rotor::identity()));
    assert_eq!(
        world
            .get::<Motor3>(rigid)
            .expect("motor")
            .transform_point(Vec3::ZERO),
        Vec3::new(4.0, 5.0, 6.0)
    );
    assert_eq!(
        registry
            .component_value(&world, split, &name("position"))
            .expect("serialized"),
        Some(Vec3::new(1.0, 2.0, 3.0).write())
    );
}
