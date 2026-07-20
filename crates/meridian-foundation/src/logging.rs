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

/// The macro back end: formats one line, writes it to stderr, into the
/// crash-report ring, and (when [`file::init`] has run) hands it to the
/// buffered async file sink. Use the `log_*!` macros instead of calling
/// this directly (they fill in the module path as the target).
pub fn log(level: LogLevel, target: &str, args: std::fmt::Arguments<'_>) {
    if !enabled(level) {
        return;
    }
    let sink = sink();
    let elapsed = sink.start.elapsed().as_secs_f64();
    let line = format!("[{elapsed:9.3}] {} {target}: {args}", level.label());
    eprintln!("{line}");
    {
        let mut ring = sink
            .ring
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if ring.len() == RING_CAPACITY {
            ring.pop_front();
        }
        ring.push_back(line.clone());
    }
    #[cfg(feature = "file-logging")]
    file::submit(level, line);
}

/// Buffered, asynchronous file logging (feature `file-logging`; pulls
/// in `tokio` — the only reason this lives behind a feature is to keep
/// the crate's zero-dependency default intact for consumers that never
/// write log files).
///
/// [`init`](file::init) spawns one background thread running a
/// current-thread `tokio` runtime; log lines reach it over a channel
/// (the game thread never touches the filesystem) and are written
/// through a buffered writer, flushed on an interval — and immediately
/// on `Error`, so a crash can't lose the line that explains it. Only
/// `Error`/`Warn`/`Info` reach the file; `Debug`/`Trace` stay
/// stderr-only diagnostics.
///
/// Retention: on startup and then daily, files matching
/// `<app_name>-*.log` older than `retention_days` (default 7) are
/// deleted.
#[cfg(feature = "file-logging")]
pub mod file {
    use super::LogLevel;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    /// Configuration for the file sink.
    #[derive(Debug, Clone)]
    pub struct FileLogConfig {
        /// Log file name prefix (typically the binary name).
        pub app_name: String,
        /// Directory for log files; created on demand. Default `logs`.
        pub directory: PathBuf,
        /// Files older than this are deleted (checked at startup and
        /// daily). Default 7 days.
        pub retention_days: u64,
        /// How often the buffer is flushed to disk. Default 500 ms.
        pub flush_interval_ms: u64,
    }

    impl FileLogConfig {
        pub fn new(app_name: impl Into<String>) -> Self {
            Self {
                app_name: app_name.into(),
                directory: PathBuf::from("logs"),
                retention_days: 7,
                flush_interval_ms: 500,
            }
        }

        pub fn with_directory(mut self, directory: impl Into<PathBuf>) -> Self {
            self.directory = directory.into();
            self
        }

        pub fn with_retention_days(mut self, days: u64) -> Self {
            self.retention_days = days;
            self
        }
    }

    enum Message {
        Line { level: LogLevel, line: String },
        Flush(std::sync::mpsc::Sender<()>),
    }

    static FILE_TX: OnceLock<std::sync::mpsc::Sender<Message>> = OnceLock::new();

    /// Starts the async file sink. Second and later calls are no-ops
    /// (the first configuration wins).
    pub fn init(config: FileLogConfig) {
        FILE_TX.get_or_init(|| {
            let (tx, rx) = std::sync::mpsc::channel::<Message>();
            std::thread::Builder::new()
                .name("meridian-log-writer".into())
                .spawn(move || writer_thread(config, rx))
                .expect("failed to spawn log writer thread");
            tx
        });
    }

    /// Blocks until every line submitted so far is on disk — call
    /// before process exit if the last lines matter.
    pub fn flush() {
        if let Some(tx) = FILE_TX.get() {
            let (done_tx, done_rx) = std::sync::mpsc::channel();
            if tx.send(Message::Flush(done_tx)).is_ok() {
                let _ = done_rx.recv();
            }
        }
    }

    pub(super) fn submit(level: LogLevel, line: String) {
        // Debug/Trace never reach the file — Error/Warn/Info only.
        if level > LogLevel::Info {
            return;
        }
        if let Some(tx) = FILE_TX.get() {
            let _ = tx.send(Message::Line { level, line });
        }
    }

