//! Reading an instance's `overrides`: which entity, which component, and
//! what is done to it. Applying them waits for the prefab's entities.

use std::rc::Rc;

use substrate_data::schema::is_metadata_key;
use substrate_data::{Expected, MapRef, Path, Reader, Seg, Value, ValueRef};

use super::document_reader::DocumentReader;
use crate::document_keys::{COMPONENTS, FIELDS, OVERRIDE_TARGET, OVERRIDES, REMOVE};
use crate::draft::{ComponentOverride, OverrideEdit};
use crate::load::SceneProblem;
use crate::{ComponentName, EntityKey};

/// The instance whose `overrides` are read: its key as the document wrote
/// it, and as it is keyed once read.
struct Instance<'a> {
    written: &'a EntityKey,
    keyed: &'a EntityKey,
}

impl Instance<'_> {
    /// `target`, written in the document, as keyed: the instance or an
    /// entity below it; `None` for any other.
    fn key_of(&self, target: &EntityKey) -> Option<EntityKey> {
        if target == self.written {
            return Some(self.keyed.clone());
        }
        let below = target.as_str().strip_prefix(self.written.as_str())?;
        below.starts_with('/').then_some(())?;
        EntityKey::new(&format!("{}{below}", self.keyed)).ok()
    }
}

/// The overrides of one instance being collected.
struct InstanceReading<'a> {
    instance: Instance<'a>,
    found: Vec<ComponentOverride>,
}

/// The overrides of one target being collected.
struct TargetReading<'t> {
    keyed: EntityKey,
    at: Path,
    out: &'t mut Vec<ComponentOverride>,
}

impl TargetReading<'_> {
    fn push(
        &mut self,
        names: &Rc<super::DocumentNames>,
        name: ComponentName,
        edit: OverrideEdit,
        at: Path,
    ) {
        self.out.push(ComponentOverride {
            target: self.keyed.clone(),
            name,
            edit,
            target_at: self.at.clone(),
            at,
            names: Rc::clone(names),
        });
    }
}

/// How the values of a section of a target become edits.
fn edit_of(section: &str, value: Value) -> OverrideEdit {
    if section == COMPONENTS {
        OverrideEdit::Replace(value)
    } else {
        OverrideEdit::Merge(value)
    }
}

impl DocumentReader<'_> {
    /// The `overrides` of `entity`, a map from target to what is done to
    /// its components. Empty when there are none; `has_prefab` says whether
    /// the entity is an instance, which only an instance may carry them.
    pub(super) fn read_overrides(
        &mut self,
        entity: MapRef<'_>,
        (written, keyed): (&EntityKey, &EntityKey),
        has_prefab: bool,
        cx: &mut Reader<'_>,
    ) -> Option<Vec<ComponentOverride>> {
        let Some(v) = entity.get(OVERRIDES) else {
            return Some(Vec::new());
        };
        if !has_prefab {
            let problem = SceneProblem::OverridesWithoutPrefab { key: keyed.clone() };
            self.find(problem, cx.path().key(OVERRIDES), true);
            return None;
        }
        let mut reading = InstanceReading {
            instance: Instance { written, keyed },
            found: Vec::new(),
        };
        cx.at(Seg::Key(OVERRIDES.into()), |cx| {
            let map = cx.map(v)?;
            let mut ok = true;
            for (target, value) in map.iter().filter(|(k, _)| !is_metadata_key(k)) {
                ok &= self.read_target(target, value, &mut reading, cx);
            }
            ok.then_some(reading.found)
        })
    }

    /// What is done to the components of `target` (as written), into `out`.
    fn read_target(
        &mut self,
        target: &str,
        v: &ValueRef<'_>,
        reading: &mut InstanceReading<'_>,
        cx: &mut Reader<'_>,
    ) -> bool {
        let instance = &reading.instance;
        let at = cx.path().key(target);
        let written = match EntityKey::new(target) {
            Ok(written) => written,
            Err(e) => {
                self.find(SceneProblem::BadOverrideTarget(e), at, true);
                return false;
            }
        };
        let Some(keyed) = instance.key_of(&written) else {
            let problem = SceneProblem::OverrideOutsideInstance {
                target: written,
                instance: instance.written.clone(),
            };
            self.find(problem, at, true);
            return false;
        };
        cx.at(Seg::Key(target.into()), |cx| {
            let Some(map) = cx.map(v) else {
                return false;
            };
            cx.unknown_keys(map, &OVERRIDE_TARGET);
            let mut target = TargetReading {
                keyed,
                at,
                out: &mut reading.found,
            };
            let mut ok = self.read_edits(map, COMPONENTS, &mut target, cx);
            ok &= self.read_edits(map, FIELDS, &mut target, cx);
            ok & self.read_removals(map, &mut target, cx)
        })
    }

    /// The `components` or `fields` of a target: a component name and its
    /// value each.
    fn read_edits(
        &mut self,
        target: MapRef<'_>,
        section: &'static str,
        reading: &mut TargetReading<'_>,
        cx: &mut Reader<'_>,
    ) -> bool {
        let Some(v) = target.get(section) else {
            return true;
        };
        cx.at(Seg::Key(section.into()), |cx| {
            let Some(map) = cx.map(v) else {
                return false;
            };
            let mut ok = true;
            for (key, value) in map.iter().filter(|(k, _)| !is_metadata_key(k)) {
                let at = cx.path().key(key);
                let Some(name) = self.known_component(key, at.clone(), true) else {
                    ok = false;
                    continue;
                };
                let edit = edit_of(section, value.to_value());
                reading.push(&self.names, name, edit, at);
            }
            ok
        })
    }

    /// The `remove` of a target: a list of component names.
    fn read_removals(
        &mut self,
        target: MapRef<'_>,
        reading: &mut TargetReading<'_>,
        cx: &mut Reader<'_>,
    ) -> bool {
        let Some(v) = target.get(REMOVE) else {
            return true;
        };
        cx.at(Seg::Key(REMOVE.into()), |cx| {
            let Some(list) = v.as_list() else {
                cx.expected(Expected::List, v);
                return false;
            };
            let mut ok = true;
            for (i, item) in list.iter().enumerate() {
                let at = cx.path().index(i);
                ok &= match item.as_str() {
                    Some(text) => self.removal(text, at, reading),
                    None => {
                        cx.at(Seg::Index(i), |cx| cx.expected(Expected::Text, item));
                        false
                    }
                };
            }
            ok
        })
    }

    fn removal(&mut self, text: &str, at: Path, reading: &mut TargetReading<'_>) -> bool {
        match ComponentName::new(text) {
            Ok(name) => {
                reading.push(&self.names, name, OverrideEdit::Remove, at);
                true
            }
            Err(e) => {
                self.find(SceneProblem::BadComponentName(e), at, false);
                false
            }
        }
    }
}
