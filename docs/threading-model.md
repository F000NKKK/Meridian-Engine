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

- **Worker threads** — `Scheduler::new(worker_count)` spawns `worker_count - 1`
  threads once and parks them between runs; the thread that calls
  `Scheduler::run` is the last worker. `worker_count` is sized by the caller to
  available cores. A job that panics skips the jobs not yet started and the
  panic is re-raised from `run`. Per run: ~0.35 µs for a one-job graph
  (was ~210 µs while threads were spawned per run), ~0.33 ms for a
  1 000-job chain, ~6 ms for 10 000 independent tiny jobs, on 4 cores
  (`cargo bench -p meridian-task-core`).
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
  cases, run repeatedly to catch flakiness). Per-worker work-stealing
  deques would only help at job counts far above a frame's (the 10 000-job
  benchmark above is ~0.6 µs/job through the queue); build them when a real
  frame profile shows queue contention, not before. The public API
  (`JobGraph`, `Scheduler::run`) doesn't change when that swap happens.

## Relationship to `compute-runtime`

`task-core` schedules CPU-side jobs (systems, subsystem updates).
`meridian-compute-runtime` is the separate, explicit path for SIMD/GPU compute
dispatch used by physics, rendering, and future animation/particles/AI work
— see [dependency-rules.md](dependency-rules.md) rule 5. `compute-runtime`
itself depends on `task-core` for CPU-side scheduling of that compute work,
but the two are not the same layer: `task-core` doesn't know what a compute
buffer is, and `compute-runtime` doesn't schedule general application logic.

## `engine-core`'s `JobExecutor` port

`meridian-engine-core::Runtime` runs its stages through the `JobExecutor`
port (ADR 025): it builds a list of `Job`s (name, dependency indices, action)
every tick and hands it to the executor. `meridian-task-core::TaskExecutor`
implements the port with the `Scheduler` above, sized by the composition root
(`meridian_sdk::parallel_runtime` uses the detected CPU thread count from
`platform-core`, so `engine-core` itself never asks). Independent stages run
in parallel; a stage that depends on another sees its finished work. Stages
share state through `Shared` handles the composition root passes in, not
through a runtime-owned hub. `testing::InlineExecutor` is the deterministic
single-threaded fake.

## Determinism note

Job graph *ordering* is not currently guaranteed to be identical across
runs unless a job explicitly declares itself sequential. Deterministic
scheduling (needed for physics replay/networking) is tracked in
[roadmap.md](roadmap.md), not yet implemented.
