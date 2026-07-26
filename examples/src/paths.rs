//! Asset-path resolution shared by every example.

/// Joins `relative` onto this crate's own `CARGO_MANIFEST_DIR` —
/// `env!("CARGO_MANIFEST_DIR")` resolves to `examples/` regardless of
/// which file in this crate expands it, so every example (and this
/// shared crate itself) resolves the same way. Asset/scene loaders in
/// `meridian_sdk` deliberately don't assume any particular crate's
/// manifest directory (they're a shared dependency of every
/// application), so this join has to live at the call site.
pub fn asset_path(relative: &str) -> String {
    format!("{}/{}", env!("CARGO_MANIFEST_DIR"), relative)
}
