//! Saving a prefab instance writes what changed from its prefab: field
//! level overrides for the instance and for the entities its prefab made,
//! so that load, edit, save and load again gives the same world.

mod common;
mod saving_support;

use common::{Follow, Motion, RigidBody, key, registry, transform};
use meridian_data_substrate::SceneView;
use meridian_ecs::Transform;
use meridian_gac::{Motor3, Vec3};
use saving_support::{SCENE, at, library, load, save, state, value_at};
use substrate_data::Value;

#[test]
fn an_untouched_instance_saves_as_just_the_prefab() {
    let (world, map) = load(SCENE.as_bytes());
    let saved = save(&world, &map);
    assert!(
        value_at(&saved, "entities[0].overrides").is_none(),
        "{saved}"
    );
    assert!(
        value_at(&saved, "entities[0].components").is_none(),
        "{saved}"
    );
    assert!(value_at(&saved, "entities[0].name").is_none(), "{saved}");
    assert!(saved.contains("@prefab:actors/hero"), "{saved}");
    assert!(
        !saved.contains("hero/gun") && !saved.contains("h/gun"),
        "{saved}"
    );
}

#[test]
fn editing_an_instance_and_its_children_saves_only_what_changed() {
    let (mut world, map) = load(SCENE.as_bytes());
    let body = RigidBody {
        motion: Motion::Dynamic,
        mass: 5.0,
    };
    world.insert(at(&map, "h"), body);
    let follow = Follow {
        target: key("other"),
    };
    world.insert(at(&map, "h/gun"), follow);
    let saved = save(&world, &map);
    // The instance's mass alone, not the motion that stayed.
    let mass = value_at(&saved, "entities[0].overrides.h.fields.rigid_body");
    let mass = mass.unwrap_or_else(|| panic!("{saved}"));
    assert_eq!(mass.get("mass"), Some(&Value::Float(5.0)), "{saved}");
    assert!(mass.get("motion").is_none(), "{saved}");
    // The generated child under its path, the field but nothing else.
    let target = value_at(&saved, "entities[0].overrides.h/gun.fields.follow.target");
    assert!(target.is_some(), "{saved}");
    assert!(!saved.contains("transform"), "{saved}");
    // Load, save, load: the same world and the same document.
    let (again, back) = load(saved.as_bytes());
    assert_eq!(state(&again, &back), state(&world, &map));
    assert_eq!(save(&again, &back), saved);
}

#[test]
fn a_component_whose_form_changes_is_written_whole() {
    // A pure translation is written as `translation` or as `dual`, by what
    // it holds exactly: the keys differ, so no field patch can express it.
    let (mut world, map) = load(SCENE.as_bytes());
    let moved = Transform::from_motor(Motor3::translation(Vec3::new(0.5, 3.0, 0.0)));
    world.insert(at(&map, "h/gun"), moved);
    let saved = save(&world, &map);
    let whole = value_at(&saved, "entities[0].overrides.h/gun.components.transform");
    assert!(whole.is_some(), "{saved}");
    assert!(
        value_at(&saved, "entities[0].overrides.h/gun.fields").is_none(),
        "{saved}"
    );
    let (again, back) = load(saved.as_bytes());
    assert_eq!(state(&again, &back), state(&world, &map));
    assert_eq!(
        transform(&again, &back, "h/gun"),
        Motor3::translation(Vec3::new(0.5, 3.0, 0.0))
    );
}

#[test]
fn the_binary_form_saves_and_loads_the_same_overrides() {
    let (mut world, map) = load(SCENE.as_bytes());
    world.insert(
        at(&map, "h"),
        RigidBody {
            motion: Motion::Static,
            mass: 80.0,
        },
    );
    let bytes = registry()
        .save_scene_binary_with_prefabs(SceneView::new(&world, &map), &mut library())
        .unwrap();
    assert!(bytes.starts_with(b"SDAT"));
    let (again, back) = load(&bytes);
    assert_eq!(state(&again, &back), state(&world, &map));
    // The document is the one text spells.
    let document = registry()
        .scene_document_with_prefabs(SceneView::new(&world, &map), &mut library())
        .unwrap();
    assert_eq!(substrate_data::binary::decode(&bytes).unwrap(), document);
}

#[test]
fn a_whole_replace_written_by_hand_saves_as_field_overrides() {
    let scene = r#"{ format: 1, entities: [
        { id: "h", name: "Mine", prefab: "@prefab:actors/hero",
          components: { rigid_body: { motion: "dynamic", mass: 12 } } } ] }"#;
    let (world, map) = load(scene.as_bytes());
    let saved = save(&world, &map);
    assert!(
        value_at(&saved, "entities[0].components").is_none(),
        "{saved}"
    );
    let patch = value_at(&saved, "entities[0].overrides.h.fields.rigid_body").unwrap();
    assert_eq!(patch.as_map().map(|m| m.len()), Some(1), "{saved}");
    assert!(saved.contains("Mine"), "{saved}");
    let (again, back) = load(saved.as_bytes());
    assert_eq!(state(&again, &back), state(&world, &map));
}

#[test]
fn save_scene_without_prefabs_is_unchanged() {
    let (world, map) = load(SCENE.as_bytes());
    let plain = registry().save_scene(&world, &map);
    // All the instance's components, as before; the prefab makes the rest.
    assert!(
        value_at(&plain, "entities[0].components.rigid_body").is_some(),
        "{plain}"
    );
    assert!(
        value_at(&plain, "entities[0].overrides").is_none(),
        "{plain}"
    );
}
