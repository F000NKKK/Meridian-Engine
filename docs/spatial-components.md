# Independent spatial components

Spatial authoring is ordinary ECS data, independent of scene files. Attach either:

- `Motor3`: one complete local rigid pose; or
- `Position(Vec3)` and/or `Rotor`: independent translation and rotation. Missing
  position means zero; missing rotor means identity.

`meridian-ecs` implements its Component trait for the shared GAC Rotor and Motor3
values; GAC itself stays ECS-independent. Position is a semantic wrapper so a
translation cannot be confused with an arbitrary vector component. Scale3, mesh
rendering and gameplay components remain separate and attach to the same entity.

`resolve_spatial_components(&mut world)` returns typed per-entity diagnostics and
writes local Transform only for valid source entities. Rotor inputs are normalized
through the checked GAC API; Motor3 inputs must already be unit rigid motions from
GAC constructors. Non-finite inputs are rejected. Motor3 plus Position/Rotors on
one entity is a competing-authority error. Its last valid Transform is preserved.
A Transform on a source entity is derived output: edit the source, not both.
Entities with only legacy Transform are untouched. Removing all sources leaves
the last derived pose in place; remove Transform explicitly to remove that pose.

For scheduled execution, register SpatialStage after source changes and before
TransformStage. Clone `stage.issues()` before moving the stage into Runtime to
observe diagnostics afterwards. Hierarchy propagation then derives GlobalTransform;
existing rendering extraction consumes it. `add_world_stages` remains unchanged:
spatial resolution is opt-in, and must not overwrite animation root motion or
physics-authored Transform on the same entity.

DATA registration uses the ordinary component registry:

```rust,ignore
registry.register::<Rotor>(ComponentName::new("rotor")?)?;
registry.register::<Motor3>(ComponentName::new("motor")?)?;
registry.register_with::<Position, PositionCodec>(ComponentName::new("position")?)?;
```

Independent authored values can be `{position:[1,2,3],rotor:[1,0,0,0]}` or
`{motor:{rotor:[1,0,0,0],translation:[1,2,3]}}`. Rotor storage is GAC scalar/bivector
order, **not glTF quaternion order**; prefer constructors or named schema codecs.
The registry/codec works for scene, prefab and component editing rather than a
scene-specific parser branch. IDLE must opt into registration and scheduling before
these types become available in its component picker.

## Collision and physics

`meridian-physics-app` owns independent `Collider(ColliderShape)` and `PhysicsBody`
components. A collider without a body is static. A body supplies mass and linear/
angular velocities; positive mass is dynamic, zero is static. Rendering is optional.

`step_ecs_physics(world, physics, stepper, dt)` resolves spatial sources, gathers
colliders into a **dedicated** PhysicsSubsystem, steps the real PhysicsStepper and
writes dynamic poses and velocities back to ECS. The dedicated body's vector is
rebuilt each step, ordered by entity identity. Do not mix manually inserted bodies
or a second PhysicsStepStage into that subsystem. Contact indices refer only to
that step; entity contact events are not wired yet.

Writing back also updates Motor3, or both Position and Rotor when split authoring
is present. A later spatial resolution therefore preserves simulated motion.
Legacy Transform-only bodies remain supported. Removing/despawning a collider
removes its physics body on the next step. Run hierarchy propagation after stepping.

Initial restrictions are explicit diagnostics: root entities only, unit scale,
finite pose/motion, finite nonnegative mass, positive finite collider dimensions.
No implicit scaling of collision shapes. Invalid dt rejects the entire step before
mutation. Supported shapes are the physics engine's sphere and cuboid; capsule and
mesh collision are not introduced here. IDLE component registration/Inspector and
DATA codecs for physics components remain follow-up work.

Regression tests simulate a sphere landing on a static cuboid over 240 steps,
verify removal and Motor3/split-pose write-back, and reject invalid configurations.
