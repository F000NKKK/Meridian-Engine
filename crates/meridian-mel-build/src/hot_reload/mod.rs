//! Hot reload of a `.mel` UI document (backlog item 8): re-applies the
//! changed attributes of a re-parsed document to the elements already in
//! a `UiTree`.

mod live_ui;
mod reload;

pub use live_ui::LiveUi;
pub use reload::Reload;

#[cfg(test)]
mod tests;
