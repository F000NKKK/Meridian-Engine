//! Walking the tree into sprites.

use meridian_gac_core::{Motor3, Vec3};
use meridian_graphics_core::{Camera2d, Scene2d, Sprite};
use meridian_ui_core::layout::value::Visibility;
use meridian_ui_core::properties::element::{BACKGROUND, VISIBILITY};
use meridian_ui_core::{NodeId, Point, Rect, Size, UiError, UiTree};

use crate::UiPaint;

/// Extracts the visible subtree under `root` into a screen-space
/// [`Scene2d`] whose camera is `surface` logical pixels, origin top-left,
/// `y` down.
///
/// Elements draw in document order, children over their parent and later
/// siblings over earlier ones, as ascending [`Sprite::layer`]s. A `Hidden`
/// or `Collapsed` element hides its whole subtree. Each sprite's frame
/// translation is the center of its quad, in surface pixels.
///
/// # Errors
///
/// [`UiError::StaleNode`] when `root` is not in `tree`.
pub fn extract_scene(
    tree: &UiTree,
    root: NodeId,
    surface: Size,
    paint: UiPaint,
) -> Result<Scene2d, UiError> {
    tree.kind(root)?;
    let mut extraction = Extraction {
        tree,
        paint,
        sprites: Vec::new(),
    };
    extraction.visit(root, Point::new(0.0, 0.0));
    Ok(Scene2d {
        camera: Camera2d::from_surface_size(surface.width, surface.height),
        sprites: extraction.sprites,
    })
}

/// State of one extraction.
struct Extraction<'a> {
    tree: &'a UiTree,
    paint: UiPaint,
    sprites: Vec<Sprite>,
}

impl Extraction<'_> {
    /// Emits `node` and its children; `origin` is the parent's top-left in
    /// surface pixels.
    fn visit(&mut self, node: NodeId, origin: Point) {
        let visibility: Visibility = self.tree.get(node, VISIBILITY).unwrap_or_default();
        let Ok(local) = self.tree.layout_rect(node) else {
            return;
        };
        if visibility != Visibility::Visible {
            return;
        }
        let area = Rect {
            origin: Point::new(origin.x + local.origin.x, origin.y + local.origin.y),
            size: local.size,
        };
        self.fill_background(node, area);
        for child in self.tree.children(node).unwrap_or_default() {
            self.visit(*child, area.origin);
        }
    }

    fn fill_background(&mut self, node: NodeId, area: Rect) {
        let color = self.tree.get(node, BACKGROUND).unwrap_or_default();
        if color.is_transparent() || area.size.width <= 0.0 || area.size.height <= 0.0 {
            return;
        }
        let center = Vec3::new(
            area.origin.x + area.size.width / 2.0,
            area.origin.y + area.size.height / 2.0,
            0.0,
        );
        self.sprites.push(Sprite {
            texture: self.paint.fill,
            source_rect: None,
            size: [area.size.width, area.size.height],
            tint: color.to_array(),
            frame: Motor3::translation(center),
            layer: i32::try_from(self.sprites.len()).unwrap_or(i32::MAX),
        });
    }
}
