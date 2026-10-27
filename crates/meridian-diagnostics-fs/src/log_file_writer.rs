//! The log-file writer thread and its open file.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use meridian_diagnostics::LogLevel;

use crate::FileLogConfig;
use crate::log_retention::clean_old_logs;

/// What the producer side sends to the writer thread.
#[derive(Debug)]
pub(crate) enum Message {
    Line { level: LogLevel, line: String },
    Flush(Sender<()>),
}

/// The dedicated writer thread: a current-thread tokio runtime
/// draining the channel into a buffered async writer.
pub(crate) fn run(config: FileLogConfig, rx: &Receiver<Message>) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            meridian_diagnostics::log_error!(
                "file logging disabled: failed to start tokio runtime: {err}"
            );
            return;
        }
    };
    runtime.block_on(async move {
        let Some(mut writer) = LogFileWriter::open(config).await else {
            return;
        };
        let flush_every = std::time::Duration::from_millis(writer.config.flush_interval_ms.max(1));
        // The producer side is a plain std channel (callers must
        // never need an async context to log). Blocking on it here is
        // safe: this current-thread runtime exists only for this
        // loop, and recv and the awaited writes strictly alternate.
        while writer.handle(rx.recv_timeout(flush_every)).await {}
    });
}

/// The open log file and the writer thread's bookkeeping.
struct LogFileWriter {
    config: FileLogConfig,
    path: PathBuf,
    writer: tokio::io::BufWriter<tokio::fs::File>,
    dirty: bool,
    last_cleanup: std::time::Instant,
}

impl LogFileWriter {
    /// Creates the directory and today's file, after a retention pass.
    /// Prints why and returns `None` when the file cannot be created.
    async fn open(config: FileLogConfig) -> Option<Self> {
        if let Err(err) = tokio::fs::create_dir_all(&config.directory).await {
            meridian_diagnostics::log_error!(
                "file logging disabled: cannot create {}: {err}",
                config.directory.display()
            );
            return None;
        }
        clean_old_logs(&config).await;
        let unix_seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let path = config
            .directory
            .join(format!("{}-{unix_seconds}.log", config.app_name));
        match tokio::fs::File::create(&path).await {
            Ok(file) => Some(Self {
                config,
                path,
                writer: tokio::io::BufWriter::new(file),
                dirty: false,
                last_cleanup: std::time::Instant::now(),
            }),
            Err(err) => {
                meridian_diagnostics::log_error!(
                    "file logging disabled: cannot create {}: {err}",
                    path.display()
                );
                None
            }
        }
    }

    /// Handles one channel event; `false` once the writer must stop.
    async fn handle(
        &mut self,
        message: Result<Message, std::sync::mpsc::RecvTimeoutError>,
    ) -> bool {
        let written = match message {
            Ok(Message::Line { level, line }) => self.write_line(level, &line).await,
            Ok(Message::Flush(done)) => {
                let flushed = self.flush().await;
                // The caller of `flush` may have stopped waiting; then
                // nobody is left to tell.
                let _told: bool = done.send(()).is_ok();
                flushed
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => self.idle().await,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                if let Err(err) = self.flush().await {
                    meridian_diagnostics::log_warn!(
                        "file logging: final flush of {} failed: {err}",
                        self.path.display()
                    );
                }
                return false;
            }
        };
        if let Err(err) = written {
            meridian_diagnostics::log_error!(
                "file logging disabled: write to {} failed: {err}",
                self.path.display()
            );
            return false;
        }
        true
    }

    /// Appends `line`; an Error may be the process's last words, so it
    /// is flushed at once instead of sitting in the buffer.
    async fn write_line(&mut self, level: LogLevel, line: &str) -> std::io::Result<()> {
        use tokio::io::AsyncWriteExt;

        self.writer.write_all(line.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.dirty = true;
        if level == LogLevel::Error {
            self.flush().await?;
        }
        Ok(())
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        use tokio::io::AsyncWriteExt;

        self.dirty = false;
        self.writer.flush().await
    }

    /// A quiet interval: flush pending lines, run the daily retention pass.
    async fn idle(&mut self) -> std::io::Result<()> {
        if self.last_cleanup.elapsed() > std::time::Duration::from_secs(24 * 60 * 60) {
            clean_old_logs(&self.config).await;
            self.last_cleanup = std::time::Instant::now();
        }
        if self.dirty {
            self.flush().await?;
        }
        Ok(())
    }
}
