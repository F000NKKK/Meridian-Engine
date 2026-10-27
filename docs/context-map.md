# Context map

The bounded contexts of Meridian-Engine and how they relate (Craftbook
ch. 7 §7.2). Machine-readable form: `craftbook.toml` (layers per crate,
`[[context_edge]]` per relation), checked by `craftcheck deps`. Words of each
context: [glossary.md](glossary.md). Decisions:
[ADR 019](adr/019-ports-in-domain-drivers-as-adapters.md) (ports and
adapters) and [ADR 020](adr/020-craftbook-adoption.md).

The table maps today's crates to their intended layer. A crate that holds
two layers today is listed under the one it is moving to; Craftbook's
`adoption/meridian-engine.md` §3 gives the target crate names.

## Contexts

| Context | Kind | Crates today (layer) | Owns |
|---|---|---|---|
| geometry | shared kernel | `numeric-core` (kernel), `gac-core` (kernel, perf), `gac-compute` (infra) | scalars, `Fixed`, vectors, bivectors, rotors, motors, shapes |
| identity | shared kernel | `memory-core` (kernel, perf), `resource-core` (kernel) | generational handles, pools, arenas, resource ids, versions, dependency graph |
| entity | shared kernel | `ecs-core` (kernel, perf) | entities, components, archetypes, queries, `Transform` |
| diagnostics | shared kernel | `foundation` (kernel; file sink and crash report move to an adapter, ADR 020 D7) | `EngineError`, log levels, the process log sink |
| scene-markup | shared kernel / published language | `dsl-core` (kernel), `dsl-macros` (interface) | markup elements, tags, the tag registry |
| platform | infrastructure | `platform-core` (infra) | window, input state, clock, CPU/GPU capabilities, dynamic libraries |
| compute | application + adapters | `compute-runtime` (app), `compute-driver` (infra), `gpu-driver` (infra) | compute context, kernels, dispatch size, untyped buffers, GPU device |
| assets | domain | `asset-core` (domain; codecs move to adapters in wave 5) | decoded images, meshes, audio, shader sources, decoders |
| rendering | domain + adapter | `graphics-core` (domain: scene, ports `FrameRecorder`/`MeshStore`/`TextureStore`), `graphics-driver` (infra: wgpu recording, implements the ports) | scenes, cameras, lights, materials, render graph, frame submission |
| physics | domain + adapters | `physics-core` (domain, perf), `physics-compute` (infra) | rigid bodies, colliders, contacts, solver, integrator, soft bodies |
| audio | domain + adapter | `audio-core` (domain), `audio-driver` (infra) | listener, emitters, speakers, mixer, DSP graph |
| ui | domain + adapter | `ui-core` (domain), `mel-build` (infra: build-time `.mel` generator), `ui-render` (infra: adapter from the UI tree to `graphics-core::Scene2d`) | element tree, typed properties, layout, hit testing, generated bindings |
| data | infrastructure | `scene` (infra: Substrate Data scene documents) | scene document schema and migration |
| editor | infrastructure | `editor-protocol`, `editor-host`, `editor-tcp` (infra) | host/editor wire messages, versions, ids; the host's session, ports and loopback TCP adapter |
| runtime | application + interface | `engine-core` (app), `task-core` (infra), `sdk` (interface: composition root), `examples` (interface) | frame stages, scheduler, events, job graph |

## Relations

Every context may use the shared kernels (geometry, identity, entity,
diagnostics, scene-markup) without an edge. Other cross-context
dependencies need one of these:

| From → to | Relation | What crosses |
|---|---|---|
| physics → geometry | shared kernel | motors, shapes, `GaFlavor` |
| rendering → geometry | shared kernel | motors, frusta, bounds |
| audio → geometry | shared kernel | listener/emitter frames |
| rendering → assets | published language | decoded image, mesh and shader CPU types |
| audio → assets | published language | decoded PCM |
| physics → compute | customer–supplier | `ComputeKernel`, `ComputeContext` |
| rendering → compute | customer–supplier | `ComputeKernel`, `ComputeContext` |
| geometry → compute | customer–supplier | `gac-compute` kernels implement compute's port |
| rendering → platform | customer–supplier | window handle for the surface |
| compute → platform | customer–supplier | GPU/CPU capability report |
| audio → platform | customer–supplier | capability report |
| physics → platform | customer–supplier | `CpuCapabilities` (`platform-core`) |
| runtime → physics, audio, rendering, platform, assets, compute | customer–supplier | each context's published API, called by runtime use cases |
| ui → rendering | customer–supplier | `ui-render` extracts the UI tree into `graphics-core::Scene2d` (ADR 021) |
| editor → data | customer–supplier | the protocol carries scene documents and entity refs |
| scene-markup → runtime | anticorruption layer | markup tags translated into runtime commands |

## Known gaps

`craftcheck deps` lists them; `.craftcheck-deps-baseline` holds the count and
may only shrink.

- `graphics-core → asset-core` is a domain-to-domain edge declared as a
  published language; Craftbook requires customer–supplier between two domain
  crates. Resolved when the decoded CPU types move to a published-language
  crate in the assets split (wave 5).
- `sdk → dsl-macros` (runtime → scene-markup interface crate) has no edge;
  resolved when the DSL tags leave `sdk` (ADR 020 D3, wave 6).
- I/O in the diagnostics kernel (`foundation`'s tokio file sink, crash
  report): ADR 020 D7, wave 6. (`audio-core::AudioOutput::open` left the
  domain in wave 5.)
