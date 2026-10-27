//! Tests of the scheduler.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use crate::{JobGraph, Scheduler};

#[test]
fn empty_graph_runs_without_blocking() {
    Scheduler::new(2).run(JobGraph::new());
}

#[test]
fn single_job_runs() {
    let ran = Arc::new(AtomicUsize::new(0));
    let mut graph = JobGraph::new();
    let ran2 = ran.clone();
    graph.add_job("only", &[], move || {
        ran2.fetch_add(1, Ordering::SeqCst);
    });
    Scheduler::new(1).run(graph);
    assert_eq!(ran.load(Ordering::SeqCst), 1);
}

#[test]
fn dependent_job_runs_after_its_dependency() {
    let order = Arc::new(StdMutex::new(Vec::new()));
    let mut graph = JobGraph::new();

    let order_a = order.clone();
    let a = graph.add_job("a", &[], move || order_a.lock().unwrap().push("a"));

    let order_b = order.clone();
    graph.add_job("b", &[a], move || order_b.lock().unwrap().push("b"));

    Scheduler::new(4).run(graph);
    assert_eq!(*order.lock().unwrap(), vec!["a", "b"]);
}

#[test]
fn independent_jobs_all_run_regardless_of_worker_count() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut graph = JobGraph::new();
    for _ in 0..20 {
        let c = count.clone();
        graph.add_job("independent", &[], move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
    }
    Scheduler::new(4).run(graph);
    assert_eq!(count.load(Ordering::SeqCst), 20);
}

#[test]
fn diamond_dependency_runs_in_valid_order() {
    // a -> b, a -> c, (b, c) -> d — d must see both b's and c's effects.
    let order = Arc::new(StdMutex::new(Vec::new()));
    let mut graph = JobGraph::new();

    let oa = order.clone();
    let a = graph.add_job("a", &[], move || oa.lock().unwrap().push("a"));

    let ob = order.clone();
    let b = graph.add_job("b", &[a], move || ob.lock().unwrap().push("b"));

    let oc = order.clone();
    let c = graph.add_job("c", &[a], move || oc.lock().unwrap().push("c"));

    let od = order.clone();
    graph.add_job("d", &[b, c], move || od.lock().unwrap().push("d"));

    Scheduler::new(4).run(graph);

    let order = order.lock().unwrap();
    assert_eq!(order.first(), Some(&"a"));
    assert_eq!(order.last(), Some(&"d"));
    assert_eq!(order.len(), 4);
}

#[test]
#[should_panic(expected = "dependency cycle")]
fn cyclic_graph_panics_instead_of_hanging() {
    // Build a 2-cycle by hand: add_job only accepts already-issued
    // JobIds, so a JobGraph built purely through the public API can
    // never contain a cycle. To exercise the panic path we reach past
    // the API and forge a cycle directly on the private field — this
    // is the one test in the suite allowed to do that, specifically
    // to prove the guard that protects against a malformed graph
    // (e.g. one deserialized from bad data in the future) works.
    let mut graph = JobGraph::new();
    let a = graph.add_job("a", &[], || {});
    let b = graph.add_job("b", &[a], || {});
    graph.jobs[a.0].dependencies.push(b);

    Scheduler::new(2).run(graph);
}

#[test]
fn worker_count_reports_the_configured_count_and_is_at_least_one() {
    assert_eq!(Scheduler::new(4).worker_count(), 4);
    assert_eq!(Scheduler::new(0).worker_count(), 1);
}

#[test]
fn one_scheduler_runs_many_graphs_on_the_same_workers() {
    let scheduler = Scheduler::new(4);
    let count = Arc::new(AtomicUsize::new(0));
    for _ in 0..50 {
        let mut graph = JobGraph::new();
        for _ in 0..8 {
            let c = count.clone();
            graph.add_job("tick", &[], move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        scheduler.run(graph);
    }
    assert_eq!(count.load(Ordering::SeqCst), 400);
}

#[test]
fn single_worker_scheduler_runs_a_dependency_chain_in_order() {
    let order = Arc::new(StdMutex::new(Vec::new()));
    let mut graph = JobGraph::new();
    let mut previous = Vec::new();
    for step in 0..5 {
        let order = order.clone();
        let id = graph.add_job("link", &previous, move || order.lock().unwrap().push(step));
        previous = vec![id];
    }

    Scheduler::new(1).run(graph);

    assert_eq!(*order.lock().unwrap(), vec![0, 1, 2, 3, 4]);
}

#[test]
fn concurrent_runs_on_one_scheduler_both_finish() {
    let scheduler = Arc::new(Scheduler::new(3));
    let count = Arc::new(AtomicUsize::new(0));
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let scheduler = scheduler.clone();
            let count = count.clone();
            std::thread::spawn(move || {
                let mut graph = JobGraph::new();
                for _ in 0..25 {
                    let c = count.clone();
                    graph.add_job("tick", &[], move || {
                        c.fetch_add(1, Ordering::SeqCst);
                    });
                }
                scheduler.run(graph);
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(count.load(Ordering::SeqCst), 100);
}

#[test]
fn panicking_job_reaches_the_caller_and_skips_its_dependents() {
    let scheduler = Scheduler::new(3);
    let dependent_ran = Arc::new(AtomicUsize::new(0));
    let mut graph = JobGraph::new();
    let bad = graph.add_job("bad", &[], || panic!("job failed on purpose"));
    let flag = dependent_ran.clone();
    graph.add_job("after-bad", &[bad], move || {
        flag.fetch_add(1, Ordering::SeqCst);
    });

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scheduler.run(graph)));

    let payload = result.expect_err("run must re-raise the job's panic");
    assert_eq!(
        payload.downcast_ref::<&str>(),
        Some(&"job failed on purpose")
    );
    assert_eq!(dependent_ran.load(Ordering::SeqCst), 0);
}

#[test]
fn scheduler_still_works_after_a_job_panicked() {
    let scheduler = Scheduler::new(2);
    let mut failing = JobGraph::new();
    failing.add_job("bad", &[], || panic!("first run fails"));
    let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scheduler.run(failing)));
    assert!(first.is_err());

    let ran = Arc::new(AtomicUsize::new(0));
    let mut graph = JobGraph::new();
    let flag = ran.clone();
    graph.add_job("fine", &[], move || {
        flag.fetch_add(1, Ordering::SeqCst);
    });
    scheduler.run(graph);

    assert_eq!(ran.load(Ordering::SeqCst), 1);
}

#[test]
#[should_panic(expected = "doesn't belong")]
fn foreign_job_id_panics() {
    let mut other = JobGraph::new();
    other.add_job("a", &[], || {});
    let foreign = other.add_job("b", &[], || {});
    let mut graph = JobGraph::new();
    graph.add_job("only", &[foreign], || {});

    Scheduler::new(2).run(graph);
}
