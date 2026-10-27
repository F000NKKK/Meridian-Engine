//! [`NoResolver`]: references kept as written.

use substrate_data::{Path, Ref, RefKey, Resolve, Unresolved};

/// A [`Resolve`] that accepts every outside reference as written, for
/// scenes whose prefab references the caller checks later (or never).
/// `@entity:` references are still checked against the document.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoResolver;

impl Resolve for NoResolver {
    fn reference(&mut self, r: &Ref, _path: &Path) -> Result<RefKey, Unresolved> {
        Ok(r.id.clone())
    }
}
