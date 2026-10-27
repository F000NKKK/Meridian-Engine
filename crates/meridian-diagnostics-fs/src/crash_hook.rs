//! The crash-reporting panic hook: a post-mortem file on panic.
//!
//! When anything panics, a report lands in the configured directory with the
//! panic message and location, a captured backtrace, the tail of the unified
//! log (`meridian_diagnostics::logging::recent_lines`) and basic build/OS
//! identification. The previous hook still runs afterwards, so the normal
//! stderr panic output is unchanged.

use std::io::Write;
use std::path::PathBuf;

use crate::CrashReportConfig;

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
    let (mut file, path) = create_report_file(config, unix_seconds)?;
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
    for line in meridian_diagnostics::logging::recent_lines() {
        writeln!(file, "{line}")?;
    }
    Ok(path)
}

/// Creates the report file, taking the first free `crash-<app>-<time>-<n>.txt`
/// name so that several panics within one second never overwrite each other.
fn create_report_file(
    config: &CrashReportConfig,
    unix_seconds: u64,
) -> std::io::Result<(std::fs::File, PathBuf)> {
    let mut index = 0_u32;
    loop {
        let path = config.directory.join(format!(
            "crash-{}-{unix_seconds}-{index}.txt",
            config.app_name
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((file, path)),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => index += 1,
            Err(err) => return Err(err),
        }
    }
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
