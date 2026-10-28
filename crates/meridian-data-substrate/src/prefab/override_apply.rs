//! Applying an instance's `overrides` to the entities its prefab made.

use std::rc::Rc;

use substrate_data::{Bump, Path, Problem, Reader, Seg, Value};

use super::name_check::suggestion;
use crate::KeptComponent;
use crate::draft::{ComponentOverride, ComponentsDraft, OverrideEdit, PendingEntity};
use crate::load::SceneProblem;
use crate::patch::merged;
use crate::read::{DocumentResolver, Finding, ReadContext};
use crate::registry::{Entry, Insert};

impl ReadContext<'_> {
    /// Applies `overrides`, in the order written, to `group`: the entities
    /// one instance made, the root first. Every override that can't be
    /// applied is a finding; the others still are.
    pub(super) fn apply_overrides(
        &mut self,
        group: &mut [PendingEntity],
        overrides: Vec<ComponentOverride>,
    ) -> Vec<Finding> {
        let mut found = Vec::new();
        let mut reported: Vec<Path> = Vec::new();
        for o in overrides {
            let Some(at) = group.iter().position(|e| e.key == o.target) else {
                if !reported.contains(&o.target_at) {
                    found.push(missing_target(&o, group));
                    reported.push(o.target_at.clone());
                }
                continue;
            };
            if let Err(problems) = self.apply_one(&mut group[at].components, &o) {
                found.extend(problems);
            }
        }
        found
    }

    fn apply_one(
        &mut self,
        components: &mut ComponentsDraft,
        o: &ComponentOverride,
    ) -> Result<(), Vec<Finding>> {
        let value = match &o.edit {
            OverrideEdit::Remove if components.remove(&o.name) => return Ok(()),
            OverrideEdit::Remove => return Err(vec![removes_missing(o)]),
            OverrideEdit::Replace(whole) => whole.clone(),
            OverrideEdit::Merge(patch) => merged(components.document(&o.name), patch),
        };
        let registry = self.registry;
        match registry.entry(&o.name) {
            Some(entry) => {
                let insert = self.read_edited(entry, &value, o)?;
                components.put(o.name.clone(), insert);
            }
            None => components.put_kept(KeptComponent {
                name: o.name.clone(),
                value,
            }),
        }
        Ok(())
    }

    /// `value`, the component after the override, read as the component's
    /// type; its problems are placed where the override wrote it.
    fn read_edited(
        &mut self,
        entry: &Entry,
        value: &Value,
        o: &ComponentOverride,
    ) -> Result<Insert, Vec<Finding>> {
        let arena = Bump::new();
        let shown = value.borrow(&arena);
        let mut resolver = DocumentResolver::new(Rc::clone(&o.names), &mut *self.resolver);
        let mut cx = Reader::with_resolver(&mut resolver);
        let insert = within(&mut cx, o.at.segments(), &mut |cx| (entry.read)(&shown, cx));
        let problems = std::mem::take(&mut cx.problems);
        self.pending.append(&mut resolver.pending);
        match insert {
            Some(insert) if problems.is_empty() => Ok(insert),
            _ if problems.is_empty() => Err(vec![invalid(o)]),
            _ => Err(problems.into_iter().map(Finding::of_diagnostic).collect()),
        }
    }
}

/// Runs `f` with `cx` at `segs`, one step at a time.
fn within<T>(cx: &mut Reader<'_>, segs: &[Seg], f: &mut dyn FnMut(&mut Reader<'_>) -> T) -> T {
    match segs.split_first() {
        None => f(cx),
        Some((seg, rest)) => cx.at(seg.clone(), |cx| within(cx, rest, f)),
    }
}

fn missing_target(o: &ComponentOverride, group: &[PendingEntity]) -> Finding {
    let instance = group[0].key.clone();
    Finding {
        problem: SceneProblem::NoSuchGeneratedEntity {
            suggestion: suggestion(&o.target, &instance, group),
            key: o.target.clone(),
            instance,
        },
        path: o.target_at.clone(),
        on_key: true,
    }
}

fn removes_missing(o: &ComponentOverride) -> Finding {
    Finding {
        problem: SceneProblem::OverrideRemovesMissing {
            target: o.target.clone(),
            name: o.name.clone(),
        },
        path: o.at.clone(),
        on_key: false,
    }
}

/// A component value that did not read, though no problem was said.
fn invalid(o: &ComponentOverride) -> Finding {
    Finding {
        problem: SceneProblem::Data {
            problem: Problem::Invalid,
            message: format!("`{}` doesn't read after the override", o.name),
        },
        path: o.at.clone(),
        on_key: false,
    }
}
