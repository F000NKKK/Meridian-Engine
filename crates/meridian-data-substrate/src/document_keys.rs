//! The keys of an `.mscene` document, defined once.

/// The `.mscene` format version this crate reads and writes. Formats evolve
/// by appending; a bump comes with a `Value -> Value` migration (ADR 016).
pub const FORMAT: u32 = 1;

pub(crate) const FORMAT_KEY: &str = "format";
pub(crate) const NAME: &str = "name";
pub(crate) const ENTITIES: &str = "entities";
pub(crate) const ID: &str = "id";
pub(crate) const PREFAB: &str = "prefab";
pub(crate) const PARENT: &str = "parent";
pub(crate) const COMPONENTS: &str = "components";

/// The keys a document may have.
pub(crate) const DOCUMENT: [&str; 3] = [FORMAT_KEY, NAME, ENTITIES];
/// The keys an entity may have.
pub(crate) const ENTITY: [&str; 5] = [ID, NAME, PREFAB, PARENT, COMPONENTS];
