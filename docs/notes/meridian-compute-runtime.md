# `meridian-compute-runtime` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `lib`

This crate owns the ports `ComputeDevice` (CPU dispatch) and `GpuBackend`
(a GPU, as far as the application layer needs to know it); `compute-driver`
implements them (ADR 019). [`ComputeContext::with_gpu`] takes an already
acquired `Arc<dyn GpuBackend>`, e.g. `compute-driver`'s `GpuComputeDevice`
(built on `wgpu` via `meridian_gpu_driver`); acquiring it is the adapter's
genuine I/O and its `async fn` (see [ADR 009](../adr/009-async-io-via-tokio.md)).
[`ComputeContext::gpu`]`::<T>()` is what a [`ComputeKernel`] adapter reaches
for when it wants to dispatch on the GPU specifically: it downcasts to the
adapter type it knows. This crate doesn't pick a backend on a kernel's
behalf; the composition root builds the devices and the `ComputeContext`.

[`HybridKernel`]/[`ComputeContext::run_hybrid`] are the actual "split
one dispatch across both backends at once" mechanism — plain
[`ComputeKernel`]/[`ComputeContext::parallel_for`] can only ever run
on the CPU (an arbitrary `impl Fn(usize)` closure has no WGSL
equivalent a compiler can derive for you, so there is no such thing as
an automatic "run this same code on either backend" switch — a kernel
author has to write both implementations by hand, the same way
`gac-compute::fixed_wgsl` hand-writes its WGSL alongside
`meridian_numeric_core::Fixed`'s existing Rust). A kernel that
*does* provide both implements [`HybridKernel`]
(`run_cpu`/`run_gpu`, one method per backend, each covering an index
range rather than the whole dispatch), and [`ComputeContext::run_hybrid`]
splits `count` items between them per a [`BackendSplit`] policy
(`Ratio(0.5)` for an even split, among others) and runs both halves
*concurrently*, not sequentially: the CPU half runs on
`tokio::task::spawn_blocking`'s dedicated thread pool (needing
`K: Send + Sync + 'static`, hence the `Arc<K>` in the signature) while
this task directly awaits the GPU half's dispatch+readback — real
overlap, not "CPU work, then wait for GPU work."
