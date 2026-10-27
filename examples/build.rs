//! Compiles `assets/scenes/*.mel` into `$OUT_DIR/mel/*.g.rs` for
//! `meridian_sdk::mel_include!` (ADR 017). A malformed scene fails the
//! build here, with its file, line and column.

fn main() {
    if let Err(error) = meridian_mel_build::compile_dir("assets/scenes") {
        panic!("{error}");
    }
}
