use crate::{CrashReportConfig, install_crash_hook as install};

/// The report writer is exercised directly (not via a real panic —
/// a panicking test can't assert on its own hook's output): a
/// panic in a spawned thread triggers the installed hook for real.
#[test]
fn a_panicking_thread_produces_a_report_file() {
    let dir = std::env::temp_dir().join(format!("meridian-crash-test-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }

    install(CrashReportConfig::new("crash-test").with_directory(&dir));

    let result = std::thread::spawn(|| panic!("deliberate test panic")).join();
    assert!(result.is_err(), "the thread must have panicked");

    let reports: Vec<_> = std::fs::read_dir(&dir)
        .expect("crash directory must exist")
        .flatten()
        .map(|e| e.path())
        .collect();
    // Other tests' panics also fire the (global) hook, so look for
    // *this* panic's report rather than assuming it's alone.
    let content = reports
        .iter()
        .map(|p| std::fs::read_to_string(p).unwrap())
        .find(|c| c.contains("deliberate test panic"))
        .expect("a report for the deliberate panic must exist");
    assert!(content.contains("location:"));
    assert!(content.contains("backtrace"));
    // The recent-log section exists; its exact contents race with
    // other tests sharing the global ring, so no line assertion.
    assert!(content.contains("---- recent log"));

    std::fs::remove_dir_all(&dir).unwrap();
}
