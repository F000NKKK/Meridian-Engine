//! The registry of built-in properties, grouped by owning element
//! (`Grid.Row` lives in [`grid`]). Ids are dense and unique; the
//! `unique_ids` test guards that.

pub mod border;
pub mod dock_panel;
pub mod element;
pub mod grid;
pub mod interaction;
pub mod stack_panel;
pub mod text_block;

use crate::{PropertyId, PropertyMeta};

/// Every built-in property, indexed by [`PropertyId::raw`].
pub const ALL: &[PropertyMeta] = &[
    element::WIDTH.meta(),
    element::HEIGHT.meta(),
    element::MARGIN.meta(),
    element::HORIZONTAL_ALIGNMENT.meta(),
    element::VERTICAL_ALIGNMENT.meta(),
    element::VISIBILITY.meta(),
    border::PADDING.meta(),
    stack_panel::ORIENTATION.meta(),
    dock_panel::DOCK.meta(),
    dock_panel::LAST_CHILD_FILL.meta(),
    grid::ROW.meta(),
    grid::COLUMN.meta(),
    grid::ROW_DEFINITIONS.meta(),
    grid::COLUMN_DEFINITIONS.meta(),
    text_block::TEXT.meta(),
    text_block::FONT_SIZE.meta(),
    interaction::FOCUSABLE.meta(),
    interaction::IS_HIT_TEST_VISIBLE.meta(),
    element::BACKGROUND.meta(),
    text_block::FOREGROUND.meta(),
];

/// Looks a property up by id.
pub fn meta(id: PropertyId) -> Option<&'static PropertyMeta> {
    ALL.get(usize::from(id.raw())).filter(|meta| meta.id == id)
}

/// Looks a property up by its qualified markup name (`"Grid.Row"`).
pub fn by_name(name: &str) -> Option<&'static PropertyMeta> {
    ALL.iter().find(|meta| meta.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_ids() {
        for (index, meta) in ALL.iter().enumerate() {
            assert_eq!(
                usize::from(meta.id.raw()),
                index,
                "{} is out of order",
                meta.name
            );
        }
    }

    #[test]
    fn lookup_by_name_and_id() {
        let row = by_name("Grid.Row").expect("Grid.Row is registered");
        assert_eq!(meta(row.id).map(|m| m.name), Some("Grid.Row"));
        assert!(by_name("Grid.Nope").is_none());
    }
}
