//! Text: measuring through a font face and drawing it from a glyph atlas.

mod face_measurer;
mod glyph_sprites;
mod paint;

pub use face_measurer::FaceMeasurer;
pub(crate) use glyph_sprites::push_text;
pub use paint::TextPaint;
