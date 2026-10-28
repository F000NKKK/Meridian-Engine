//! [`DependencyGraph`]: the topology of declared dependencies.

use std::collections::HashMap;

use meridian_memory::Handle;

use crate::ResourceDependency;

/// Tracks the topology of declared [`ResourceDependency`] edges, keyed by
/// the underlying `Handle` (type-erasing `A`/`B` — the graph reasons about
/// "does this handle transitively depend on that one," not about what kind
/// of resource either is). Deliberately does not store resource data: this
/// is a graph over handles the caller already owns elsewhere, not a pool.
#[derive(Debug, Default)]
pub struct DependencyGraph {
    edges: HashMap<Handle, Vec<Handle>>,
}

impl DependencyGraph {
    /// A graph with no edge.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records that `dep.from` depends on `dep.on`. Idempotent: recording
    /// the same edge twice doesn't duplicate it.
    pub fn add_dependency<A, B>(&mut self, dep: ResourceDependency<A, B>) {
        let targets = self.edges.entry(dep.from.handle).or_default();
        if !targets.contains(&dep.on.handle) {
            targets.push(dep.on.handle);
        }
    }

    /// The handles `from` directly depends on (not transitive).
    #[must_use]
    pub fn direct_dependencies(&self, from: Handle) -> &[Handle] {
        self.edges.get(&from).map_or(&[], Vec::as_slice)
    }

    /// True if `from` depends on `on`, directly or transitively.
    #[must_use]
    pub fn depends_on(&self, from: Handle, on: Handle) -> bool {
        let mut stack = vec![from];
        let mut visited = std::collections::HashSet::new();
        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            for &next in self.direct_dependencies(current) {
                if next == on {
                    return true;
                }
                stack.push(next);
            }
        }
        false
    }

    /// True if recording a dependency from `from` on `on` would close a
    /// cycle (including the trivial `from == on` self-dependency) — check
    /// this before [`add_dependency`](Self::add_dependency) if the graph
    /// must stay acyclic (e.g. before wiring a material to a texture, to
    /// catch "texture's shader references this same material" upstream).
    #[must_use]
    pub fn would_cycle(&self, from: Handle, on: Handle) -> bool {
        from == on || self.depends_on(on, from)
    }
}
