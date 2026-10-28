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
| geometry | shared kernel | `numeric` (kernel), `gac` (kernel, perf), `gac-wgpu` (infra) | scalars, `Fixed`, vectors, bivectors, rotors, motors, shapes |
| identity | shared kernel | `memory` (kernel, perf), `resource` (kernel) | generational handles, pools, arenas, resource ids, versions, dependency graph |
| entity | shared kernel | `ecs` (kernel, perf) | entities, components, archetypes, queries, `Transform` |
| diagnostics | shared kernel | `diagnostics` (kernel: `EngineError`, the log kernel and `LogSink` port), `diagnostics-fs` (infra: log file and crash report, implements `LogSink`) | `EngineError`, log levels, the process log sink |
| scene-markup | shared kernel / published language | `markup` (kernel), `markup-macros` (interface) | markup elements, tags, the tag registry |
| platform | infrastructure | `platform-winit` (infra) | window, input state, clock, CPU/GPU capabilities, dynamic libraries |
| compute | application + adapters | `compute-app` (app), `compute-wgpu` (infra), `gpu-wgpu` (infra) | compute context, kernels, dispatch size, untyped buffers, GPU device |
| assets | domain | `asset-domain` (domain: types, `Decoder` port, pure parsers), `asset-png`, `asset-jpeg`, `asset-gltf`, `asset-symphonia` (infra: codec adapters) | decoded images, meshes, audio, shader sources, decoders |
| rendering | domain + adapter | `rendering-domain` (domain), `rendering-app` (app: ports `FrameRecorder`/`MeshStore`/`TextureStore`), `rendering-wgpu` (infra: wgpu recording, implements the ports) | scenes, cameras, lights, materials, render graph, frame submission |
| physics | domain + adapters | `physics-domain` (domain, perf), `physics-app` (app: `PhysicsSubsystem`, the `PhysicsStepper` port), `physics-wgpu` (infra: batched stepper) | rigid bodies, colliders, contacts, solver, integrator, soft bodies |
| audio | domain + adapter | `audio-domain` (domain), `audio-app` (app: `AudioSubsystem`, the `AudioSink` port), `audio-cpal` (infra: cpal, implements it) | listener, emitters, speakers, mixer, DSP graph |
| animation | domain | `animation-domain` (domain: skeletons, joints, keyframed clips, playback, joint matrices) | skeleton, joint, pose, clip, channel, playback |
| text | domain | `text-domain` (domain: TrueType parsing, outlines, rasterizer, glyph atlas, shaper) | faces, outlines, coverage bitmaps, glyph atlas |
| ui | domain + adapter | `ui-domain` (domain), `ui-mel` (infra: build-time `.meui` generator), `ui-scene2d` (infra: adapter from the UI tree to `rendering-domain::Scene2d`) | element tree, typed properties, layout, hit testing, generated bindings |
| data | infrastructure | `scene` (infra: Substrate Data scene documents) | scene document schema and migration |
| editor | infrastructure | `editor-domain`, `editor-app`, `editor-tcp` (infra) | host/editor wire messages, versions, ids; the host's session, ports and loopback TCP adapter |
| runtime | application + interface | `runtime-app` (app: stages, `JobExecutor` port, events; owns no state), `runtime-threads` (infra: implements `JobExecutor`), `sdk` (interface: composition root), `examples` (interface) | frame stages, job executor, events, job graph |

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
| geometry → compute | customer–supplier | `gac-wgpu` kernels implement compute's port |
| rendering → platform | customer–supplier | window handle for the surface |
| compute → platform | customer–supplier | GPU/CPU capability report |
| audio → platform | customer–supplier | capability report |
| physics → platform | customer–supplier | `CpuCapabilities` (`platform-winit`) |
| runtime → physics, audio, rendering, animation, platform, assets, compute, diagnostics | customer–supplier | each context's published API, called by runtime use cases |
| ui → text | customer–supplier | `ui-scene2d` measures and draws text through `text-domain` faces and the glyph atlas (ADR 023) |
| ui → rendering | customer–supplier | `ui-scene2d` extracts the UI tree into `rendering-domain::Scene2d` (ADR 021) |
| editor → data | customer–supplier | the protocol carries scene documents and entity refs |
| scene-markup → runtime | anticorruption layer | markup tags translated into runtime commands |

## Known gaps

`craftcheck deps` lists them; `.craftcheck-deps-baseline` holds the count and
may only shrink.

- `sdk → markup-macros` (runtime → scene-markup interface crate) has no edge;
  resolved when the DSL tags leave `sdk` (ADR 020 D3, wave 6).
- I/O in the diagnostics kernel (`diagnostics`'s tokio file sink, crash
  report): ADR 020 D7, wave 6. (`audio-domain::AudioOutput::open` left the
  domain in wave 5.)
