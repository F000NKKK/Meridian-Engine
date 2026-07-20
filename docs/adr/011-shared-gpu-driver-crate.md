# ADR 011: A shared meridian-gpu-driver crate underneath graphics-driver and compute-driver

## Status

Accepted

## Context

`graphics-driver` gained a real headless `wgpu` device early in this
workspace's GPU work (`Device`, `Buffer`, `Shader`, a compute pipeline +
`dispatch_compute`, alongside its own render-specific `Surface`/
`RenderPipeline`). When `compute-driver` needed its own real GPU backend
(so `compute-runtime`'s `ComputeContext` — the sanctioned path to
CPU-SIMD/GPU-compute for domain crates, per
[dependency-rules.md](../dependency-rules.md) rule 5 — could dispatch on
GPU, not just CPU), the fastest path would have been reaching directly
into `graphics-driver`'s `Device` from `compute-driver`. That's wrong on
two counts: `graphics-driver` is documented as knowing "nothing about
scenes or materials," and general GPU *compute* dispatch (a compute
pipeline bound to a buffer, unrelated to presenting a rendered frame) has
nothing to do with rendering either — putting it in `graphics-driver`
already blurred that boundary, and routing `compute-driver` through
`graphics-driver` to reach it would have made a `*-driver` crate depend
on a *different* subsystem's driver, which
[dependency-rules.md](../dependency-rules.md) rule 1's sibling rule (rule
2, no driver depends on its own core; the general principle extends to
"no driver reaches into a different hardware domain's driver either)
exists to prevent.

The alternative — `compute-driver` growing its *own*, independently
written `wgpu::Device`/`Buffer`/`Shader`/`ComputePipeline` wrapper — would
have duplicated real, non-trivial code (`wgpu` instance/adapter/device
acquisition, the buffer readback pattern via a staging buffer +
`map_async` + manual `poll`, shader compilation, compute-pipeline
creation and dispatch) between two sibling crates for no reason tied to
what makes them different. That's exactly the shape CLAUDE.md's "don't
drag another crate's logic into your own" rule exists to catch, and this
workspace already has a template for "two crates independently need the
same underlying thing": [ADR 007](007-batch-transforms-via-compute.md)'s
`meridian-gac-compute`, a third adapter crate both depend on.

## Decision

**A third crate, `meridian-gpu-driver`**, owns the actual `wgpu` device/
buffer/shader/compute-pipeline mechanics: `Device` (headless via `new`,
windowed-handshake via `new_windowed`), `Buffer`, `Shader`,
`ComputePipeline`, `BindGroup`, `CommandBuffer` (`dispatch_compute` +
`submit`), `DeviceError`. Both `graphics-driver` and `compute-driver`
depend on it and add only what's specific to their own hardware role on
top:

```text
meridian-platform-core
        |
meridian-gpu-driver
        |
   +----+----+
   |         |
graphics-  compute-
driver     driver
```

- `graphics-driver` wraps `meridian_gpu_driver::Device` in its own
  `Device` newtype (forwarding buffer/shader/texture/read methods
  directly, no reimplementation) and adds `Surface`/`SurfaceFrame`/
  `RenderPipeline`/`DepthTexture`/`VertexLayout`/`RenderPass` —
  everything that's about presenting a rendered frame, which
  `gpu-driver` deliberately doesn't know about. It no longer has its own
  compute-pipeline/`dispatch_compute` — that moved to `compute-driver`,
  where it belongs.
- `compute-driver` gets `GpuComputeDevice`, a thin compute-dispatch-shaped
  API (`allocate_buffer`/`write_buffer`/`read_buffer`/
  `create_compute_pipeline`/`dispatch`) over the same
  `meridian_gpu_driver::Device`, alongside its existing CPU
  `ComputeDevice` — two independent backends, a caller picks explicitly
  (see `compute-runtime::ComputeContext::with_gpu`).

