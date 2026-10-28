# ADR 048: animation in the runtime

## Status

Accepted. Builds on [ADR 047](047-skeletal-animation-vocabulary.md) (the
vocabulary, skinning and morphing) and [ADR 025](025-wave-6-runtime-and-composition-root.md)
(the runtime owns no state).

## Context

ADR 047 left animation to whoever holds a rig: the viewer advanced a player
by hand and copied joint matrices into renderables. A game wants many
animated entities, each at its own point in its own animation, driven by the
same frame loop as physics and audio, without the application code touching
matrices.

## Decisions

**D1 — the rig and the player move into `animation-domain`.** `Rig` (the
skeletons, the animations and the rest morph weights), `Animation`,
`AnimationLayer` and `AnimationPlayer` are plain data and math with no asset
or renderer types. `sdk` keeps only the conversion from a decoded
`ModelAsset` (`ModelRig` is an alias of `Rig`). Joint matrices come out as
plain arrays; whoever draws wraps them (`JointPalette`).

**D2 — animation state is ECS components.** `Animator` (a shared `Arc<Rig>`
and an `AnimationPlayer`) goes on one entity per model instance;
`AnimationTarget` (the animator's entity, the skeleton that skins the piece,
the part whose morph weights it takes) goes on the entity of each drawn
piece. `animation-domain` takes an edge to `ecs` for the `Component`
trait, as `rendering-domain` does for `MeshRenderer`.

**D3 — the output is two rendering components.** The animation stage writes a
`JointPalette` and a `MorphWeights` onto each target; `extract_scene3d` reads
them into the renderable's `palette` and `morph_weights`. The animation
domain knows nothing of rendering, and the renderer nothing of animation: the
plain-data components in `rendering-domain` are the contract.

**D4 — one stage, one place in the schedule.** `AnimationStage` (in
`runtime-app`, with the shared `FrameDelta` that carries the frame's real
seconds) advances every animator and posts the results. It changes no
`Transform` (animation of nodes goes through a node skeleton, ADR 047), so it
has no ordering against physics or `TransformStage`. It must run after any
stage that starts or switches animations (gameplay) and before the scene is
extracted for rendering; the composition root registers it with those as its
dependencies. It runs once per display frame (`tick_only`), not per physics
step, since animation follows real time, not the fixed step.

**D5 — no registry of rigs.** A rig is shared by `Arc` and lives as long as
anyone holds it; the runtime keeps no table of them and no global animation
manager.

## Consequences

- `runtime-app` takes edges to `animation-domain` and `rendering-domain`;
  `animation-domain` moves to tier 4.
- Entities that share a rig play independently; a crossfade is a call on the
  entity's player.
- State machines (choosing the next animation from game state) and root
  motion are not here; a gameplay stage that calls `play`/`crossfade_to` on
  the animator is the extension point.
- Spawning a loaded model into a world (one animator entity, one target per
  piece) is the composition root's job for now.
