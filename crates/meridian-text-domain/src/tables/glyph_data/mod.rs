//! The `glyf` table: glyph outlines.

mod composite;
mod contour_builder;
mod glyf;
mod raw_point;
mod simple;

pub(crate) use glyf::Glyf;