**Windowed surface creation is split at the handshake boundary.**
`meridian_gpu_driver::Device::new_windowed` does only the adapter/device
request with `compatible_surface` set (the part that's genuinely
`wgpu`-mechanical and shareable) and hands back the raw `wgpu::Surface`
*and* the `wgpu::Adapter` it was chosen against — it does not own
`SurfaceConfiguration`/format selection/swapchain lifecycle/present,
which is `graphics-driver`'s job (`compute-driver` never touches this
constructor at all, only ever `Device::new`). The adapter is returned
specifically because `wgpu` ties a surface's format/present-mode
capabilities to the adapter, not the device, and `gpu-driver` has nowhere
else to expose it from once construction returns.

**`gpu-driver` exposes raw `wgpu` escape hatches deliberately.**
`Device::wgpu_device`/`wgpu_queue`, `Buffer::wgpu_buffer`,
`Shader::wgpu_shader`, `ComputePipeline::bind_group_layout`,
`CommandBuffer::encoder_mut`/`device` — these exist so `graphics-driver`
can build its own render-pass/surface-configuration logic on the same
underlying objects without `gpu-driver` having to anticipate every
graphics-specific operation itself. This is a narrower, more honest
contract than trying to make `gpu-driver`'s own API cover rendering too
(which would just re-create the original boundary problem one layer
down).

## Alternatives considered

- **`compute-driver` depends on `graphics-driver` directly** — rejected:
  crosses a `*-driver`-to-different-subsystem's-`*-driver` boundary for a
  capability (general compute dispatch) that was never really
  `graphics-driver`'s to own in the first place.
- **`compute-driver` reimplements its own `wgpu` device wrapper** —
  rejected: duplicates real, easy-to-get-subtly-wrong mechanics
  (the buffer readback pattern in particular) between two crates, exactly
  what CLAUDE.md's "don't drag another crate's logic into your own" rule
  exists to catch, once a second consumer (here: `compute-driver`) of the
  same underlying mechanics existed.
- **Put the shared mechanics in `platform-core`** — rejected: every
  `*-driver` crate depends on `platform-core` (`audio-driver`/
  `physics-driver` included), so a `wgpu` dependency there would drag
  `wgpu`'s heavy transitive tree into drivers that never touch a GPU.
  `platform-core`'s own scope ("OS abstraction: window, input,
  filesystem, time, threading, dynamic library loading") doesn't include
  GPU device management either — the same reasoning that already made
  `wgpu` a deliberate, scoped exception to zero-external-deps for
  `graphics-driver` specifically (see docs/roadmap.md's `wgpu` entry)
  argues against widening that exception to every driver.

## Consequences

- `graphics-driver` and `compute-driver` share one real implementation of
  `wgpu` device/buffer/shader/compute-pipeline mechanics, not two
  independently-maintained copies.
- `compute-runtime::ComputeContext` has a real GPU dispatch path now
  (`with_gpu`/`gpu()`), proven end-to-end by a compute-shader round-trip
  test reachable through `compute-runtime` itself, not just
  `compute-driver` or `gpu-driver` in isolation.
- `graphics-driver` no longer has its own compute pipeline — any future
  graphics-specific compute need (particle simulation feeding directly
  into a render pass, for instance) goes through `compute-driver`/
  `compute-runtime` like every other domain, per rule 5, rather than
  reappearing as a `graphics-driver`-local shortcut.
- Two independent `wgpu::Device` instances can exist in one process (one
  via `graphics-driver`, one via `compute-driver`) if an application
  constructs both — not a safety problem (each is an ordinary, independent
  safe Rust object; `wgpu::Device`/`Queue` are `Arc`-backed and cheap to
  clone on their own), but it does mean no zero-copy GPU buffer sharing
  between a compute dispatch and a render pass without an explicit
  readback/upload round-trip through the CPU. Should that cost ever
  matter for a concrete use case, the fix is a caller constructing one
  `meridian_gpu_driver::Device` and handing clones of it to both
  `graphics-driver`'s and `compute-driver`'s wrapper types (both accept
  one via their existing constructors' inputs conceptually, though
  neither has a `from_shared` constructor yet — tracked as follow-up, not
  built speculatively ahead of a concrete need).
