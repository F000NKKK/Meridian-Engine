# Glossary

The ubiquitous language of Meridian-Engine, one section per bounded context
(Craftbook ch. 7 §7.1; contexts in [context-map.md](context-map.md)). The
code uses these words and these words name the code: renaming one renames
the other, in the same change.

## Geometry

| Term | Meaning | In code |
|---|---|---|
| Scalar | The float flavour's number type, `f32`. | `numeric_core::Scalar` |
| Fixed | Q16.16 fixed-point number; bit-exact on every machine (ADR 008). | `numeric_core::Fixed` |
| Flavour | A choice of number type for the whole algebra: float or fixed. | `GaFlavor`, `FloatFlavor`, `FixedFlavor` |
| Vector | A grade-1 element: a direction or a point offset. | `Vec3`, `FixedVec3` |
| Bivector | A grade-2 element: an oriented plane; angular velocity and torque. | `Bivector3` |
| Multivector | A general element of the algebra. | `Multivector` |
| Rotor | An element that rotates by sandwiching: `R v R̃`. | `Rotor` |
| Motor | A rotor combined with a translation: a rigid transform. | `Motor3` |
| Frame | A motor read as a local coordinate system. | `Frame` |
| Shape | A bounded convex region used for culling and collision. | `Shape`, `Aabb`, `Sphere`, `Obb`, `Cone`, `Plane`, `ConvexVolume` |
| Batch kernel | A compute kernel that applies one GA operation to many values. | `MotorTransformKernel`, `MotorComposeKernel` |

## Identity

| Term | Meaning | In code |
|---|---|---|
| Handle | Index plus generation; stale once its slot is freed and reused. | `Handle` |
| Pool | Storage that hands out handles and resolves them. | `ResourcePool` |
| Arena | Bump allocation freed all at once, per frame or for good. | `FrameArena`, `PersistentArena` |
| Resource id | A typed handle naming a resource of one kind. | `ResourceId` |
| Version | A counter bumped when a resource changes. | `Version` |
| Resource dependency | "A needs B" between resources; cycles are rejected. | `ResourceDependency`, `DependencyGraph` |

## Entity

| Term | Meaning | In code |
|---|---|---|
| Entity | An id with no data of its own. | `Entity` |
| Component | Plain data attached to an entity. | `Component` |
| Archetype | The set of entities with exactly the same component types; stored column by column. | `Archetype` |
| World | Owns every entity and archetype. | `World` |
| Query | Iteration over every entity that has given components. | `Query`, `QueryMut` |
| Transform | An entity's motor in world space; the one spatial component ECS knows. | `Transform` |

## Diagnostics

| Term | Meaning | In code |
|---|---|---|
| Engine error | The shared error convention every crate's error converts into. | `EngineError` |
| Log level | Severity of a log line. | `LogLevel` |
| Log sink | The process-wide destination of log lines; the one allowed global. | `logging::install` |
| Crash report | A file written when the process panics. | `CrashReportConfig` |

## Scene markup

| Term | Meaning | In code |
|---|---|---|
| Element | A parsed markup node: tag, attributes, children. | `Element` |
| Tag | A markup element kind registered with a builder. | `DslTag` |
| Tag registry | Maps tag names to builders; built once, passed in. | `DslRegistry` |
| Built node | What a tag builder produced from an element. | `BuiltNode` |

## Platform

| Term | Meaning | In code |
|---|---|---|
| Window | An OS window with a surface handle. | `Window` |
| Input state | Keys and mouse buttons held this frame. | `InputState`, `KeyCode`, `MouseButton` |
| Clock | Source of frame time. | `Clock`, `Time` |
| Capabilities | What the CPU or GPU of this machine offers. | `CpuCapabilities`, `GpuCapabilities`, `BackendCapabilities` |
| Dynamic library | A shared library loaded at run time. | `DynamicLibrary` |

## Compute

