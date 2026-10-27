//! Deletes old log files.

use crate::FileLogConfig;

/// Deletes `<app_name>-*.log` files older than the retention window.
pub(crate) async fn clean_old_logs(config: &FileLogConfig) {
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
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > retention);
        if old_enough && let Err(err) = tokio::fs::remove_file(entry.path()).await {
            // Retention is best effort: a file we cannot delete stays.
            meridian_foundation::log_warn!("file logging: cannot remove old log {name}: {err}");
        }
    }
}
