# Threading model — `meridian-task-core`

Not a plain thread pool. Frame work is expressed as a **job graph**: systems
declare their dependencies, and the scheduler derives execution order,
rather than the engine calling systems in a hand-written sequence.

## Shape of a frame

```text
Input
  |
Physics
  |
Animation
  |
Culling
  |
Render Prepare
  |
GPU Submit
```

Each node is a job; edges are data dependencies. Independent branches (e.g.
audio processing alongside animation) run in parallel automatically because
the graph — not a hardcoded call order — determines what can overlap.

## Mechanics

- **Worker threads** — `Scheduler::new(worker_count)` spawns exactly that
  many (`std::thread::scope`), sized by the caller to available cores.
- **Dependency tracking** — `JobGraph::add_job` takes the `JobId`s a job
  depends on; `Scheduler::run` computes in-degrees and only makes a job
  runnable once every dependency has completed. A cycle or a `JobId` from
  a different graph panics immediately (would otherwise hang forever
  waiting for an in-degree that never reaches zero) rather than running
  something order-broken.
- **Shared ready-queue, not per-worker work-stealing deques (yet)** — every
  worker pulls from one `Mutex`-guarded queue rather than each having its
  own deque with the classic steal-from-a-busy-peer protocol. This is
  correct and easy to verify by test (see `meridian-task-core`'s test
  suite: sequential-dependency, diamond-dependency, and independent-jobs
  cases, run repeatedly to catch flakiness). True lock-free work-stealing
  deques are a real throughput win at high job counts but are a separate,
  riskier piece of unsafe-adjacent work — worth doing once there's a real
  frame's worth of jobs to profile against, not before. The public API
  (`JobGraph`, `Scheduler::run`) doesn't change when that swap happens.

## Relationship to `compute-runtime`

`task-core` schedules CPU-side jobs (systems, subsystem updates).
`meridian-compute-runtime` is the separate, explicit path for SIMD/GPU compute
dispatch used by physics, rendering, and future animation/particles/AI work
— see [dependency-rules.md](dependency-rules.md) rule 5. `compute-runtime`
itself depends on `task-core` for CPU-side scheduling of that compute work,
but the two are not the same layer: `task-core` doesn't know what a compute
buffer is, and `compute-runtime` doesn't schedule general application logic.

## Determinism note

Job graph *ordering* is not currently guaranteed to be identical across
runs unless a job explicitly declares itself sequential. Deterministic
scheduling (needed for physics replay/networking) is tracked in
[roadmap.md](roadmap.md), not yet implemented.