| Term | Meaning | In code |
|---|---|---|
| Compute context | The CPU and, if present, GPU a kernel is dispatched on. | `ComputeContext` |
| Kernel | One batched algorithm with a CPU and optionally a GPU path. | `ComputeKernel` |
| Hybrid kernel | A kernel whose batch is split between CPU and GPU. | `HybridKernel`, `BackendSplit` |
| Dispatch size | How many items one dispatch covers. | `DispatchSize` |
| Compute buffer | Untyped bytes on a device; meaning belongs to the adapter. | `ComputeBuffer` |
| GPU device | The `wgpu` device and queue. | `gpu_driver::Device`, `GpuComputeDevice` |

## Assets

| Term | Meaning | In code |
|---|---|---|
| Decoder | Turns bytes already in memory into a CPU-side value. | `Decoder` |
| ImageAsset | Decoded RGBA8 pixels with width and height. | `ImageAsset` |
| Mesh (asset) | Decoded vertex positions, normals, UVs and indices. | `MeshAsset` |
| Audio clip | Decoded PCM samples with rate and channel count. | `AudioClip` |
| Streaming decode | Decoding audio packet by packet instead of all at once. | `StreamingAudioDecoder` |
| Shader source | WGSL text loaded as an asset. | `ShaderSource` |

## Rendering

| Term | Meaning | In code |
|---|---|---|
| Scene | What one frame draws: renderables, lights, camera. | `Scene3d`, `Scene2d` |
| Renderable | A mesh drawn with a material at a motor. | `Renderable3d` |
| Camera | Projection and view motor. | `Camera`, `Camera2d` |
| Light | A directional, point or spot light. | `Light` |
| Material | How a surface is shaded. | `Material`, `MaterialRegistry` |
| Mesh registry | GPU meshes by handle. | `MeshRegistry`, `MeshHandle` |
| Render graph | Passes ordered by the resources they read and write. | `RenderGraph`, `RenderPass` |
| Bloom | Post-processing that blurs bright areas back over the frame. | `BloomPass`, `BloomConfig` |
| Frustum culling | Dropping renderables outside the camera's view. | `Frustum`, `cull_scene3d` |
| Surface | The swapchain a window presents frames from. | `Surface`, `SurfaceFrame` |

## Physics

| Term | Meaning | In code |
|---|---|---|
| Rigid body | A body that does not deform: mass, frame, velocity, angular velocity. | `RigidBody` |
| Collider shape | The shape a body collides with. | `ColliderShape` |
| Broad phase | Finds pairs whose bounds overlap. | `BroadPhase` |
| Narrow phase | Turns a candidate pair into contacts. | `NarrowPhase` |
| Contact | Point, normal and depth where two bodies touch. | `Contact` |
| Constraint solver | Resolves contacts into velocity changes. | `ConstraintSolver` |
| Integrator | Advances bodies by one step. | `Integrator` |
| Soft body | Particles joined by springs. | `SoftBody`, `Spring`, `SoftBodyIntegrator` |
| Step | One fixed-length advance of the simulation. | `PhysicsSubsystem::step` |

## Audio

| Term | Meaning | In code |
|---|---|---|
| Listener | The ear's frame in the world. | `Listener` |
| Emitter | A sound source's frame and gain. | `Emitter` |
| Speaker | An output channel's direction. | `Speaker`, `SpeakerLayout` |
| Attenuation | Gain falling with distance. | `AttenuationModel` |
| Mixer | Renders emitters into speaker channels. | `Mixer` |
| DSP graph | Effects applied to a signal, in order. | `DspGraph`, `DspNode` |
| Binaural rendering | Headphone spatialization by head-related delays and filters. | `BinauralRenderer` |
| Audio sink | The port through which mixed samples leave the audio domain; the platform adapter implements it. | `AudioSink`, `AudioStream` |

## Runtime

| Term | Meaning | In code |
|---|---|---|
| Stage | One step of a frame (physics, audio, render). | `Stage`, `StageId` |
| Frame scheduler | Orders stages by their dependencies. | `FrameScheduler` |
| Job graph | Jobs with dependencies, run by worker threads. | `JobGraph`, `JobId`, `Scheduler` |
| Event system | Typed publish/subscribe between stages. | `EventSystem` |
| Runtime | Drives stages each tick. | `Runtime` |
| Composition root | The one place that wires adapters into use cases. | `meridian-sdk`, examples |
