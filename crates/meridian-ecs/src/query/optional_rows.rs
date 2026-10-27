//! The rows of an optional element: the column's values, or `None` for every row.

/// Yields `Some(value)` row by row when the archetype has the column, and
/// `None` for each of `rows` rows when it has not.
#[derive(Debug)]
pub struct OptionalRows<I> {
    column: Option<I>,
    rows: usize,
}

impl<I> OptionalRows<I> {
    pub(crate) fn new(column: Option<I>, rows: usize) -> Self {
        Self { column, rows }
    }
}

impl<I: Iterator> Iterator for OptionalRows<I> {
    type Item = Option<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        self.rows = self.rows.checked_sub(1)?;
        Some(self.column.as_mut().and_then(Iterator::next))
    }
}
