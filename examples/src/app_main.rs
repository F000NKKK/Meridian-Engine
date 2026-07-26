//! Crash-reporting/logging bootstrap every example's `main()` runs
//! first, identically, before `run_windowed_app`.

/// Installs the crash-report panic hook and the buffered file-log sink
/// under `app_name` — call this before anything else in `main()`, so
/// even a panic during `AppHandler::new`/`on_ready` is captured.
pub fn install_diagnostics(app_name: &str) {
    meridian_sdk::crash_reporting::install(meridian_sdk::CrashReportConfig::new(app_name));
    meridian_sdk::logging::file::init(meridian_sdk::logging::file::FileLogConfig::new(app_name));
}
