# meridian-task-core

Job graph scheduler: worker threads and dependency tracking for parallel frame execution.

**Layer and context:** infrastructure (adapter) · runtime

- **For:** `JobGraph`: named jobs with dependencies, and a `Scheduler` that runs them on worker threads.
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

- Depends on: no workspace crate
- Used by: [`meridian-engine-core`](../meridian-engine-core)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).
