//! Where and under what name crash reports are written.

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
    /// Reports for `app_name` under `./crashes`.
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            directory: PathBuf::from("crashes"),
        }
    }

    /// Writes reports into `directory` instead.
    #[must_use]
    pub fn with_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = directory.into();
        self
    }
}
