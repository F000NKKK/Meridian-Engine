//! Criterion benches for the job-graph scheduler: per-run overhead, queue
//! contention on a wide graph, and a dependency chain.

use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use criterion::{Criterion, criterion_group, criterion_main};
use meridian_runtime_threads::{JobGraph, Scheduler};

const WORKERS: usize = 4;

fn wide_graph(jobs: usize, counter: &Arc<AtomicU64>) -> JobGraph {
    let mut graph = JobGraph::new();
    for _ in 0..jobs {
        let counter = Arc::clone(counter);
        graph.add_job("tiny", &[], move || {
            counter.fetch_add(1, Ordering::Relaxed);
        });
    }
    graph
}

fn chain_graph(jobs: usize, counter: &Arc<AtomicU64>) -> JobGraph {
    let mut graph = JobGraph::new();
    let mut previous = Vec::new();
    for _ in 0..jobs {
        let counter = Arc::clone(counter);
        let id = graph.add_job("link", &previous, move || {
            counter.fetch_add(1, Ordering::Relaxed);
        });
        previous = vec![id];
    }
    graph
}

fn run_empty_graph_of_one_job(c: &mut Criterion) {
    let scheduler = Scheduler::new(WORKERS);
    let counter = Arc::new(AtomicU64::new(0));
    c.bench_function("run_1_job_4_workers", |b| {
        b.iter(|| scheduler.run(black_box(wide_graph(1, &counter))));
    });
}

fn run_wide_graph(c: &mut Criterion) {
    let scheduler = Scheduler::new(WORKERS);
    let counter = Arc::new(AtomicU64::new(0));
    c.bench_function("run_wide_10000_jobs_4_workers", |b| {
        b.iter(|| scheduler.run(black_box(wide_graph(10_000, &counter))));
    });
}

fn run_chain(c: &mut Criterion) {
    let scheduler = Scheduler::new(WORKERS);
    let counter = Arc::new(AtomicU64::new(0));
    c.bench_function("run_chain_1000_jobs_4_workers", |b| {
        b.iter(|| scheduler.run(black_box(chain_graph(1000, &counter))));
    });
}

criterion_group!(
    benches,
    run_empty_graph_of_one_job,
    run_wide_graph,
    run_chain
);
criterion_main!(benches);
