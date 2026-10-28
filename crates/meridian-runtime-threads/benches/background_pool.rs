//! Criterion benches for the background pool: submission-to-completion cost
//! of many tiny jobs, and the cost of a high-priority job behind a backlog.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use meridian_asset_app::{Priority, Spawn};
use meridian_runtime_threads::BackgroundPool;

const THREADS: usize = 4;

/// Counts finished jobs and wakes the waiter when all of them are done.
struct Latch {
    done: AtomicUsize,
    total: usize,
    gate: Mutex<()>,
    signal: Condvar,
}

impl Latch {
    fn new(total: usize) -> Arc<Self> {
        Arc::new(Self {
            done: AtomicUsize::new(0),
            total,
            gate: Mutex::new(()),
            signal: Condvar::new(),
        })
    }

    fn finish_one(&self) {
        if self.done.fetch_add(1, Ordering::AcqRel) + 1 == self.total {
            let _held = self
                .gate
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            self.signal.notify_all();
        }
    }

    fn wait(&self) {
        let mut held = self
            .gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while self.done.load(Ordering::Acquire) < self.total {
            held = self
                .signal
                .wait(held)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
}

fn spawn_tiny_jobs(pool: &BackgroundPool, jobs: usize, priority: Priority) {
    let latch = Latch::new(jobs);
    for _ in 0..jobs {
        let latch = Arc::clone(&latch);
        pool.spawn(priority, Box::new(move || latch.finish_one()));
    }
    latch.wait();
}

fn tiny_jobs(c: &mut Criterion) {
    let pool = BackgroundPool::new(THREADS);
    c.bench_function("background_1_job_4_threads", |b| {
        b.iter(|| spawn_tiny_jobs(&pool, 1, Priority::Normal));
    });
    c.bench_function("background_1000_jobs_4_threads", |b| {
        b.iter(|| spawn_tiny_jobs(&pool, 1_000, Priority::Normal));
    });
}

fn mixed_priorities(c: &mut Criterion) {
    let pool = BackgroundPool::new(THREADS);
    c.bench_function("background_1000_mixed_priorities_4_threads", |b| {
        b.iter_batched(
            || Latch::new(1_000),
            |latch| {
                let priorities = [Priority::Low, Priority::Normal, Priority::High];
                for i in 0..1_000 {
                    let latch = Arc::clone(&latch);
                    pool.spawn(priorities[i % 3], Box::new(move || latch.finish_one()));
                }
                latch.wait();
            },
            BatchSize::PerIteration,
        );
    });
}

criterion_group!(benches, tiny_jobs, mixed_priorities);
criterion_main!(benches);
