# Roadmap

## Current state

Every crate in the workspace is a scaffold: correct name, correct
dependency edges, a one-line doc comment. No implementation yet. This is
intentional — see "Why implementation is deliberately last" below.

## Why implementation is deliberately last

The riskiest failure mode at this project's current size isn't technical
complexity — it's layers slowly bleeding into each other (a rendering
concept leaking into `ecs-core`, a global manager reappearing in
`asset-core`, a driver crate quietly depending on its own core). Once real
code exists, every such violation costs a refactor. Right now it costs a
one-line correction to [dependency-rules.md](dependency-rules.md). The
priority before writing implementations is keeping that document and the
[ADRs](adr/) accurate and complete.

## Suggested implementation order

1. `meridian-foundation`, `meridian-numeric-core` — scalar/SIMD groundwork
   everything else depends on.
2. `meridian-gac-core` — motors, rotors, transforms. Nothing above this
   layer can be meaningfully tested without it.
3. `meridian-memory-core`, `meridian-task-core`, `meridian-platform-core` —
   in parallel, no interdependency.
4. `meridian-ecs-core` — archetype storage, `Transform` as a component.
5. `meridian-compute-driver` → `meridian-compute-core` — needed before
   physics or graphics can use compute.
6. `meridian-asset-core` — decoders, independent of the above once
   `platform-core` exists.
7. `meridian-graphics-driver` → `meridian-graphics-core`, and
   `meridian-physics-driver` → `meridian-physics-core`, and
   `meridian-audio-driver` → `meridian-audio-core` — in parallel across
   subsystems once their shared dependencies (steps 1-6) exist.
8. `meridian-engine-core` — wires everything into the main loop last, once
   there's something real to schedule.

## Explicitly out of scope for now

`animation-core`, `particles-core`, `ai-core` — referenced in
[dependency-rules.md](dependency-rules.md) as future consumers of
`compute-core`, but not part of the current workspace. Add them only when
there's a concrete subsystem to build, not speculatively.

## Not yet decided

- Deterministic simulation mode (fixed-point vs. ordered floating point) —
  needed for physics replay/networking, tracked but unspecified.
- Concrete graphics backend(s) beneath `graphics-driver` (Vulkan first,
  per [ADR 005](adr/005-driver-core-separation.md), but not started).
