//! `Grid`: children placed in cells of row and column definitions.

mod cells;
mod tracks;

use super::arrange::arrange as arrange_child;
use super::measure::measure as measure_child;
use super::text_measurer::TextMeasurer;
use super::value::GridLength;
use crate::properties::grid::{COLUMN_DEFINITIONS, ROW_DEFINITIONS};
use crate::{NodeId, Rect, Size, UiTree};
use cells::{Cell, auto_extents, cell_of};
use tracks::{offsets, resolve};

/// The definitions of a grid; no definitions mean one `*` track.
fn definitions(tree: &UiTree, node: NodeId) -> (Vec<GridLength>, Vec<GridLength>) {
    let mut rows: Vec<GridLength> = tree.get_or_default(node, ROW_DEFINITIONS);
    let mut columns: Vec<GridLength> = tree.get_or_default(node, COLUMN_DEFINITIONS);
    for tracks in [&mut rows, &mut columns] {
        if tracks.is_empty() {
            tracks.push(GridLength::Star(1.0));
        }
    }
    (rows, columns)
}

/// Each child with its cell.
fn placed_children(
    tree: &UiTree,
    node: NodeId,
    rows: usize,
    columns: usize,
) -> Vec<(NodeId, Cell)> {
    tree.children_vec(node)
        .into_iter()
        .map(|child| (child, cell_of(tree, child, rows, columns)))
        .collect()
}

pub(super) fn measure(
    tree: &mut UiTree,
    text: &dyn TextMeasurer,
    node: NodeId,
    room: Size,
) -> Size {
    let (rows, columns) = definitions(tree, node);
    let placed = placed_children(tree, node, rows.len(), columns.len());
    for (child, cell) in &placed {
        let offer = Size::new(offered(columns[cell.column]), offered(rows[cell.row]));
        measure_child(tree, text, *child, offer);
    }
    let (auto_rows, auto_columns) = auto_extents(tree, &placed, rows.len(), columns.len());
    let widths = resolve(&columns, &auto_columns, room.width);
    let heights = resolve(&rows, &auto_rows, room.height);
    Size::new(widths.iter().sum(), heights.iter().sum())
}

/// What a child in a track is offered while measuring: its fixed size, or
/// no limit.
fn offered(track: GridLength) -> f32 {
    match track {
        GridLength::Px(pixels) => pixels,
        GridLength::Auto | GridLength::Star(_) => f32::INFINITY,
    }
}

pub(super) fn arrange(tree: &mut UiTree, node: NodeId, size: Size) {
    let (rows, columns) = definitions(tree, node);
    let placed = placed_children(tree, node, rows.len(), columns.len());
    let (auto_rows, auto_columns) = auto_extents(tree, &placed, rows.len(), columns.len());
    let widths = resolve(&columns, &auto_columns, size.width);
    let heights = resolve(&rows, &auto_rows, size.height);
    let (left, top) = (offsets(&widths), offsets(&heights));
    for (child, cell) in placed {
        let slot = Rect::new(
            left[cell.column],
            top[cell.row],
            widths[cell.column],
            heights[cell.row],
        );
        arrange_child(tree, child, slot);
    }
}
