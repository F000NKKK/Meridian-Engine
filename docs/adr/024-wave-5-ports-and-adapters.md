# ADR 024: Wave 5 — ports in app crates, adapters implement them

## Status

Accepted. Applies [ADR 019](019-ports-in-domain-drivers-as-adapters.md);
amends [ADR 013](013-compressed-audio-codecs.md) and
[ADR 014](014-streaming-audio-decode.md) on where the codecs live.

## Context

ADR 019 says dependencies point inwards: a layer declares the port it
needs, an adapter crate implements it. Wave 5 of the Craftbook migration
carries that out for every driver. Four questions were not settled by
ADR 019 and are decided here.

## Decisions

**D1 — I/O ports live in an `-app` crate created now, with its final name.**
Craftbook 7 puts I/O ports in the app layer, not the domain. Audio and
rendering had no app crate, so `meridian-audio-app` (`AudioSink`) and
`meridian-rendering-app` (`FrameRecorder`, `TextureStore`, `MeshStore`) are
new. They take their wave 7 names already so they are not renamed twice.
Compute already had an app crate, `meridian-compute-runtime`, which now
holds `ComputeDevice` and `GpuBackend`. Physics had no driver use at all:
`meridian-physics-driver` is deleted.

**D2 — the app layer stays technology-free, so the GPU handle is typed by
downcast.** `ComputeContext` holds `Arc<dyn ComputeDevice>` and an optional
`Arc<dyn GpuBackend>`. `GpuBackend` has only `device_name` and `as_any`;
the adapters that really need a wgpu device (`gac-compute`,
`physics-compute`) call `context.gpu::<GpuComputeDevice>()`. The runtime
crate would otherwise need `wgpu` or `platform-core`, which point outwards.
Device capabilities cross the port as the plain `ComputeSupport` value.

**D3 — ports are synchronous when their cost is bounded.** `AudioSink`
pushes and `TextureStore`/`MeshStore` uploads record work locally and
return at once. Opening a device or a surface is real I/O and stays an
`async fn` in the adapter (`open_output`, `gpu_context`), outside the port.

**D4 — every port has a fake and a contract suite (Craftbook 8.4).** Both
sit in the port's crate under `testing` (feature `testing`): a fake adapter
and `run_*_contract` functions. An adapter runs the same contract against
itself; adapters that need hardware do so behind `hw-tests`. The
`FrameRecorder` contract cannot run on the windowed wgpu recorder in CI, so
only its fake runs it; the real texture store runs its contract under
`hw-tests`.

**D5 — codecs are adapters; pure parsers stay in the domain.**
`asset-core` keeps what needs no library and no I/O: the asset types, the
`Decoder` port, `DecodeError`, format detection by magic bytes and the
hand-rolled BMP, WAV, OBJ and shader-source parsers. PNG moves to
`meridian-asset-png` and MP3, OGG/Vorbis, FLAC, OGG/Opus plus streaming to
`meridian-asset-symphonia`. The sniffing front door for images
(`AnyImageDecoder`) lives in the composition root (`sdk`), because choosing
which adapters are wired is its job; `AnyAudioDecoder` stays beside the
codecs it dispatches to. The process-wide `OnceLock` codec registry is gone:
each decoder builds its registry value.

**D6 — `graphics-core` no longer depends on `asset-core`.** The edge was
unused, and it was a domain to domain edge that is not customer-supplier
(L603). It is deleted rather than justified.

## Consequences

The sdk (composition root) names the adapters; the domain crates name none.
Pending outward edges shrink to the three of `engine-core`, removed in
wave 6. Public paths changed: see the wave 5 report for the old to new
mapping.
