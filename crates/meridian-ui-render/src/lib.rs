//! Extraction of a UI tree into a [`Scene2d`](meridian_graphics_core::Scene2d).
//!
//! `meridian-ui-core` owns structure, layout and input and draws nothing;
//! `graphics-core` owns the 2D scene vocabulary and knows nothing about UI.
//! This crate is the adapter between them (ADR 021): it walks a laid-out
//! [`UiTree`](meridian_ui_core::UiTree) and emits sprites in painter's order,
//! so the UI composes as the last view of a frame like any other `Scene2d`.

mod extract;
mod paint;

pub use extract::extract_scene;
pub use paint::UiPaint;
