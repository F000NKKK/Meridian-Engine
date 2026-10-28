//! `Selection` for tuples of two to four `Fetch` elements.

use std::any::TypeId;
use std::iter::{Copied, Map, Zip};
use std::slice;

use super::columns::{typed, typed_mut};
use super::selection::sealed::{ReadSelectionRows, SelectionRows};
use super::{Fetch, ReadFetch, ReadSelection, Selection};
use crate::{Archetype, Entity, Tick};

/// Implements the query traits for one tuple arity. `nest_*` describe how
/// the per-column iterators are zipped (`((a, b), c)` for three), and the
/// final `map` flattens that nesting into `(a, b, c)`.
macro_rules! impl_query_tuple {
    ([$($F:ident $f:ident),+], $nest_ty:ty, $nest_pat:pat, $nest_expr:expr) => {
        impl<$($F: Fetch),+> SelectionRows for ($($F,)+) {
            type Rows<'w> = Zip<
                Copied<slice::Iter<'w, Entity>>,
                Map<$nest_ty, fn(<$nest_ty as Iterator>::Item) -> Self::RowItem<'w>>,
            >;
            type RowItem<'w> = ($($F::Item<'w>,)+);

            fn component_ids() -> Vec<TypeId> {
                [$(($F::ACCESSES_COLUMN).then(TypeId::of::<$F::Component>)),+]
                    .into_iter()
                    .flatten()
                    .collect()
            }

            fn rows_mut(archetype: &mut Archetype, since: Tick, now: Tick) -> Option<Self::Rows<'_>> {
                let Archetype { entities, columns, .. } = archetype;
                $(
                    if !$F::admits(
                        columns.get(&TypeId::of::<$F::Component>()).map(|column| column.changed()),
                        since,
                    ) {
                        return None;
                    }
                )+
                $( let mut $f = None; )+
                // One pass over the columns hands each accessed one to the
                // element that wants it. Accessed types are pairwise distinct
                // (checked in `QueryManyMut::new`), so no column goes twice.
                for (id, column) in columns.iter_mut() {
                    $(
                        if $F::ACCESSES_COLUMN && *id == TypeId::of::<$F::Component>() {
                            if $F::WRITES {
                                column.mark_changed(now);
                            }
                            $f = Some(column);
                        } else
                    )+ {}
                }
                let rows = entities.len();
                $( let $f = $F::iter($f.and_then(|column| typed_mut(column.as_mut())), rows); )+
                let flatten: fn(_) -> _ = |$nest_pat| ($($f,)+);
                Some(entities.iter().copied().zip($nest_expr.map(flatten)))
            }
        }

        impl<$($F: Fetch),+> Selection for ($($F,)+) {}

        impl<$($F: ReadFetch),+> ReadSelectionRows for ($($F,)+) {
            fn rows(archetype: &Archetype, since: Tick) -> Option<Self::Rows<'_>> {
                $(
                    let id = TypeId::of::<$F::Component>();
                    if !$F::admits(archetype.columns.get(&id).map(|column| column.changed()), since) {
                        return None;
                    }
                    let $f = $F::iter_shared(
                        archetype
                            .columns
                            .get(&id)
                            .filter(|_| $F::ACCESSES_COLUMN)
                            .and_then(|column| typed(column.as_ref())),
                        archetype.entities.len(),
                    );
                )+
                let flatten: fn(_) -> _ = |$nest_pat| ($($f,)+);
                Some(archetype.entities.iter().copied().zip($nest_expr.map(flatten)))
            }
        }

        impl<$($F: ReadFetch),+> ReadSelection for ($($F,)+) {}
    };
}

impl_query_tuple!(
    [A a, B b],
    Zip<A::Iter<'w>, B::Iter<'w>>,
    (a, b),
    a.zip(b)
);
impl_query_tuple!(
    [A a, B b, C c],
    Zip<Zip<A::Iter<'w>, B::Iter<'w>>, C::Iter<'w>>,
    ((a, b), c),
    a.zip(b).zip(c)
);
impl_query_tuple!(
    [A a, B b, C c, D d],
    Zip<Zip<Zip<A::Iter<'w>, B::Iter<'w>>, C::Iter<'w>>, D::Iter<'w>>,
    (((a, b), c), d),
    a.zip(b).zip(c).zip(d)
);
