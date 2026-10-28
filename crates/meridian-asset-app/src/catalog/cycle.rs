//! Finding a loop in the dependency graph.

use std::collections::HashMap;

use meridian_asset_domain::AssetId;

use super::CatalogEntry;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Visiting,
    Done,
}

/// An id on a dependency loop, if the entries have one. Every dependency
/// must already be in `entries`. Iterative, so a long chain cannot overflow
/// the stack.
pub(super) fn find_cycle(entries: &HashMap<AssetId, CatalogEntry>) -> Option<AssetId> {
    let mut marks: HashMap<AssetId, Mark> = HashMap::new();
    let mut roots: Vec<AssetId> = entries.keys().copied().collect();
    roots.sort();
    for root in roots {
        if marks.contains_key(&root) {
            continue;
        }
        let mut stack = vec![(root, 0_usize)];
        marks.insert(root, Mark::Visiting);
        while let Some((id, next)) = stack.pop() {
            let dependencies = &entries[&id].dependencies;
            let Some(&dependency) = dependencies.get(next) else {
                marks.insert(id, Mark::Done);
                continue;
            };
            stack.push((id, next + 1));
            match marks.get(&dependency) {
                Some(Mark::Visiting) => return Some(dependency),
                Some(Mark::Done) => {}
                None => {
                    marks.insert(dependency, Mark::Visiting);
                    stack.push((dependency, 0));
                }
            }
        }
    }
    None
}
