# ADR 009: Async I/O via tokio, scoped to genuine I/O only

## Status

Accepted

## Context

`meridian-graphics-driver`'s real `wgpu` backend (see
[ADR](../roadmap.md) "Not yet decided" wgpu entry) introduced this
workspace's first operations with genuinely unbounded, externally-
determined completion time: `Device::new` (an OS/driver handshake to
find and open a GPU) and `Device::read_buffer` (waiting for in-flight
GPU work to finish). The first version of this backend bridged `wgpu`'s
own `async` API to a synchronous `Device::new`/`read_buffer` via
`pollster::block_on`, on the reasoning (recorded in the original wgpu
roadmap entry) that pulling in a full async runtime for one call wasn't
worth it.

That reasoning held only while GPU acquisition was the *only* I/O-shaped
operation in the workspace. It stops holding once the engine is expected
to do I/O more generally — file/asset loading, a real window/input event
loop, an audio output device, eventually networked replication — all of
which share the same shape: unbounded wait, no useful CPU work to do
while waiting, and (for anything beyond a single blocking call) benefit
from being composed with other concurrent waits instead of serialized
behind them one `pollster::block_on` at a time. The workspace is meant to
be an async-native engine, not a sync engine with isolated
`pollster`-wrapped exceptions.

## Decision

**`tokio` is the workspace's async runtime**, added once to
`[workspace.dependencies]` and pulled in by any crate that has a real I/O
operation to perform. `meridian-graphics-driver` is the first consumer:
`Device::new` and `Device::read_buffer` are real `async fn`s now, not
`pollster`-wrapped synchronous functions. `Device::read_buffer` still has
to manually pump `wgpu::Device::poll` to drive its `map_async` completion
callback (`wgpu` has no reactor integration of its own), which is itself
a blocking call — done via `tokio::task::spawn_blocking` so it can't
stall other work sharing the runtime's worker threads while it waits.

**Async is scoped to genuine I/O, not applied uniformly.** "Genuine I/O"
means: completion time is unbounded and determined by something outside
this process (an OS/driver handshake, a GPU/device queue, a filesystem,
a network peer) — the textbook definition, not "any function that talks
to an external system's *type*." Recording/allocation calls on
`meridian-graphics-driver::Device` (`create_buffer`, `create_texture`,
`create_shader`, `create_compute_pipeline`, `write_buffer`,
`CommandBuffer::submit`, ...) stay plain synchronous functions: they're
local validation-and-enqueue work with bounded, effectively-instant cost
— the same reason `Vec::push` isn't `async`. Making them `async` would
add executor/`Future`-polling overhead for no correctness or throughput
benefit, and would make ordinary hot-path code (a physics step building
GPU command buffers every frame, say) pay `async fn` state-machine cost
it has nothing to gain from. The same distinction applies to every future
consumer of this decision: `asset-core`'s decoders take `&[u8]` today (no
filesystem access at all — see that crate's module doc) and stay
synchronous even once something adds real file loading *around* them;
only the actual `read`-from-disk call at that new call site becomes
`async`, not the decoder.

**Not every crate needs `tokio` as a dependency.** Only a crate with a
genuine I/O operation of its own adds it — `platform-core::Window`/
`DynamicLibrary` (still stubs) and `audio-driver::AudioDevice`/
`AudioStream` (still stubs) will each add `tokio` when their real
implementations land, the same way `graphics-driver` just did; nothing
about this decision forces `tokio` onto `ecs-core`, `gac-core`,
`physics-core`, or any other crate with no I/O of its own.

## Alternatives considered

- **Keep `pollster::block_on` at every I/O boundary, workspace-wide** —
  rejected: this is the status quo this ADR reverses. It composes badly
  (each blocking call serializes on its own OS thread instead of yielding
  to other concurrent waits) and doesn't scale past "exactly one blocking
  call in the whole engine," which `graphics-driver` alone already
  exceeds (`Device::new` *and* `read_buffer`, with more I/O-shaped
  operations coming as `Window`/`AudioDevice`/asset loading go from stub
  to real).
- **`smol`/`async-std`** — lighter-weight, smaller transitive dependency
  tree than `tokio`. Rejected: `tokio` is the de facto standard in the
  Rust ecosystem, with the broadest compatibility surface for anything
  this engine might need later (HTTP clients for asset downloads, `wgpu`
  examples/ecosystem code already assuming `tokio` in practice even
  though `wgpu` itself is runtime-agnostic); the extra weight is an
  acceptable, one-time cost for an engine that already accepted `wgpu`'s
  own heavy transitive tree in the prior decision.
- **A custom minimal poll-based executor in `engine-core`** — rejected:
  reinventing an executor is a well-known trap (correct waker
  implementations, work-stealing, timer wheels — `task-core`'s own
  scheduler doc already documents avoiding exactly this kind of
  reinvention for CPU dispatch, and the same reasoning applies here more
  strongly since async I/O correctness bugs are subtle and easy to get
  wrong). Zero-external-dependencies is not an absolute rule in this
  workspace — `wgpu` is the precedent — and an async runtime is exactly
  the kind of foundational, hard-to-get-right infrastructure that
  precedent is meant to cover.

## Consequences

- `meridian-graphics-driver::Device::new`/`read_buffer` are real `async
  fn`s; every call site (currently just this crate's own tests, via
  `#[tokio::test]`) must run inside a `tokio` runtime.
- `pollster` is no longer a dependency anywhere in the workspace —
  removed from `graphics-driver`'s `Cargo.toml` and
  `[workspace.dependencies]`.
- Future real I/O (windowing, audio device, file-based asset loading,
  networking) has a settled runtime to build on rather than each crate
  choosing its own bridging strategy or its own runtime.
- Recording/allocation-shaped APIs across the workspace stay synchronous
  by design — this decision does not open the door to "make everything
  `async` for consistency"; see "Decision" above for the concrete
  boundary and why it's drawn where it is.
