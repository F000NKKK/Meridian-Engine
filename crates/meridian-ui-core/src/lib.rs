//! WPF-style retained UI runtime: an element tree addressed by generational
//! [`NodeId`]s, typed dependency [`Property`] values, two-pass
//! measure/arrange layout and hit testing.
//!
//! This crate is the *runtime* half of the UI model in
//! [ADR 018](../../docs/adr/018-ui-architecture.md). It owns no pixels and no
//! markup syntax: `.mel` parsing and codegen belong to `dsl-core` and
//! `mel-build` (ADR 017), and turning the laid-out tree into `Scene2D`
//! renderables is a later step above this crate.
//!
//! ```
//! use meridian_ui_core::{ElementKind, FixedAdvanceText, Size, UiTree, compute_layout};
//! use meridian_ui_core::layout::value::Length;
//! use meridian_ui_core::properties::element::WIDTH;
//!
//! let mut tree = UiTree::new();
//! let root = tree.create(ElementKind::StackPanel);
//! tree.set(root, WIDTH, Length::Px(100.0)).unwrap();
//! compute_layout(&mut tree, root, Size::new(800.0, 600.0), &FixedAdvanceText::default()).unwrap();
//! assert_eq!(tree.layout_rect(root).unwrap().size.width, 100.0);
//! ```

mod color;
mod error;
mod hit_test;
mod node;

pub mod geometry;
pub mod input;
pub mod layout;
pub mod properties;
pub mod property;
pub mod tree;

pub use color::Color;
pub use error::UiError;
pub use geometry::{Point, Rect, Size, Thickness};
pub use hit_test::hit_test;
pub use input::{
    Handled, InputEvent, InputOutcome, IntoHandled, Key, Modifiers, PointerButton, UiEvent,
    UiEventKind, UiEventTag,
};
pub use layout::{FixedAdvanceText, TextMeasurer, compute_layout};
pub use node::{ElementKind, NodeId};
pub use property::{LiteralError, Property, PropertyId, PropertyMeta, PropertyType, PropertyValue};
pub use tree::{Change, ChangeSource, FocusDirection, UiTree};
