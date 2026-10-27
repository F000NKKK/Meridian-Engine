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
