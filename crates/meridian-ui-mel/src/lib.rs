//! The build-time half of `.meui` code-behind (ADR 017): every `.meui`
//! document becomes a generated `<name>.g.rs` in the build output, the
//! counterpart of WPF's `MainWindow.g.cs`. A code-behind module pulls it
//! in with `meridian_sdk::meui_include!("<name>")` and gets:
//!
//! - `SOURCE`, the markup (embedded with `include_str!`, so the binary
//!   carries it and Cargo rebuilds when it changes);
//! - `Name`, one enum variant per `x:Name`, with `as_str` and `ALL`;
//! - `Class` / `ViewModel`, type aliases for the root's `x:Class` /
//!   `x:ViewModel`, so a typo in the code-behind path fails the build;
//! - `build(registry)` and `find(root, name)` for a scene document;
//! - for a UI document (root is a `meridian-ui-domain` element), `build_ui`
//!   instead, which creates the elements in a `UiTree`;
//! - with `{x:Bind}` attributes, `Names` (a `NodeId` per `x:Name`),
//!   `apply_one_time_bindings`, `apply_bindings` and `write_back_bindings`
//!   against a `meridian-ui-domain` tree (ADR 018); `rustc` checks every
//!   binding against the view model.
//!
//! `.mres` resource sets (`docs/meui-resources.md`) found next to the markup
//! become `Set` structs of typed constants, and `@res:Set.name` attribute
//! values are replaced by their values.
//!
//! The document is validated while generating, so a malformed `.meui`
//! fails `cargo build` with `file:line:column`, not the program at run
//! time. Called from a `build.rs` through [`compile_dir`]; nothing here
//! runs in the game.

mod binding;
mod build_error;
mod code_writer;
mod compile;
mod generated_module;
mod hot_reload;
mod invalid_markup;
mod meui_document;
mod named_element;
mod resources;
mod source_dir;
mod ui_bindings;
mod ui_build;
mod ui_builder;
mod ui_literal;
mod ui_node;

pub(crate) use binding::snake_case;
pub use binding::{
    CompareOperator, EventHandler, Expr as BindingExpr, ExprError as BindingExprError,
    ExprErrorKind as BindingExprErrorKind, Literal, Mode as BindingMode, PropertyBinding,
    PropertyPath,
};
pub use build_error::BuildError;
pub use compile::{compile_dir, compile_file, compile_file_with};
pub use generated_module::GeneratedModule;
pub use hot_reload::{HotDocument, LiveUi, Rebuilt, Refreshed, Reload, ReloadError, SourceWatcher};
pub use invalid_markup::InvalidMarkup;
pub use meui_document::MeuiDocument;
pub use named_element::NamedElement;
pub use resources::{
    Culture, GeneratedResources, LocalResources, ResourceEntry, ResourceError, ResourceProblem,
    ResourceSet, ResourceSets, ResourceValue,
};
pub(crate) use resources::{ResourceTag, read_markup_value};
pub use ui_literal::UiLiteral;
pub use ui_node::UiNode;
