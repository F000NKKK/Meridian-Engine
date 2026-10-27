//! One sprite per glyph of a `TextBlock`.

use meridian_gac::{Motor3, Vec3};
use meridian_rendering_domain::Sprite;
use meridian_text_domain::shape;
use meridian_ui_domain::properties::text_block::{FONT_SIZE, FOREGROUND, TEXT};
use meridian_ui_domain::{NodeId, Rect, UiTree};

use super::paint::TextPaint;
use crate::RenderError;

/// Appends the glyph sprites of `node`'s text, laid out in `area`.
/// Glyph origins are snapped to whole pixels so atlas pixels land 1:1.
#[expect(
    clippy::cast_precision_loss,
    reason = "glyph bitmaps and bearings are at most a few thousand pixels"
)]
pub(crate) fn push_text(
    sprites: &mut Vec<Sprite>,
    tree: &UiTree,
    node: NodeId,
    area: Rect,
    paint: &mut TextPaint<'_>,
) -> Result<(), RenderError> {
    let text: String = tree.get(node, TEXT).unwrap_or_default();
    let size: f32 = tree.get(node, FONT_SIZE).unwrap_or_default();
    let tint = tree.get(node, FOREGROUND).unwrap_or_default().to_array();
    let run = shape(paint.face, &text, size);
    let baseline = (area.origin.y + run.ascent).round();
    for placed in &run.glyphs {
        let glyph = paint.cache.glyph(paint.face, placed.glyph, size)?;
        if glyph.rect.width == 0 || glyph.rect.height == 0 {
            continue;
        }
        let atlas = paint.cache.atlas();
        let (width, height) = (glyph.rect.width as f32, glyph.rect.height as f32);
        let left = (area.origin.x + placed.x).round() + glyph.left as f32;
        let top = baseline - glyph.top as f32;
        sprites.push(Sprite {
            texture: paint.atlas,
            source_rect: Some(glyph.rect.normalized(atlas.width(), atlas.height())),
            size: [width, height],
            tint,
            frame: Motor3::translation(Vec3::new(left + width / 2.0, top + height / 2.0, 0.0)),
            layer: i32::try_from(sprites.len()).unwrap_or(i32::MAX),
        });
    }
    Ok(())
}
