use meridian_diagnostics::logging::{LogLevel, set_max_level};
use meridian_diagnostics::{log_debug, log_error, log_info};

use crate::{FileLog, FileLogConfig};

#[test]
fn file_log_writes_flushes_and_cleans_old_logs() {
    let dir = std::env::temp_dir().join(format!("meridian-log-test-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    // A stale log that the retention pass must delete: mtime cannot be set
    // portably without external crates, so use retention of zero days,
    // which makes everything pre-existing "too old".
    let stale = dir.join("file-log-test-1.log");
    std::fs::write(&stale, "old").unwrap();

    set_max_level(LogLevel::Info);
    let log = FileLog::start(
        FileLogConfig::new("file-log-test")
            .with_directory(&dir)
            .with_retention_days(0),
    );
    log_info!("file sink info line");
    log_error!("file sink error line");
    log_debug!("file sink debug line must not reach the file");
    log.flush();

    assert!(!stale.exists(), "retention must remove the stale file");
    let current: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().collect();
    assert_eq!(current.len(), 1, "exactly the current log file");
    let content = std::fs::read_to_string(current[0].path()).unwrap();
    assert!(content.contains("file sink info line"));
    assert!(content.contains("file sink error line"));
    assert!(!content.contains("debug line must not"));

    std::fs::remove_dir_all(&dir).unwrap();
}
