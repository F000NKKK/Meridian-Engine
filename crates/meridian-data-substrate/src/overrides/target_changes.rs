//! [`TargetChanges`]: what an instance changes in one of its entities.

use substrate_data::{Map, Value};

use crate::ComponentName;
use crate::document_keys::{COMPONENTS, FIELDS, REMOVE};

/// What changes in one entity an instance made, as `overrides` write it.
#[derive(Debug, Default)]
pub(crate) struct TargetChanges {
    /// Components written whole (`components`).
    pub(crate) replace: Vec<(ComponentName, Value)>,
    /// Components patched field by field (`fields`).
    pub(crate) merge: Vec<(ComponentName, Value)>,
    /// Components taken away (`remove`).
    pub(crate) remove: Vec<ComponentName>,
}

impl TargetChanges {
    /// The target's entry in `overrides`; `None` when nothing changed.
    pub(crate) fn into_value(self) -> Option<Value> {
        let mut target = Map::new();
        for (key, section) in [(COMPONENTS, self.replace), (FIELDS, self.merge)] {
            if !section.is_empty() {
                target.insert(key, Value::Map(named(section)));
            }
        }
        if !self.remove.is_empty() {
            let names = self.remove.iter().map(|n| Value::Str(n.to_string()));
            target.insert(REMOVE, Value::List(names.collect()));
        }
        (!target.is_empty()).then_some(Value::Map(target))
    }
}

fn named(section: Vec<(ComponentName, Value)>) -> Map {
    let mut map = Map::new();
    for (name, value) in section {
        map.insert(name.as_str(), value);
    }
    map
}
