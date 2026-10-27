//! Walking the tree into sprites.

use meridian_gac::{Motor3, Vec3};
use meridian_rendering_domain::{Camera2d, Scene2d, Sprite};
use meridian_text_domain::FontError;
use meridian_ui_domain::layout::value::Visibility;
use meridian_ui_domain::properties::element::{BACKGROUND, VISIBILITY};
use meridian_ui_domain::{ElementKind, NodeId, Point, Rect, Size, UiError, UiTree};

use crate::text::{TextPaint, push_text};
use crate::{RenderError, UiPaint};

/// Extracts the visible subtree under `root` into a screen-space
/// [`Scene2d`] whose camera is `surface` logical pixels, origin top-left,
/// `y` down.
///
/// Elements draw in document order, children over their parent and later
/// siblings over earlier ones, as ascending [`Sprite::layer`]s. A `Hidden`
/// or `Collapsed` element hides its whole subtree. Each sprite's frame
/// translation is the center of its quad, in surface pixels. Backgrounds
/// only; see [`extract_scene_with_text`] for text.
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
    Ok(Extraction::new(tree, paint, None).run(root, surface).0)
}

/// [`extract_scene`] plus every `TextBlock`: one sprite per glyph, sampling
/// `text.atlas`, tinted with the block's `Foreground`.
///
/// Rasterizing a glyph can fill the glyph cache's page, which wipes it; if
/// that happens mid-walk the whole tree is walked again so every sprite
/// points at the final page. Upload `text.cache.atlas()` after this returns
/// whenever `text.cache.version()` changed.
///
/// # Errors
///
/// [`RenderError::Tree`] when `root` is not in `tree`, and
/// [`RenderError::Font`] when a glyph cannot be rasterized or the text
/// needs more than one page of glyphs.
pub fn extract_scene_with_text(
    tree: &UiTree,
    root: NodeId,
    surface: Size,
    paint: UiPaint,
    text: &mut TextPaint<'_>,
) -> Result<Scene2d, RenderError> {
    tree.kind(root)?;
    for _ in 0..2 {
        let resets = text.cache.resets();
        let (scene, failure) = Extraction::new(tree, paint, Some(&mut *text)).run(root, surface);
        if let Some(error) = failure {
            return Err(error);
        }
        if text.cache.resets() == resets {
            return Ok(scene);
        }
    }
    Err(RenderError::Font(FontError::BitmapTooLarge))
}

/// State of one extraction.
struct Extraction<'t, 'p> {
    tree: &'t UiTree,
    paint: UiPaint,
    text: Option<&'t mut TextPaint<'p>>,
    sprites: Vec<Sprite>,
    failure: Option<RenderError>,
}

impl<'t, 'p> Extraction<'t, 'p> {
    const fn new(tree: &'t UiTree, paint: UiPaint, text: Option<&'t mut TextPaint<'p>>) -> Self {
        Self {
            tree,
            paint,
            text,
            sprites: Vec::new(),
            failure: None,
        }
    }

    /// Walks from `root`; the scene plus the first failure, if any.
    fn run(mut self, root: NodeId, surface: Size) -> (Scene2d, Option<RenderError>) {
        self.visit(root, Point::new(0.0, 0.0));
        let scene = Scene2d {
            camera: Camera2d::from_surface_size(surface.width, surface.height),
            sprites: self.sprites,
        };
        (scene, self.failure)
    }

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
        self.draw_text(node, area);
        for child in self.tree.children(node).unwrap_or_default() {
            self.visit(*child, area.origin);
        }
    }

    fn draw_text(&mut self, node: NodeId, area: Rect) {
        let Some(text) = self.text.as_deref_mut() else {
            return;
        };
        if self.failure.is_some() || self.tree.kind(node) != Ok(ElementKind::TextBlock) {
            return;
        }
        if let Err(error) = push_text(&mut self.sprites, self.tree, node, area, text) {
            self.failure = Some(error);
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
