//! Crash reporting: a panic hook that writes a post-mortem file.
//!
//! [`install`] wraps the process's panic hook. When anything panics,
//! a report lands in the configured directory containing the panic
//! message and location, a captured backtrace
//! (`std::backtrace::Backtrace`, no external crate), the tail of the
//! unified log ([`logging::recent_lines`](crate::logging::recent_lines) —
//! the "what was the engine doing" context), and basic build/OS
//! identification. The previous hook still runs afterwards, so the
//! normal stderr panic output is unchanged.
//!
//! Set `RUST_BACKTRACE=1` (or use `Backtrace::force_capture` — which
//! this module does) for symbolized frames in debug builds; release
//! builds report addresses unless debug info is kept.

use std::io::Write;
use std::path::PathBuf;

/// Where and under what name crash reports are written.
#[derive(Debug, Clone)]
pub struct CrashReportConfig {
    /// Prefix for report file names (typically the binary/app name).
    pub app_name: String,
    /// Directory for reports; created on demand. Default: `./crashes`.
    pub directory: PathBuf,
}

impl CrashReportConfig {
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            directory: PathBuf::from("crashes"),
        }
    }

    pub fn with_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = directory.into();
        self
    }
}

/// Installs the crash-reporting panic hook (idempotent per process in
/// practice — installing twice chains harmlessly, each writing its own
/// report). Call once, first thing in `main`.
pub fn install(config: CrashReportConfig) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let report_path = write_report(&config, panic_info);
        match &report_path {
            Ok(path) => eprintln!("crash report written to {}", path.display()),
            Err(err) => eprintln!("failed to write crash report: {err}"),
        }
        previous(panic_info);
    }));
}

fn write_report(
    config: &CrashReportConfig,
    panic_info: &std::panic::PanicHookInfo<'_>,
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(&config.directory)?;
    let unix_seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = config
        .directory
        .join(format!("crash-{}-{unix_seconds}.txt", config.app_name));

    let mut file = std::fs::File::create(&path)?;
    writeln!(file, "==== Meridian Engine crash report ====")?;
    writeln!(file, "app:       {}", config.app_name)?;
    writeln!(file, "time:      {unix_seconds} (unix seconds)")?;
    writeln!(
        file,
        "platform:  {} / {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )?;
    writeln!(file)?;

    let message = panic_message(panic_info);
    let location = panic_info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "<unknown location>".to_string());
    writeln!(file, "panic:     {message}")?;
    writeln!(file, "location:  {location}")?;
    writeln!(file)?;

    writeln!(file, "---- backtrace ----")?;
    writeln!(file, "{}", std::backtrace::Backtrace::force_capture())?;
    writeln!(file)?;

    writeln!(file, "---- recent log (oldest first) ----")?;
    for line in crate::logging::recent_lines() {
        writeln!(file, "{line}")?;
    }
    Ok(path)
}

/// The panic payload as text — `&str` and `String` payloads cover
/// everything `panic!`/`assert!` produce.
fn panic_message(panic_info: &std::panic::PanicHookInfo<'_>) -> String {
    if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = panic_info.payload().downcast_ref::<String>() {
        message.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The report writer is exercised directly (not via a real panic —
    /// a panicking test can't assert on its own hook's output): a
    /// panic in a spawned thread triggers the installed hook for real.
    #[test]
    fn a_panicking_thread_produces_a_report_file() {
        let dir = std::env::temp_dir().join(format!(
            "meridian-crash-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);

        crate::logging::set_max_level(crate::logging::LogLevel::Info);
        crate::log_info!("context line before the crash");
        install(CrashReportConfig::new("crash-test").with_directory(&dir));

        let result = std::thread::spawn(|| panic!("deliberate test panic"))
            .join();
        assert!(result.is_err(), "the thread must have panicked");

        let reports: Vec<_> = std::fs::read_dir(&dir)
            .expect("crash directory must exist")
            .flatten()
            .map(|e| e.path())
            .collect();
        assert_eq!(reports.len(), 1, "exactly one report expected");

        let content = std::fs::read_to_string(&reports[0]).unwrap();
        assert!(content.contains("deliberate test panic"));
        assert!(content.contains("location:"));
        assert!(content.contains("backtrace"));
        assert!(content.contains("context line before the crash"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
