//! [`ComponentOverride`]: one change an instance makes to a component of
//! one of its entities.

use std::rc::Rc;

use substrate_data::Path;

use super::OverrideEdit;
use crate::read::DocumentNames;
use crate::{ComponentName, EntityKey};

/// One change to component `name` of the entity keyed `target`, read from
/// an instance's `overrides` but not applied: that needs the prefab's
/// entities. The edit's `Value` is as written, so its `@entity:` names
/// mean what `names` says.
#[derive(Debug, Clone)]
pub(crate) struct ComponentOverride {
    pub(crate) target: EntityKey,
    pub(crate) name: ComponentName,
    pub(crate) edit: OverrideEdit,
    /// The target's key in the document.
    pub(crate) target_at: Path,
    /// The component's value (or name, for a removal) in the document.
    pub(crate) at: Path,
    /// The document the override is written in.
    pub(crate) names: Rc<DocumentNames>,
}
