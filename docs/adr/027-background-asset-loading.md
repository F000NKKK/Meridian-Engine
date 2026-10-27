# ADR 027: Background asset loading on our own threads

## Status

Accepted. Supersedes the file-I/O part of [ADR 009](009-async-io-via-tokio.md):
reading files does not use `tokio`. ADR 009 stays in force for network I/O.

## Context

`asset-domain` decodes bytes that are already in memory. Reading the bytes
blocks on a disk (later a network), so a frame cannot do it inline. ADR 009
named `tokio` the runtime for genuine I/O, but for a file read that gives
nothing: `tokio::fs` is `spawn_blocking` onto a second thread pool, and every
crate that loads assets would need an executor and `async`.

The frame `Scheduler` cannot host the work either. `Scheduler::run` blocks
until a whole graph is done, and a worker parked in a blocking read would
stall the frame's own job graph.

## Decision

**D1 — a loader in the app layer, with two ports.** `meridian-asset-app`
(app layer, assets context) holds `ByteSource` (read the bytes of an
`AssetPath`), `Spawn` (run a job in the background), `AssetLoader` (read and
decode as one job) and `Pending<T>`. Decoding stays the domain's `Decoder`.

**D2 — adapters.** `meridian-asset-fs` is the file-system `ByteSource`; it
rejects a path that leaves its root. `meridian-runtime-threads` gets
`BackgroundPool`, the `Spawn` implementation: a fixed set of threads fed from
one queue per priority, separate from the frame workers.

**D3 — no `async` in the engine loop.** A load returns at once with a
`Pending`. The frame polls it (`take`) or, off the frame thread, blocks on it
(`wait`). `Pending` always resolves: a decoder panic is caught and becomes
`LoadError::Panicked`, and a job the pool never ran becomes `LoadError::Cancelled`.

**D3a — priority and cancellation.** `Spawn::spawn` takes a `Priority`
(`Low`, `Normal`, `High`); a pool starts the highest first and keeps queue
order within one. `AssetLoader::load` is `Normal`, `load_with` takes the
priority. `Pending::cancel` is a request: a load still queued is skipped and
resolves as `LoadError::Cancelled`; one already running finishes and keeps its
result. A running read is not interrupted.

**D4 — `tokio` stays for the network.** A future HTTP source is another
`ByteSource` adapter that may use `tokio` inside; the port and the loader do
not change ([ADR 028](028-http-asset-source.md) built it blocking, without
`tokio`). The rule in CLAUDE.md ("`async fn` only for genuine I/O") stands.

**D5 — lifetime stays elsewhere.** The loader hands back decoded CPU data. It
does not register ids or decide how long the data lives; that is `resource`.

## Consequences

- No new dependency: `std` threads, a mutex and a condvar only.
- The composition root (`sdk`) builds `FsSource` + `BackgroundPool` and
  passes them to `AssetLoader::new` (`sdk::asset_loader`).
