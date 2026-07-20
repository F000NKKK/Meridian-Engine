//! The workspace's one logging system: leveled, targeted, zero-dependency.
//!
//! Every crate logs through the same sink via the [`log_error!`],
//! [`log_warn!`], [`log_info!`], [`log_debug!`] and [`log_trace!`]
//! macros (the target is the calling module's path, automatically) —
//! no more scattered `eprintln!`s that vanish when no terminal is
//! attached. Output goes to stderr; the last messages are additionally
//! kept in an in-memory ring so a crash report
//! ([`crash_reporting`](crate::crash_reporting)) can include what the
//! engine was doing right before it died.
//!
//! This is deliberately a process-wide *diagnostics sink*, the same
//! category as `std`'s panic hook — not a resource/asset/lifetime
//! manager, which the workspace forbids (dependency-rules rule on
//! global managers). It owns no engine objects and hands out no
//! handles; it appends lines.
//!
//! The maximum level defaults to `Info` and is overridable both from
//! code ([`set_max_level`]) and from the environment
//! (`MERIDIAN_LOG=error|warn|info|debug|trace`, read once on first use).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// Log severity, most severe first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Trace = 4,
}

impl LogLevel {
    fn label(self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN ",
            LogLevel::Info => "INFO ",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }

    fn from_env(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "error" => Some(LogLevel::Error),
            "warn" | "warning" => Some(LogLevel::Warn),
            "info" => Some(LogLevel::Info),
            "debug" => Some(LogLevel::Debug),
            "trace" => Some(LogLevel::Trace),
            _ => None,
        }
    }
}

/// How many formatted lines the in-memory ring keeps for crash reports.
const RING_CAPACITY: usize = 256;

struct LogSink {
    start: Instant,
    ring: Mutex<VecDeque<String>>,
}

static SINK: OnceLock<LogSink> = OnceLock::new();
/// `u8::MAX` = "not configured yet" (resolve from env on first use).
static MAX_LEVEL: AtomicU8 = AtomicU8::new(u8::MAX);

fn sink() -> &'static LogSink {
    SINK.get_or_init(|| LogSink {
        start: Instant::now(),
        ring: Mutex::new(VecDeque::with_capacity(RING_CAPACITY)),
    })
}

fn max_level() -> u8 {
    let level = MAX_LEVEL.load(Ordering::Relaxed);
    if level != u8::MAX {
        return level;
    }
    let resolved = std::env::var("MERIDIAN_LOG")
        .ok()
        .as_deref()
        .and_then(LogLevel::from_env)
        .unwrap_or(LogLevel::Info) as u8;
    MAX_LEVEL.store(resolved, Ordering::Relaxed);
    resolved
}

/// Sets the maximum level that gets emitted (overrides `MERIDIAN_LOG`).
pub fn set_max_level(level: LogLevel) {
    MAX_LEVEL.store(level as u8, Ordering::Relaxed);
}

/// True if `level` would currently be emitted — lets callers skip
/// building expensive log arguments.
pub fn enabled(level: LogLevel) -> bool {
    (level as u8) <= max_level()
}

/// The macro back end: formats one line, writes it to stderr and into
/// the crash-report ring. Use the `log_*!` macros instead of calling
/// this directly (they fill in the module path as the target).
pub fn log(level: LogLevel, target: &str, args: std::fmt::Arguments<'_>) {
    if !enabled(level) {
        return;
    }
    let sink = sink();
    let elapsed = sink.start.elapsed().as_secs_f64();
    let line = format!("[{elapsed:9.3}] {} {target}: {args}", level.label());
    eprintln!("{line}");
    let mut ring = sink.ring.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if ring.len() == RING_CAPACITY {
        ring.pop_front();
    }
    ring.push_back(line);
}

/// The most recent formatted log lines (oldest first) — what
/// [`crash_reporting`](crate::crash_reporting) embeds in a report.
pub fn recent_lines() -> Vec<String> {
    match SINK.get() {
        Some(sink) => sink
            .ring
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .cloned()
            .collect(),
        None => Vec::new(),
    }
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logging::log($crate::logging::LogLevel::Error, module_path!(), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logging::log($crate::logging::LogLevel::Warn, module_path!(), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logging::log($crate::logging::LogLevel::Info, module_path!(), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logging::log($crate::logging::LogLevel::Debug, module_path!(), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        $crate::logging::log($crate::logging::LogLevel::Trace, module_path!(), format_args!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_order_most_severe_first() {
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }

    #[test]
    fn ring_keeps_recent_lines_and_respects_level() {
        set_max_level(LogLevel::Info);
        crate::log_info!("ring test message one");
        crate::log_debug!("must be filtered out at Info");
        crate::log_warn!("ring test message two");

        let lines = recent_lines();
        assert!(lines.iter().any(|l| l.contains("ring test message one")));
        assert!(lines.iter().any(|l| l.contains("ring test message two")));
        assert!(!lines.iter().any(|l| l.contains("must be filtered out")));
        assert!(lines.iter().any(|l| l.contains("meridian_foundation")));
    }

    #[test]
    fn ring_is_bounded() {
        set_max_level(LogLevel::Info);
        for i in 0..(RING_CAPACITY + 50) {
            crate::log_info!("flood {i}");
        }
        assert!(recent_lines().len() <= RING_CAPACITY);
    }
}
