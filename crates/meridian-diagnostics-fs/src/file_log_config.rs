//! Configuration of the log-file sink.

use std::path::PathBuf;

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
    /// Logs for `app_name` under `./logs`, kept 7 days, flushed every 500 ms.
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            directory: PathBuf::from("logs"),
            retention_days: 7,
            flush_interval_ms: 500,
        }
    }

    /// Writes into `directory` instead.
    #[must_use]
    pub fn with_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = directory.into();
        self
    }

    /// Keeps files for `days` days.
    #[must_use]
    pub fn with_retention_days(mut self, days: u64) -> Self {
        self.retention_days = days;
        self
    }
}
