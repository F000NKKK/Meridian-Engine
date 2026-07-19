# Graphics design — `meridian-graphics-driver` + `meridian-graphics-core`

## The split

`graphics-driver` is the hardware abstraction — the engine's equivalent of a
thin Vulkan/DX12/Metal wrapper: devices, command queues, buffers, textures,
shaders, pipelines, synchronization. It has no concept of a scene, a
material, or a camera. In other engines this role is sometimes called an
RHI (Render Hardware Interface); `graphics-driver` *is* that layer under a
different name — not a separate crate to add later. Concrete backends
(`vulkan-driver`, `dx12-driver`, `metal-driver`) are expected to become
their own crates implementing `graphics-driver`'s abstraction, without
`graphics-core` ever changing — see
[ADR 005](adr/005-driver-core-separation.md). No backend crate exists yet;
adding one is out of scope until there's a concrete backend to build (see
[roadmap.md](roadmap.md)).

`graphics-core` is everything that *does* know about scenes:

```text
Render Graph        automatic pass ordering + resource dependency tracking
Scene Extraction     pulling renderable state out of the ECS each frame
Visibility / Culling
Lighting
Materials
Camera
Animation
Post Processing
```

## Render graph over hand-ordered passes

Instead of a hardcoded `Shadow → Geometry → Lighting → Postprocess`
sequence, passes declare their resource reads/writes and the graph derives
execution order and resource lifetime automatically. This is what allows
adding a pass without manually re-threading every pass after it.

## GPU-driven rendering

The long-term direction is minimizing CPU-side per-object work: the CPU
prepares data, the GPU (via indirect draw, compute-based culling, and
instancing) decides what actually gets drawn. This is why `graphics-core`
depends on `meridian-compute-core` — GPU culling and compute-based
visibility are compute workloads, not rendering-specific ones (see
[dependency-rules.md](dependency-rules.md) rule 5).

## What this crate does not own

Asset bytes and decoding belong to `asset-core`; `graphics-core` consumes
already-decoded CPU-side representations and turns them into GPU resources
via `graphics-driver`. It does not implement image/mesh decoders itself.

`TextureHandle`, `MeshHandle`, `BufferHandle` and `ShaderHandle` are typed
resource identities from `meridian-resource-core`, not ad hoc handles
defined in this crate — see
[ADR 006](adr/006-resource-core-separation.md).
