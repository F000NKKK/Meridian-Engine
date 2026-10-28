//! What the tests that save prefab instances share: a hero prefab, a scene
//! that instances it, and ways to load, save and compare.

use meridian_data_substrate::{
    ComponentName, EntityKey, NoResolver, PrefabLibrary, PrefabPath, SceneMap, SceneView,
};
use meridian_ecs::{Entity, World};
use substrate_data::{Path, Value, text};

use crate::common::{key, registry};

pub fn path(s: &str) -> PrefabPath {
    PrefabPath::new(s).unwrap()
}

pub const HERO: &str = r#"{
  format: 1,
  entities: [
    { id: "body", name: "Body", components: {
        transform: { motor: { rotor: [1, 0, 0, 0], translation: [0, 1, 0] } },
        rigid_body: { motion: "dynamic", mass: 80 } } },
    { id: "gun", parent: "@entity:body", components: {
        transform: { motor: { rotor: [1, 0, 0, 0], translation: [0.5, 0, 0] } },
        follow: { target: "@entity:body" } } },
  ],
}"#;

pub const SCENE: &str = r#"{ format: 1, name: "Arena", entities: [
    { id: "h", prefab: "@prefab:actors/hero" },
    { id: "other", components: { rigid_body: { motion: "static", mass: 1 } } } ] }"#;

pub fn library() -> PrefabLibrary {
    let mut lib = PrefabLibrary::new();
    lib.insert(path("actors/hero"), HERO);
    lib
}

pub fn load(bytes: &[u8]) -> (World, SceneMap) {
    let mut world = World::new();
    let map = registry()
        .load_scene_with_prefabs(bytes, &mut world, &mut NoResolver, &mut library())
        .unwrap();
    (world, map)
}

pub fn save(world: &World, map: &SceneMap) -> String {
    registry()
        .save_scene_with_prefabs(SceneView::new(world, map), &mut library())
        .unwrap()
}

pub fn at(map: &SceneMap, k: &str) -> Entity {
    map.entity(&key(k)).unwrap()
}

/// The value at `p` (`entities[0].overrides`) of a saved document.
pub fn value_at(saved: &str, p: &str) -> Option<Value> {
    let doc = text::parse(saved).unwrap().value;
    let mut path = Path::root();
    for step in p.split('.') {
        path = match step.strip_suffix(']').and_then(|s| s.split_once('[')) {
            Some((k, i)) => path.key(k).index(i.parse().unwrap()),
            None => path.key(step),
        };
    }
    doc.at(&path).cloned()
}

/// What a scene's entities are in `world`: key, parent, and each registered
/// component's value.
pub type Entities = Vec<(EntityKey, Option<EntityKey>, Vec<(ComponentName, Value)>)>;

/// Everything in the world the scene lists, for comparing loads.
pub fn state(world: &World, map: &SceneMap) -> Entities {
    let reg = registry();
    let components = |e: Entity| {
        let value = |n: &ComponentName| reg.component_value(world, e, n).unwrap();
        let named = |n: &ComponentName| value(n).map(|v| (n.clone(), v));
        reg.names().filter_map(named).collect()
    };
    let all = map.entities().iter();
    all.map(|e| (e.key.clone(), e.parent.clone(), components(e.entity)))
        .collect()
}
