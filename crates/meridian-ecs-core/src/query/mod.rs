//! Iteration over components: one type ([`Query`], [`QueryMut`]) or several
//! at once ([`QueryMany`], [`QueryManyMut`]).

mod fetch;
mod many;
mod many_mut;
mod selection;
mod single;
mod single_mut;
mod tuples;

pub use fetch::{Fetch, ReadFetch};
pub use many::QueryMany;
pub use many_mut::QueryManyMut;
pub use selection::{ReadSelection, Selection};
pub use single::Query;
pub use single_mut::QueryMut;

#[cfg(test)]
mod tests;
