//! What every job executor must satisfy.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{Job, JobExecutor, Shared};

fn recording_job(name: &'static str, deps: Vec<usize>, log: &Shared<Vec<&'static str>>) -> Job {
    let log = log.clone();
    Job {
        name,
        deps,
        action: Box::new(move || log.lock().push(name)),
    }
}

/// Runs the executor contract against `executor`:
///
/// - every job runs exactly once, and `execute` returns only after all did;
/// - a job runs after all of its dependencies (checked on a chain and on a
///   diamond, repeated to give a scheduling race a chance to show);
/// - an empty job list is accepted.
///
/// # Panics
///
/// Panics, naming the broken rule, when `executor` violates the contract.
pub fn run_executor_contract(executor: &dyn JobExecutor) {
    executor.execute(Vec::new());

    for _ in 0..50 {
        let log = Shared::new(Vec::new());
        executor.execute(vec![
            recording_job("a", vec![], &log),
            recording_job("b", vec![0], &log),
            recording_job("c", vec![0], &log),
            recording_job("d", vec![1, 2], &log),
        ]);
        let order = log.lock().clone();
        assert_eq!(order.len(), 4, "every job must run exactly once: {order:?}");
        let at = |name: &str| {
            order
                .iter()
                .position(|n| *n == name)
                .expect("every job ran")
        };
        assert!(
            at("a") < at("b") && at("a") < at("c"),
            "a precedes b and c: {order:?}"
        );
        assert!(
            at("d") > at("b") && at("d") > at("c"),
            "d follows b and c: {order:?}"
        );
    }

    let runs = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&runs);
    executor.execute(vec![Job {
        name: "count",
        deps: vec![],
        action: Box::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        }),
    }]);
    assert_eq!(
        runs.load(Ordering::SeqCst),
        1,
        "execute must block until jobs finish"
    );
}
