//! Hot reload of a `.mel` UI document (backlog item 8): re-applies the
//! changed attributes of a re-parsed document to the elements already in
//! a `UiTree`, and watches the files that feed it.

mod file_stamp;
mod hot_document;
mod live_ui;
mod rebuilt;
mod refreshed;
mod reload;
mod reload_error;
mod source_watcher;
mod tree_builder;

pub use hot_document::HotDocument;
pub use live_ui::LiveUi;
pub use rebuilt::Rebuilt;
pub use refreshed::Refreshed;
pub use reload::Reload;
pub use reload_error::ReloadError;
pub use source_watcher::SourceWatcher;

#[cfg(test)]
mod tests;
