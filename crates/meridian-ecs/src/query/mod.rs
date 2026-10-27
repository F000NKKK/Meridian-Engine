//! Iteration over components: one type ([`Query`], [`QueryMut`]) or several
//! at once ([`QueryMany`], [`QueryManyMut`]).

mod columns;
mod fetch;
mod filter;
mod many;
mod many_mut;
mod optional_fetch;
mod optional_rows;
mod sealed;
mod selection;
mod single;
mod single_mut;
mod tuples;

pub use fetch::{Fetch, ReadFetch};
pub use filter::{Changed, With, Without};
pub use many::QueryMany;
pub use many_mut::QueryManyMut;
pub use optional_rows::OptionalRows;
pub use selection::{ReadSelection, Selection};
pub use single::Query;
pub use single_mut::QueryMut;

#[cfg(test)]
mod tests;
