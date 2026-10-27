# meridian-task-core

Job graph scheduler: worker threads and dependency tracking for parallel frame execution, and the engine-core `JobExecutor` adapter.

**Layer and context:** infrastructure (adapter) · runtime

- **For:** `JobGraph`: named jobs with dependencies, a `Scheduler` that runs them on worker threads, and `TaskExecutor`, the implementation of `meridian-engine-core`'s `JobExecutor` port.
- **Not for:** domain-specific work: frame stages are composed in `meridian-engine-core`.

## Usage

```rust
use meridian_task_core::{JobGraph, Scheduler};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

let counter = Arc::new(AtomicUsize::new(0));
let mut graph = JobGraph::new();
let first = graph.add_job("first", &[], {
    let counter = Arc::clone(&counter);
    move || { counter.fetch_add(1, Ordering::SeqCst); }
});
graph.add_job("second", &[first], {
    let counter = Arc::clone(&counter);
    move || { counter.fetch_add(10, Ordering::SeqCst); }
});
Scheduler::new(2).run(graph);
assert_eq!(counter.load(Ordering::SeqCst), 11);
```

## Depends on / used by

- Depends on: [`meridian-engine-core`](../meridian-engine-core)
- Used by: [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
