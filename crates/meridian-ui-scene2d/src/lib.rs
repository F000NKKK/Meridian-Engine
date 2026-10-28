//! Extraction of a UI tree into a [`Scene2d`](meridian_rendering_domain::Scene2d).
//!
//! `meridian-ui-domain` owns structure, layout and input and draws nothing;
//! `rendering-domain` owns the 2D scene vocabulary and knows nothing about UI.
//! This crate is the adapter between them (ADR 021): it walks a laid-out
//! [`UiTree`](meridian_ui_domain::UiTree) and emits sprites in painter's order,
//! so the UI composes as the last view of a frame like any other `Scene2d`.
//! Text is drawn from `meridian-text-domain`'s glyph atlas (ADR 023).

mod extract;
mod paint;
mod render_error;
mod text;

pub use extract::{extract_scene, extract_scene_with_text};
pub use paint::UiPaint;
pub use render_error::RenderError;
pub use text::{FaceMeasurer, TextPaint};
