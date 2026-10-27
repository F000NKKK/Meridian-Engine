//! Diagnostics adapter: the log-file sink and the crash-report hook.
//!
//! Layer: infrastructure; bounded context: diagnostics (`craftbook.toml`,
//! docs/context-map.md).
//!
//! The kernel (`meridian-foundation`) formats lines and declares the
//! `LogSink` port; [`FileLog`] implements it with a buffered writer thread,
//! and [`install_crash_hook`] writes a post-mortem when a thread panics. The
//! composition root decides to start them (ADR 020 D7).

#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod crash_hook;
mod crash_report_config;
mod file_log;
mod file_log_config;
mod log_file_writer;
mod log_retention;
#[cfg(test)]
mod tests;

pub use crash_hook::install as install_crash_hook;
pub use crash_report_config::CrashReportConfig;
pub use file_log::FileLog;
pub use file_log_config::FileLogConfig;
