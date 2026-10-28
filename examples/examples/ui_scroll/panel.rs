//! The UI tree: a scroll viewer over a column of rows.

use meridian_sdk::ui_core::layout::value::Length;
use meridian_sdk::ui_core::properties::element::{BACKGROUND, HEIGHT, MARGIN, WIDTH};
use meridian_sdk::ui_core::properties::scroll_viewer::VERTICAL_OFFSET;
use meridian_sdk::ui_core::{
    Color, ElementKind, FixedAdvanceText, NodeId, Property, PropertyType, Size, Thickness, UiTree,
    compute_layout,
};
use meridian_sdk::ui_scene2d::{ExtractedScene, UiPaint, extract_clipped};

const ROWS: u8 = 12;
const ROW_HEIGHT: f32 = 40.0;
const VIEW: [f32; 2] = [260.0, 300.0];

/// A scroll viewer whose window shows 7.5 of its 12 rows.
pub(crate) struct Panel {
    tree: UiTree,
    root: NodeId,
    viewer: NodeId,
}

impl Panel {
    pub(crate) fn new() -> Self {
        let mut tree = UiTree::new();
        let root = tree.create(ElementKind::Border);
        let viewer = tree.create(ElementKind::ScrollViewer);
        set(&mut tree, viewer, WIDTH, Length::Px(VIEW[0]));
        set(&mut tree, viewer, HEIGHT, Length::Px(VIEW[1]));
        set(
            &mut tree,
            viewer,
            MARGIN,
            Thickness::new(24.0, 24.0, 0.0, 0.0),
        );
        set(
            &mut tree,
            viewer,
            BACKGROUND,
            Color::new(0.02, 0.02, 0.06, 0.7),
        );
        let rows = tree.create(ElementKind::StackPanel);
        append(&mut tree, viewer, rows);
        for row in 0..ROWS {
            let cell = tree.create(ElementKind::Border);
            set(&mut tree, cell, HEIGHT, Length::Px(ROW_HEIGHT - 4.0));
            set(&mut tree, cell, MARGIN, Thickness::new(8.0, 2.0, 8.0, 2.0));
            set(&mut tree, cell, BACKGROUND, row_color(row));
            append(&mut tree, rows, cell);
        }
        append(&mut tree, root, viewer);
        Self { tree, root, viewer }
    }

    /// The panel at `time` seconds, scrolled to and fro, for a window of
    /// `size` logical pixels.
    pub(crate) fn extract(&mut self, size: Size, time: f32, paint: UiPaint) -> ExtractedScene {
        let travel = f32::from(ROWS) * ROW_HEIGHT - VIEW[1];
        let offset = travel * (0.5 - 0.5 * (time * 0.8).cos());
        set(&mut self.tree, self.viewer, VERTICAL_OFFSET, offset);
        compute_layout(
            &mut self.tree,
            self.root,
            size,
            &FixedAdvanceText::default(),
        )
        .expect("the panel's nodes are alive");
        extract_clipped(&self.tree, self.root, size, paint).expect("the panel's root is alive")
    }
}

fn row_color(row: u8) -> Color {
    let phase = f32::from(row) / f32::from(ROWS) * std::f32::consts::TAU;
    Color::new(
        0.55 + 0.45 * phase.sin(),
        0.55 + 0.45 * (phase + 2.1).sin(),
        0.55 + 0.45 * (phase + 4.2).sin(),
        1.0,
    )
}

fn set<T>(tree: &mut UiTree, node: NodeId, property: Property<T>, value: T)
where
    T: PropertyType,
{
    tree.set(node, property, value)
        .expect("the panel's nodes are alive");
}

fn append(tree: &mut UiTree, parent: NodeId, child: NodeId) {
    tree.append_child(parent, child)
        .expect("the panel's nodes are alive");
}
