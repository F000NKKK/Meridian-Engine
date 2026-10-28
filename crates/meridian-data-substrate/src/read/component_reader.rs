//! Reading an entity's `components`: each by the registry's entry for its
//! name, or kept as written.

use substrate_data::schema::is_metadata_key;
use substrate_data::{MapRef, Path, Reader, Seg, ValueRef};

use super::document_reader::DocumentReader;
use crate::document_keys::COMPONENTS;
use crate::draft::ComponentsDraft;
use crate::load::SceneProblem;
use crate::{ComponentName, KeptComponent, UnknownComponents};

impl DocumentReader<'_> {
    pub(super) fn read_components(
        &mut self,
        entity: MapRef<'_>,
        cx: &mut Reader<'_>,
    ) -> Option<ComponentsDraft> {
        let Some(v) = entity.get(COMPONENTS) else {
            return Some(ComponentsDraft::default());
        };
        cx.at(Seg::Key(COMPONENTS.into()), |cx| {
            let map = cx.map(v)?;
            let mut draft = ComponentsDraft::default();
            let mut ok = true;
            for (key, value) in map.iter().filter(|(k, _)| !is_metadata_key(k)) {
                ok &= self.read_component(key, value, cx, &mut draft);
            }
            ok.then_some(draft)
        })
    }

    /// Component `key` of an entity into `draft`; whether it read.
    fn read_component(
        &mut self,
        key: &str,
        value: &ValueRef<'_>,
        cx: &mut Reader<'_>,
        draft: &mut ComponentsDraft,
    ) -> bool {
        let path = cx.path().key(key);
        let Some(name) = self.known_component(key, path, true) else {
            return false;
        };
        let Some(entry) = self.registry.entry(&name) else {
            draft.kept.push(KeptComponent {
                name,
                value: value.to_value(),
            });
            return true;
        };
        let read = cx.at(Seg::Key(key.into()), |cx| (entry.read)(value, cx));
        read.map(|insert| draft.inserts.push((name, insert)))
            .is_some()
    }

    /// `key` as the name of a component this reader accepts: a registered
    /// one, or any when unknown ones are kept. A finding at `path` (on the
    /// key, when `on_key`) if not.
    pub(super) fn known_component(
        &mut self,
        key: &str,
        path: Path,
        on_key: bool,
    ) -> Option<ComponentName> {
        let name = match ComponentName::new(key) {
            Ok(name) => name,
            Err(e) => {
                self.find(SceneProblem::BadComponentName(e), path, on_key);
                return None;
            }
        };
        if self.registry.entry(&name).is_some()
            || self.registry.unknown_components() == UnknownComponents::Keep
        {
            return Some(name);
        }
        let suggestion = name.closest(self.registry.names()).cloned();
        let problem = SceneProblem::UnknownComponent { name, suggestion };
        self.find(problem, path, on_key);
        None
    }
}
