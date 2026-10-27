//! Shared plumbing between this workspace's examples — not a
//! `meridian-sdk` layer (this crate isn't published, isn't a dependency
//! of anything, and everything here is windowed-demo boilerplate, not
//! engine functionality), just the code that was identical across
//! `physic_figures.rs`/`magic_figures.rs` and had no reason to be
//! copy-pasted twice: asset-path resolution ([`paths`]), reading a
//! `.mel` scene file into a built DSL tree ([`scene_loader`]), and
//! crash-reporting/logging bootstrap ([`app_main`]). The acquire/submit/
//! present render-frame sequence is SDK plumbing, not example code:
//! `meridian_sdk::render_frame`/`meridian_sdk::RenderStage`.
//!
//! Each example still owns its own `App`/`AppHandler`, its own
//! `on_ready`/`on_redraw` composition, and — for `magic_figures` — its
//! own custom `#[dsl_tag]`s; this crate only factors out the parts that
//! were byte-for-byte identical, not a framework the examples build
//! against.
//!
//! **On the `.mel` extension:** scene files here are Meridian Engine
//! Language documents — today just the tag/attribute DSL
//! (`meridian_markup`/`meridian_sdk::dsl`); the planned direction
//! (see `docs/adr/017-mel-markup-and-code-behind.md`) is WPF-style
//! code-behind: the `.mel` file stays markup, names a code-behind type
//! with `x:Class`/`x:ViewModel`, and wires it with events and compiled
//! `{x:Bind}` bindings. Nothing here implements that yet —
//! [`scene_loader::load_dsl_scene`] only parses the tag/attribute
//! subset that exists today.

#![forbid(unsafe_code)]

pub mod app_main;
pub mod paths;
pub mod scene_loader;
