# `meridian-physics-domain` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `lib`

GA is used where the physics actually calls for it, not decoratively:
angular velocity and torque are bivectors (they live in the Lie
algebra of rotations, so(3) — a bivector space, not a vector space),
and orientation is integrated via the bivector exponential map,
composed onto the body's motor — this is what keeps a spinning body's
orientation exactly on the unit-rotor manifold frame after frame, the
same reason `gac` uses motors for `Transform` at all instead of a
quaternion+vector pair (see
[ADR 001](../adr/001-geometric-algebra-as-spatial-model.md)).
Linear velocity stays a plain vector — GA doesn't say vectors are
wrong, only that angular quantities specifically are bivectors.

The whole engine — [`generic::RigidBody`], [`generic::ColliderShape`],
[`generic::Contact`], [`generic::BroadPhase`], [`generic::NarrowPhase`],
[`generic::ConstraintSolver`], [`generic::Integrator`] — is written
**once**, generic over `meridian_gac::generic::GaFlavor`: unlike
`gac::Motor3`/`Vec3` (concretely duplicated because
`meridian-gac-wgpu` dispatches them to a GPU with no real `i64`),
nothing in this engine has a GPU-dispatch constraint of its own, so
duplicating it per scalar flavor would just be maintenance risk for no
reason (see CLAUDE.md's "Float/Fixed branching" rule). [`float`] and
[`fixed`] are thin `FloatFlavor`/`FixedFlavor` aliases over the one
engine in [`generic`] — `float` is re-exported at the crate root (so
`meridian_physics_domain::RigidBody`/etc. resolve to the default `f32`
path unchanged), `fixed` is an opt-in, bit-reproducible alternative
for lockstep networking/replay — see that module's doc comment.
Nothing about `float`'s types changes when `fixed` exists; it's a
parallel API, not a mode switch.

[`soft_body`] is a separate domain, deformable bodies (mass-spring),
not glob-exported at this crate's root — see that module's own doc
comment for why it's a genuinely different problem from the rigid-body
engine above, and its own float/fixed split.
