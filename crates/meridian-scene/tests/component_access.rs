//! Reading and writing one component by name, as an editor does.

use meridian_ecs_core::{Component, World};
use meridian_scene::{ComponentEditError, ComponentName, ComponentRegistry};
use substrate_data::{Schema, Value};

#[derive(Debug, Clone, PartialEq, Schema)]
struct Mass {
    kilograms: f32,
}
impl Component for Mass {}

fn name(s: &str) -> ComponentName {
    ComponentName::new(s).unwrap()
}

fn registry() -> ComponentRegistry {
    let mut registry = ComponentRegistry::new();
    registry.register::<Mass>(name("mass")).unwrap();
    registry
}

fn mass_value(kilograms: f32) -> Value {
    Mass { kilograms }.write()
}

#[test]
fn a_set_component_reads_back() {
    let registry = registry();
    let mut world = World::new();
    let entity = world.spawn();
    registry
        .set_component(&mut world, entity, &name("mass"), &mass_value(2.0))
        .unwrap();
    assert_eq!(world.get::<Mass>(entity), Some(&Mass { kilograms: 2.0 }));
    let value = registry
        .component_value(&world, entity, &name("mass"))
        .unwrap();
    assert_eq!(value, Some(mass_value(2.0)));
}

#[test]
fn a_missing_component_is_none_and_an_unknown_one_is_an_error() {
    let registry = registry();
    let mut world = World::new();
    let entity = world.spawn();
    let missing = registry.component_value(&world, entity, &name("mass"));
    assert_eq!(missing.unwrap(), None);
    let unknown = name("warp_core");
    assert!(!registry.contains(&unknown));
    assert!(registry.contains(&name("mass")));
    assert_eq!(
        registry.component_value(&world, entity, &unknown),
        Err(ComponentEditError::Unregistered { name: unknown })
    );
}

#[test]
fn an_invalid_value_leaves_the_world_untouched_and_says_why() {
    let registry = registry();
    let mut world = World::new();
    let entity = world.spawn();
    let result = registry.set_component(&mut world, entity, &name("mass"), &Value::Bool(true));
    assert!(matches!(
        result,
        Err(ComponentEditError::Invalid { ref diagnostics }) if !diagnostics.is_empty()
    ));
    assert!(world.get::<Mass>(entity).is_none());
}

#[test]
fn a_dead_entity_cannot_be_edited() {
    let registry = registry();
    let mut world = World::new();
    let entity = world.spawn();
    world.despawn(entity);
    let result = registry.set_component(&mut world, entity, &name("mass"), &mass_value(1.0));
    assert_eq!(result, Err(ComponentEditError::NoSuchEntity));
}
