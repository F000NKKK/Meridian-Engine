# ADR 005: Driver/core separation per subsystem

## Status

Accepted

## Context

Graphics, audio, physics and compute all have a hardware- or OS-facing
low-level layer (GPU device calls, audio device streams, broad-phase
collision structures, SIMD/GPU compute dispatch) and a domain-logic layer
built on top of it (materials and render graphs, mixers and DSP, rigid
bodies and constraints, scheduled compute work). Mixing the two in one
crate makes it impossible to swap the low-level backend (e.g. add a second
graphics API) without touching domain logic, and tempts the low-level layer
into leaking domain concepts downward.

## Decision

Every such subsystem splits into a `*-driver` crate and a `*-core` crate:
`graphics-driver`/`graphics-core`, `audio-driver`/`audio-core`,
`physics-driver`/`physics-core`, `compute-driver`/`compute-runtime`. The
dependency edge only ever points `*-core → *-driver`, never the reverse —
enforced by [dependency-rules.md](../dependency-rules.md) rule 2. `*-driver`
crates depend only on `platform-core`, never on their own `*-core` or on any
other subsystem's `*-core`.

"Driver" was chosen over "backend" deliberately: "backend" usually names a
concrete implementation (`VulkanBackend`, `DX12Backend`); "driver" names the
abstract hardware interface those implementations satisfy. Concrete
backends (`vulkan-driver`, `dx12-driver`, `metal-driver`) are expected to
live *underneath* `graphics-driver` later, not replace it.

`compute` is the one subsystem in this list whose domain-logic layer is
named `compute-runtime`, not `compute-core`: it has no algorithms of its own
(no domain concept the way materials are to `graphics-core` or rigid bodies
are to `physics-core`) — it's dispatch infrastructure (`ComputeContext`,
`ComputeKernel`, buffers) that other crates build kernels on top of. Actual
compute algorithms live in adapter crates like `meridian-gac-compute`, one
per domain that needs batched CPU-SIMD/GPU execution — see
[ADR 007](007-batch-transforms-via-compute.md).

## Alternatives considered

- **One crate per subsystem** (no driver/core split) — simpler short-term,
  rejected because swapping or adding a backend (e.g. a second graphics
  API) would require touching domain logic that has no business knowing
  about the backend change.
- **Backend-specific crates depending directly on domain crates** (e.g. a
  `vulkan-driver` depending on `graphics-core` to know what to render) —
  inverts the intended dependency direction and was rejected outright; see
  rule 2.

## Consequences

- Adding a second graphics/audio/physics backend later is additive (a new
  crate implementing the driver's abstraction) rather than a `graphics-core`
  refactor.
- Every `*-core` crate's implementation can be developed and tested against
  its driver's abstraction without a concrete backend existing yet.
- This roughly doubles the crate count per subsystem versus a merged
  design — accepted as the cost of keeping the swap-a-backend property.
