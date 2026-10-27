//! `Selection` for tuples of two to four `Fetch` elements.

use std::any::TypeId;
use std::iter::{Copied, Map, Zip};
use std::slice;

use super::selection::sealed::{ReadSelectionRows, SelectionRows};
use super::{Fetch, ReadFetch, ReadSelection, Selection};
use crate::{Archetype, Column, Entity};

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
                vec![$(TypeId::of::<$F::Component>()),+]
            }

            fn rows_mut(archetype: &mut Archetype) -> Option<Self::Rows<'_>> {
                let Archetype { entities, columns, .. } = archetype;
                let [$(Some($f)),+] =
                    columns.get_disjoint_mut([$(&TypeId::of::<$F::Component>()),+])
                else {
                    return None;
                };
                $(
                    let $f = $F::iter(
                        &mut $f.as_any_mut().downcast_mut::<Column<$F::Component>>()?.0,
                    );
                )+
                let flatten: fn(_) -> _ =
                    |$nest_pat| ($($f,)+);
                Some(entities.iter().copied().zip($nest_expr.map(flatten)))
            }
        }

        impl<$($F: Fetch),+> Selection for ($($F,)+) {}

        impl<$($F: ReadFetch),+> ReadSelectionRows for ($($F,)+) {
            fn rows(archetype: &Archetype) -> Option<Self::Rows<'_>> {
                $(
                    let $f = $F::iter_shared(
                        &archetype
                            .columns
                            .get(&TypeId::of::<$F::Component>())?
                            .as_any()
                            .downcast_ref::<Column<$F::Component>>()?
                            .0,
                    );
                )+
                let flatten: fn(_) -> _ =
                    |$nest_pat| ($($f,)+);
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
