//! The keys of an `.mscene` document, defined once.

/// The `.mscene` and `.mprefab` format version this crate reads and writes. Formats evolve
/// by appending; a bump comes with a `Value -> Value` migration (ADR 016).
pub const FORMAT: u32 = 1;

pub(crate) const FORMAT_KEY: &str = "format";
pub(crate) const NAME: &str = "name";
pub(crate) const ENTITIES: &str = "entities";
pub(crate) const ID: &str = "id";
pub(crate) const PREFAB: &str = "prefab";
pub(crate) const PARENT: &str = "parent";
pub(crate) const COMPONENTS: &str = "components";
pub(crate) const OVERRIDES: &str = "overrides";
pub(crate) const FIELDS: &str = "fields";
pub(crate) const REMOVE: &str = "remove";

/// The keys a document may have.
pub(crate) const DOCUMENT: [&str; 3] = [FORMAT_KEY, NAME, ENTITIES];
/// The keys an entity may have.
pub(crate) const ENTITY: [&str; 6] = [ID, NAME, PREFAB, PARENT, COMPONENTS, OVERRIDES];
/// The keys one target of an instance's `overrides` may have.
pub(crate) const OVERRIDE_TARGET: [&str; 3] = [COMPONENTS, FIELDS, REMOVE];
/// The keys a prefab document may have: a scene's without `name`.
pub(crate) const PREFAB_DOCUMENT: [&str; 2] = [FORMAT_KEY, ENTITIES];

/// The `.msave` format version this crate reads and writes: the envelope's
/// own, apart from the game's `state` (which the game versions itself).
pub const SAVE_FORMAT: u32 = 1;

pub(crate) const GAME: &str = "game";
pub(crate) const CREATED: &str = "created";
pub(crate) const SCENE: &str = "scene";
pub(crate) const STATE: &str = "state";
