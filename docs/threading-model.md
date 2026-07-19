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

- **Worker threads** — a fixed pool sized to available cores.
- **Dependency tracking** — a job only becomes runnable once its declared
  predecessors have completed.
- **Work stealing** — idle workers pull from busy workers' queues instead of
  sitting idle while work is unevenly distributed.

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
