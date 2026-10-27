//! Which cell a child sits in, and how much its cell wants.

use crate::properties::grid::{COLUMN, ROW};
use crate::{NodeId, UiTree};

/// A child's cell, clamped into the grid.
#[derive(Debug, Clone, Copy)]
pub(super) struct Cell {
    pub(super) row: usize,
    pub(super) column: usize,
}

pub(super) fn cell_of(tree: &UiTree, child: NodeId, rows: usize, columns: usize) -> Cell {
    let row: u32 = tree.get_or_default(child, ROW);
    let column: u32 = tree.get_or_default(child, COLUMN);
    Cell {
        row: (row as usize).min(rows - 1),
        column: (column as usize).min(columns - 1),
    }
}

/// The largest desired extent per row and per column, from the children's
/// last measure.
pub(super) fn auto_extents(
    tree: &UiTree,
    placed: &[(NodeId, Cell)],
    rows: usize,
    columns: usize,
) -> (Vec<f32>, Vec<f32>) {
    let mut row_extents = vec![0.0_f32; rows];
    let mut column_extents = vec![0.0_f32; columns];
    for (child, cell) in placed {
        let wanted = tree.desired_size(*child).unwrap_or_default();
        row_extents[cell.row] = row_extents[cell.row].max(wanted.height);
        column_extents[cell.column] = column_extents[cell.column].max(wanted.width);
    }
    (row_extents, column_extents)
}
