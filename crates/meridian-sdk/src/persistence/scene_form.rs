//! [`SceneForm`]: which spelling of a document is written.

/// The form a saved document takes. Text and binary of one file kind share
/// an extension and are told apart by the `SDAT` magic on load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneForm {
    /// Editable text (`.mscene` as an editor writes it).
    Text,
    /// The `SDAT` binary form (cooked content).
    Binary,
}
