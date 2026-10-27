# meridian-diagnostics-fs

Diagnostics adapter: the buffered log-file sink and the crash-report panic hook.

**Layer and context:** infrastructure · diagnostics

- **For:** `FileLog` (implements the `LogSink` port of `meridian-diagnostics`; a writer thread with a current-thread tokio runtime, size-free daily retention) and `crash_hook::install` (a panic hook that writes a post-mortem file with the tail of the log).
- **Not for:** formatting or filtering log lines, or the `log_*!` macros, which are the kernel `meridian-diagnostics`; choosing when to start either facility, which is the composition root (`meridian-sdk`, examples).

## Usage

```rust,no_run
use meridian_diagnostics_fs::{CrashReportConfig, FileLog, FileLogConfig, install_crash_hook};

install_crash_hook(CrashReportConfig::new("my-app"));
let log = FileLog::start(FileLogConfig::new("my-app"));
meridian_diagnostics::log_info!("started");
log.flush();
```

## Depends on / used by

- Depends on: [`meridian-diagnostics`](../meridian-diagnostics)
- Used by: [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
