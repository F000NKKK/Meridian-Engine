# ADR 018: Ports in the domain, drivers as adapters

## Status

Accepted. Supersedes [ADR 005](005-driver-core-separation.md).

## Context

ADR 005 split every hardware-facing subsystem into a `*-driver` crate and a
`*-core` crate and pointed the edge `*-core → *-driver`. It rejected the
opposite direction ("backend crates depending on domain crates") outright.

The workspace now follows Craftbook (https://github.com/F000NKKK/craftbook),
whose architecture chapter (7 §7.3) is domain-driven design with the
dependency rule pointing inwards: the domain and application layers own the
traits (ports) they need from the outside world, and the technology crates
(adapters) depend on them and implement them. The domain never depends on a
driver. With the ADR 005 direction:

- a domain crate (`graphics-core`, `audio-core`) compiles against `wgpu`/`cpal`
  types through its driver, so domain tests need a driver and the domain
  model is shaped by the driver's API;
- `audio-core::AudioOutput::open` is `async` because the driver's device
  handshake is, which puts I/O in a domain crate (Craftbook §4.8);
- a second backend is not additive: it must look like the first driver,
  because the domain is written against that driver's types, not a port.

The swap-a-backend goal of ADR 005 is kept; only the direction changes.

## Decision

1. **Ports belong to the layer that needs them.** I/O ports (a mesh upload, a
   frame recorder, an audio sink, a compute device) are traits in the
   subsystem's application crate, so they may be `async`. I/O-free ports (a
   clock, an id source) are synchronous traits in the domain crate.
2. **Driver crates become adapters.** `graphics-driver`, `audio-driver`,
   `compute-driver`, `gpu-driver` and the `*-compute` kernel crates are the
   infrastructure layer: they depend on the port crate and implement its
   traits. They still know nothing about scenes, materials or rigid bodies
   beyond the port's own types.
3. **Domain and application crates never depend on an adapter.** The only
   crates that name a concrete adapter are the composition root (`sdk`,
   examples).
4. **Each port has a fake and a contract suite** (Craftbook §8.4): a
   `testing` module in the port's crate, run by the fake and by each real
   adapter (hardware adapters behind the `hw-tests` feature).
5. **`physics-driver` has no port to implement.** It only reports CPU
   capabilities and was never used by `physics-core`; its dead edge is
   removed now and the crate is deleted in wave 5, its capability report
   moving to the platform context.
6. **Layers are declared once,** in `craftbook.toml`, and enforced by
   `craftcheck deps` and by `scripts/check_dependency_rules.py`, which reads
   that file. Edges that still point outwards are listed in the script as
   *migration pending*, each with the wave that removes it; the list may only
   shrink, and an outward edge not on it fails `./build.sh check-deps`.

Target names (`meridian-rendering-wgpu`, `meridian-audio-cpal`,
`meridian-compute-app`, …) and the order of the migration (wave 5: physics,
audio, compute, rendering, assets) are in Craftbook's
`adoption/meridian-engine.md`; the crate rename itself is decided in
[ADR 019](019-craftbook-adoption.md) (D4).

## Edges pending migration

At the time of this ADR the following edges contradict the decision and are
tolerated as *migration pending* (see `PENDING` in
`scripts/check_dependency_rules.py` for the live list):

| Edge | Removed by |
|---|---|
| `graphics-core → graphics-driver`, `graphics-core → gac-compute`, `graphics-core → compute-runtime` | wave 5 (rendering, compute) |
| `audio-core → audio-driver` | wave 5 (audio) |
| `physics-core → gac-compute`, `physics-core → compute-runtime` | wave 5 (compute) |
| `compute-runtime → compute-driver` | wave 5 (compute) |
| `asset-core → platform-core` | wave 5 (assets) |
| `compute-runtime → task-core`, `engine-core → task-core`, `engine-core → platform-core`, `engine-core → physics-compute` | wave 6 (runtime, [ADR 019](019-craftbook-adoption.md) D2) |

## Alternatives considered

- **Keep ADR 005 and record an exception in Craftbook.** Rejected: the
  standard does not allow local relaxations, and the exception would cover
  the whole of rendering, audio and compute.
- **Invert in one change.** Rejected: rendering alone moves `submission.rs`
  and `bloom.rs` recording into the adapter. One context per change, smallest
  first, keeps every step reviewable and green.

## Consequences

- Domain crates lose their `wgpu`/`cpal` reach; domain tests run against
  fakes, without a device.
- A second backend is a new adapter crate implementing the same ports.
- Until wave 5 ends, `docs/dependency-rules.md` describes two graphs: the
  target rule and the pending edges. The pending list is the measure of the
  migration.
