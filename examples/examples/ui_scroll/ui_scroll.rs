//! A `ScrollViewer` over a stack of coloured rows, scrolled up and down by
//! the clock and drawn as an overlay: the check that a UI's clip spans
//! (`extract_clipped`, ADR 021) reach the sprite pass, so rows outside the
//! viewer's window are cut off instead of spilling over the screen.
//!
//! Design notes: docs/notes/meridian-examples.md (`examples/ui_scroll`).

mod app;
mod panel;

use app::App;
use meridian_sdk::run_windowed_app;

fn main() {
    meridian_examples::app_main::install_diagnostics("ui_scroll");
    run_windowed_app("Meridian Engine — UI Scroll", 800, 600, App::new())
        .expect("windowed app exited with an error");
}
