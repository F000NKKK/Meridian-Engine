//! Scenes in and out of a [`ComponentRegistry`]: text and binary.

use meridian_ecs::World;
use substrate_data::{Resolve, Value};

use super::ComponentRegistry;
use crate::SceneMap;
use crate::document_kind::DocumentKind;
use crate::load::LoadError;
use crate::read::{DocumentRole, DocumentSource, ReadContext};
use crate::scene_writer::SceneWriter;

impl ComponentRegistry {
    /// Reads `.mscene` text into `world`. Every entity of the document is
    /// spawned with its registered components; `@entity:` references
    /// resolve within the document and every other kind through
    /// `resolver`. All or nothing.
    ///
    /// # Errors
    ///
    /// [`LoadError`] with every problem in the document, placed (path, line
    /// and column); nothing is spawned then.
    pub fn load_scene(
        &self,
        text: &str,
        world: &mut World,
        resolver: &mut dyn Resolve,
    ) -> Result<SceneMap, LoadError> {
        let draft = DocumentSource::Text(text)
            .draft(&DocumentRole::Scene, &mut ReadContext::new(self, resolver))?;
        Ok(draft.spawn(world))
    }

    /// Reads the `SDAT` binary form of a scene into `world`, with the same
    /// rules and the same [`SceneMap`] as [`ComponentRegistry::load_scene`].
    /// A binary document has no lines: a problem has its path, and bytes
    /// that do not decode (truncated, an unknown tag, a duplicate key, …)
    /// have the byte offset ([`crate::SceneIssue::offset`]).
    ///
    /// # Errors
    ///
    /// [`LoadError`] with every problem in the document; nothing is
    /// spawned then. Malformed bytes are an error, never a panic.
    pub fn load_scene_binary(
        &self,
        bytes: &[u8],
        world: &mut World,
        resolver: &mut dyn Resolve,
    ) -> Result<SceneMap, LoadError> {
        let draft = DocumentSource::Binary(bytes)
            .draft(&DocumentRole::Scene, &mut ReadContext::new(self, resolver))?;
        Ok(draft.spawn(world))
    }

    /// Reads a scene file's bytes in whichever form they are: `SDAT` magic
    /// is binary, anything else is UTF-8 text (never decided by the file's
    /// extension).
    ///
    /// # Errors
    ///
    /// [`LoadError`], as the form's own loader.
    pub fn load_scene_bytes(
        &self,
        bytes: &[u8],
        world: &mut World,
        resolver: &mut dyn Resolve,
    ) -> Result<SceneMap, LoadError> {
        let source = DocumentSource::detect(bytes)?;
        let draft = source.draft(&DocumentRole::Scene, &mut ReadContext::new(self, resolver))?;
        Ok(draft.spawn(world))
    }

    /// `world`'s entities listed in `scene`, as `.mscene` text.
    #[must_use]
    pub fn save_scene(&self, world: &World, scene: &SceneMap) -> String {
        SceneWriter::new(self, world, scene).text(DocumentKind::Scene)
    }

    /// `world`'s entities listed in `scene`, in the `SDAT` binary form
    /// (the same document as [`ComponentRegistry::save_scene`]).
    #[must_use]
    pub fn save_scene_binary(&self, world: &World, scene: &SceneMap) -> Vec<u8> {
        SceneWriter::new(self, world, scene).bytes(DocumentKind::Scene)
    }

    /// The scene document of `world`'s entities listed in `scene`, as a
    /// value: what both forms spell, and what a `.msave` embeds.
    #[must_use]
    pub fn scene_document(&self, world: &World, scene: &SceneMap) -> Value {
        SceneWriter::new(self, world, scene).document(DocumentKind::Scene)
    }
}