    /// The dedicated writer thread: a current-thread tokio runtime
    /// draining the channel into a buffered async writer.
    fn writer_thread(config: FileLogConfig, rx: std::sync::mpsc::Receiver<Message>) {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
        {
            Ok(runtime) => runtime,
            Err(err) => {
                eprintln!("file logging disabled: failed to start tokio runtime: {err}");
                return;
            }
        };
        runtime.block_on(async move {
            use tokio::io::AsyncWriteExt;

            if let Err(err) = tokio::fs::create_dir_all(&config.directory).await {
                eprintln!(
                    "file logging disabled: cannot create {:?}: {err}",
                    config.directory
                );
                return;
            }
            clean_old_logs(&config).await;
            let mut last_cleanup = std::time::Instant::now();

            let unix_seconds = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let path = config
                .directory
                .join(format!("{}-{unix_seconds}.log", config.app_name));
            let file = match tokio::fs::File::create(&path).await {
                Ok(file) => file,
                Err(err) => {
                    eprintln!("file logging disabled: cannot create {path:?}: {err}");
                    return;
                }
            };
            let mut writer = tokio::io::BufWriter::new(file);
            let flush_every = std::time::Duration::from_millis(config.flush_interval_ms.max(1));
            let mut dirty = false;

            loop {
                // The producer side is a plain std channel (callers must
                // never need an async context to log). Blocking on it
                // here is safe: this current-thread runtime exists only
                // for this loop, and nothing else is pending while we
                // wait — recv and the awaited writes strictly alternate.
                let message = rx.recv_timeout(flush_every);
                match message {
                    Ok(Message::Line { level, line }) => {
                        let _ = writer.write_all(line.as_bytes()).await;
                        let _ = writer.write_all(b"\n").await;
                        dirty = true;
                        // An Error may be the process's last words —
                        // never leave it sitting in the buffer.
                        if level == LogLevel::Error {
                            let _ = writer.flush().await;
                            dirty = false;
                        }
                    }
                    Ok(Message::Flush(done)) => {
                        let _ = writer.flush().await;
                        dirty = false;
                        let _ = done.send(());
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        if dirty {
                            let _ = writer.flush().await;
                            dirty = false;
                        }
                        if last_cleanup.elapsed() > std::time::Duration::from_secs(24 * 60 * 60) {
                            clean_old_logs(&config).await;
                            last_cleanup = std::time::Instant::now();
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        let _ = writer.flush().await;
                        return;
                    }
                }
            }
        });
    }

    /// Deletes `<app_name>-*.log` files older than the retention window.
    async fn clean_old_logs(config: &FileLogConfig) {
        let retention = std::time::Duration::from_secs(config.retention_days * 24 * 60 * 60);
        let prefix = format!("{}-", config.app_name);
        let Ok(mut entries) = tokio::fs::read_dir(&config.directory).await else {
            return;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if !name.starts_with(&prefix) || !name.ends_with(".log") {
                continue;
            }
            let Ok(metadata) = entry.metadata().await else {
                continue;
            };
            let old_enough = metadata
                .modified()
                .ok()
                .and_then(|m| m.elapsed().ok())
                .is_some_and(|age| age > retention);
            if old_enough {
                let _ = tokio::fs::remove_file(entry.path()).await;
            }
        }
    }
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

    #[cfg(feature = "file-logging")]
    #[test]
    fn file_sink_writes_flushes_and_cleans_old_logs() {
        let dir = std::env::temp_dir().join(format!("meridian-log-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // A stale log that the retention pass must delete: mtime can't
        // be set portably without external crates, so use retention of
        // zero days — everything pre-existing is "too old".
        let stale = dir.join("file-log-test-1.log");
        std::fs::write(&stale, "old").unwrap();

        set_max_level(LogLevel::Info);
        file::init(
            file::FileLogConfig::new("file-log-test")
                .with_directory(&dir)
                .with_retention_days(0),
        );
        crate::log_info!("file sink info line");
        crate::log_error!("file sink error line");
        crate::log_debug!("file sink debug line must not reach the file");
        file::flush();

        assert!(!stale.exists(), "retention must remove the stale file");
        let current: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().collect();
        assert_eq!(current.len(), 1, "exactly the current log file");
        let content = std::fs::read_to_string(current[0].path()).unwrap();
        assert!(content.contains("file sink info line"));
        assert!(content.contains("file sink error line"));
        assert!(!content.contains("debug line must not"));

        let _ = std::fs::remove_dir_all(&dir);
    }

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
