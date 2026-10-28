//! Compiles `assets/scenes/*.meui` into `$OUT_DIR/meui/*.g.rs` for
//! `meridian_sdk::meui_include!` (ADR 017). A malformed scene fails the
//! build here, with its file, line and column.

fn main() {
    if let Err(error) = meridian_ui_mel::compile_dir("assets/scenes") {
        panic!("{error}");
    }
}
