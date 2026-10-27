//! The buffered log-file sink: a handle to the writer thread.

use std::sync::Arc;
use std::sync::mpsc::{self, Sender};

use meridian_foundation::LogLevel;
use meridian_foundation::logging::{LogSink, install_sink};

use crate::FileLogConfig;
use crate::log_file_writer::{self, Message};

/// A running log-file sink.
///
/// [`start`](Self::start) spawns one background thread running a
/// current-thread `tokio` runtime; log lines reach it over a channel (the
/// game thread never touches the filesystem) and are written through a
/// buffered writer, flushed on an interval, and immediately on `Error`, so a
/// crash cannot lose the line that explains it. Only `Error`/`Warn`/`Info`
/// reach the file; `Debug`/`Trace` stay on stderr.
#[derive(Debug)]
pub struct FileLog {
    sender: Sender<Message>,
}

impl FileLog {
    /// Starts the writer thread and installs the sink into the process log.
    /// If a sink was installed before, this one still runs but receives no
    /// lines; the first configuration wins.
    ///
    /// # Panics
    ///
    /// If the operating system cannot spawn a thread.
    #[must_use]
    pub fn start(config: FileLogConfig) -> Arc<Self> {
        let (sender, receiver) = mpsc::channel::<Message>();
        std::thread::Builder::new()
            .name("meridian-log-writer".into())
            .spawn(move || log_file_writer::run(config, &receiver))
            .expect("failed to spawn the log writer thread");
        let log = Arc::new(Self { sender });
        install_sink(log.clone());
        log
    }

    /// Blocks until every line submitted so far is on disk; call before
    /// process exit if the last lines matter.
    pub fn flush(&self) {
        let (done_sender, done_receiver) = mpsc::channel();
        // Either error means the writer thread has stopped, after saying
        // why; there is nothing left to wait for.
        if self.sender.send(Message::Flush(done_sender)).is_ok() && done_receiver.recv().is_err() {
            meridian_foundation::log_warn!("log flush incomplete: the file writer stopped");
        }
    }
}

impl LogSink for FileLog {
    fn write(&self, level: LogLevel, line: &str) {
        // Debug/Trace never reach the file; a stopped writer drops lines
        // (it said why when it stopped).
        if level > LogLevel::Info {
            return;
        }
        let _sent = self.sender.send(Message::Line {
            level,
            line: line.to_owned(),
        });
    }
}
