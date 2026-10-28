//! Prefabs in and out of a [`ComponentRegistry`]: instancing, loading and
//! saving `.mprefab`.

use meridian_ecs::World;
use substrate_data::Resolve;

use super::ComponentRegistry;
use crate::document_kind::DocumentKind;
use crate::load::LoadError;
use crate::read::{DocumentRole, DocumentSource, ReadContext};
use crate::scene_writer::SceneWriter;
use crate::{PrefabInstance, PrefabSource, SceneMap};

impl ComponentRegistry {
    /// Reads a scene file's bytes (either form) into `world`, instancing
    /// every entity that says `prefab` from `prefabs`: the entity becomes
    /// the instance's root, with the prefab root's components under its
    /// own (a component the entity lists replaces the prefab's whole), and
    /// the prefab's other entities follow as `<entity id>/<prefab id>`,
    /// recorded in [`crate::SceneEntity::generated_by`]. Prefabs may
    /// instance prefabs, to [`crate::MAX_PREFAB_DEPTH`]; one that includes
    /// itself is refused. All or nothing, like every load.
    ///
    /// # Errors
    ///
    /// [`LoadError`] with every problem, placed. A prefab that is missing
    /// or does not load is one problem at the `prefab` field naming it
    /// ([`crate::SceneProblem::PrefabUnavailable`], `PrefabInvalid`,
    /// `PrefabCycle`, `PrefabTooDeep`); nothing is spawned.
    pub fn load_scene_with_prefabs(
        &self,
        bytes: &[u8],
        world: &mut World,
        resolver: &mut dyn Resolve,
        prefabs: &mut dyn PrefabSource,
    ) -> Result<SceneMap, LoadError> {
        let mut cx = ReadContext::new(self, resolver).instancing(prefabs);
        let draft = DocumentSource::detect(bytes)?.draft(&DocumentRole::Scene, &mut cx)?;
        Ok(draft.spawn(world))
    }

    /// Reads an `.mprefab` (either form) as itself, the way an editor opens
    /// it: its entities keep their local ids, the first being the root;
    /// prefabs it instances are expanded from `prefabs`.
    ///
    /// # Errors
    ///
    /// [`LoadError`], as [`ComponentRegistry::load_scene_with_prefabs`],
    /// plus the prefab rules: at least a root, the root has no parent and
    /// every other entity has one.
    pub fn load_prefab(
        &self,
        bytes: &[u8],
        world: &mut World,
        resolver: &mut dyn Resolve,
        prefabs: &mut dyn PrefabSource,
    ) -> Result<SceneMap, LoadError> {
        let mut cx = ReadContext::new(self, resolver).instancing(prefabs);
        let draft = DocumentSource::detect(bytes)?.draft(&DocumentRole::Prefab, &mut cx)?;
        Ok(draft.spawn(world))
    }

    /// Instances the prefab `instance.prefab` into `world` the way a scene
    /// entity `{ id: <instance.key>, prefab: … }` would be: the returned
    /// map has the root under `instance.key` and the rest under
    /// `<key>/<prefab id>`. For spawning from code at run time.
    ///
    /// # Errors
    ///
    /// [`LoadError`] as for a scene; the paths read as if the instance
    /// were `entities[0]` of a scene.
    pub fn instantiate_prefab(
        &self,
        instance: &PrefabInstance,
        world: &mut World,
        resolver: &mut dyn Resolve,
        prefabs: &mut dyn PrefabSource,
    ) -> Result<SceneMap, LoadError> {
        let mut cx = ReadContext::new(self, resolver).instancing(prefabs);
        Ok(cx.instanced(instance)?.spawn(world))
    }

    /// The entities of `scene` alive in `world`, as `.mprefab` text: the
    /// scene document without a `name`. The first entity of `scene` is the
    /// root and the others hang under it, as loading a prefab requires.
    #[must_use]
    pub fn save_prefab(&self, world: &World, scene: &SceneMap) -> String {
        SceneWriter::new(self, world, scene).text(DocumentKind::Prefab)
    }

    /// The same prefab in the `SDAT` binary form.
    #[must_use]
    pub fn save_prefab_binary(&self, world: &World, scene: &SceneMap) -> Vec<u8> {
        SceneWriter::new(self, world, scene).bytes(DocumentKind::Prefab)
    }
}
