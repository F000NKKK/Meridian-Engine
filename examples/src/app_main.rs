//! Crash-reporting/logging bootstrap every example's `main()` runs
//! first, identically, before `run_windowed_app`.

/// Installs the crash-report panic hook and the buffered file-log sink
/// under `app_name` — call this before anything else in `main()`, so
/// even a panic during `AppHandler::new`/`on_ready` is captured.
pub fn install_diagnostics(app_name: &str) {
    meridian_sdk::install_crash_hook(meridian_sdk::CrashReportConfig::new(app_name));
    // The sink stays installed for the whole process; the handle is only
    // needed to flush at exit.
    let _log = meridian_sdk::FileLog::start(meridian_sdk::FileLogConfig::new(app_name));
}

/// `MERIDIAN_FIXED_DT=<seconds>`: every frame advances the scene by exactly
/// this much instead of the wall-clock time since the last one, so a frame
/// sequence captured at any speed plays back smoothly (video capture).
#[must_use]
pub fn fixed_dt() -> Option<f32> {
    std::env::var("MERIDIAN_FIXED_DT")
        .ok()
        .and_then(|text| text.parse().ok())
        .filter(|dt: &f32| *dt > 0.0)
}

/// The window size an example opens with: `MERIDIAN_WINDOW=<width>x<height>`
/// when set and well formed (to capture or measure at another resolution),
/// else 1024 x 768.
#[must_use]
pub fn window_size() -> (u32, u32) {
    window_size_or((1024, 768))
}

/// Like [`window_size`], with `default` when `MERIDIAN_WINDOW` is unset or malformed.
#[must_use]
pub fn window_size_or(default: (u32, u32)) -> (u32, u32) {
    std::env::var("MERIDIAN_WINDOW")
        .ok()
        .and_then(|text| {
            let (width, height) = text.split_once('x')?;
            Some((width.parse().ok()?, height.parse().ok()?))
        })
        .filter(|(width, height)| *width > 0 && *height > 0)
        .unwrap_or(default)
}
