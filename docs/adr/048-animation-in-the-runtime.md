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

**D6 — a state machine as plain data plus one component.** An
`AnimationGraph` (states that each play an animation, transitions with
conditions and a crossfade time, typed parameters) is shared by `Arc`; an
`AnimationController` component holds the current state and the parameter
values. Gameplay sets parameters (`set_float`, `set_bool`, `trigger`); the
stage calls `update` on the controller before the player advances, so at
most one transition fires per frame and a new state starts that frame.
States, parameters and conditions are ids handed out by the graph, never
names, and a transition naming something foreign or testing a parameter as
the wrong kind is refused when it is added. A trigger lives for one update.

**D7 — root motion takes the horizontal travel of one joint.** A
`RootMotion { skeleton, joint }` component on the animator makes the stage
pose the model with that joint's x and z held at rest
(`Rig::matrices_pinned`) and add the travel to the entity's `Transform`,
in the entity's own frame. The travel is the difference between the joint's
position now and before, plus a whole stride for each loop, mixed by the same
weights as the pose, so crossfades blend the walks. Rotation and height stay
in the pose. This is the one case where the stage writes a `Transform`: it
runs before the transform stage, and physics must not also drive the entity.

## Consequences

- `runtime-app` takes edges to `animation-domain` and `rendering-domain`;
  `animation-domain` moves to tier 4.
- Entities that share a rig play independently; a crossfade is a call on the
  entity's player.
- `runtime-app` also takes an edge to `gac` (a root-motion displacement is
  a `Vec3`, a transform a `Motor3`).
- Yaw root motion, blend trees and layered state machines are not here.
- `GraphicsSession::spawn_model` puts a loaded model into a world: a root
  entity (its `Transform` places the model, it holds the `Animator`) and one
  child per piece with a `MeshRenderer` and an `AnimationTarget`. Pieces
  follow the root through `Parent`, so `extract_scene3d` draws an entity at
  its `GlobalTransform` when it has one (the transform stage resolves it)
  and at its `Transform` otherwise. `LoadedModel::rig` is an `Arc`, shared by
  every instance.
