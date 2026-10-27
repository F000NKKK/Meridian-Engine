//! [`CodeWriter`]: accumulates generated source line by line.

/// On every generated item: a code-behind uses only part of the module,
/// and an `expect` would fail for the parts it does use.
pub(crate) const ALLOW: &str =
    "#[allow(dead_code, reason = \"generated from markup; a code-behind uses only part of it\")]";

#[derive(Default)]
pub(crate) struct CodeWriter {
    pub(crate) text: String,
}

impl CodeWriter {
    pub(crate) fn line(&mut self, line: impl AsRef<str>) {
        self.text.push_str(line.as_ref());
        self.text.push('\n');
    }
}
