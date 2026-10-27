//! The build-time half of `.mel` code-behind (ADR 017): every `.mel`
//! document becomes a generated `<name>.g.rs` in the build output, the
//! counterpart of WPF's `MainWindow.g.cs`. A code-behind module pulls it
//! in with `meridian_sdk::mel_include!("<name>")` and gets:
//!
//! - `SOURCE`, the markup (embedded with `include_str!`, so the binary
//!   carries it and Cargo rebuilds when it changes);
//! - `Name`, one enum variant per `x:Name`, with `as_str` and `ALL`;
//! - `Class` / `ViewModel`, type aliases for the root's `x:Class` /
//!   `x:ViewModel`, so a typo in the code-behind path fails the build;
//! - `build(registry)` and `find(root, name)`;
//! - with an `x:ViewModel`, `type_check_bindings`: never called, it makes
//!   `rustc` check every `{x:Bind}` against the view model.
//!
//! The document is validated while generating, so a malformed `.mel`
//! fails `cargo build` with `file:line:column`, not the program at run
//! time. Called from a `build.rs` through [`compile_dir`]; nothing here
//! runs in the game.

mod binding;
mod build_error;
mod compile;
mod generated_module;
mod invalid_markup;
mod mel_document;
mod named_element;

pub use binding::{
    CompareOperator, Expr as BindingExpr, ExprError as BindingExprError,
    ExprErrorKind as BindingExprErrorKind, Literal, Mode as BindingMode, PropertyBinding,
};
pub use build_error::BuildError;
pub use compile::{compile_dir, compile_file};
pub use generated_module::GeneratedModule;
pub use invalid_markup::InvalidMarkup;
pub use mel_document::MelDocument;
pub use named_element::NamedElement;
