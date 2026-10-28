# ADR 047: skeletal animation vocabulary

## Status

Accepted. Gives [ADR 030](030-gltf-static-models-through-an-asset-adapter.md)'s
"skeleton and animation vocabulary" a home, and answers the open question of
[ADR 007](007-batch-transforms-via-compute.md) (an `animation-core`). The
vocabulary and its math (D1 to D4) and loading skins from glTF (D5) have landed;
drawing them (D6) follows in its own change and amends this ADR.

## Context

The renderer draws rigid meshes. glTF, the format models ship in (ADR 030),
describes characters as a mesh bound to a tree of joints (a *skin*) and
keyframed clips that move the joints. The engine has no word for a joint, a
clip or a pose, so a skinned model loads as its rest pose at best.

## Decisions

**D1 — a new domain crate, `meridian-animation-domain`.** Skeletons, clips and
their math are CPU, no I/O, and not rendering: they sit beside `rendering-domain`
in the graph (layer `domain`, context `animation`), on `gac` only. The
renderer does not depend on it; it takes joint matrices as plain data (D6).
Animation is visual: it uses `f32` and is not part of the deterministic
simulation (ADR 008).

**D2 — the vocabulary follows glTF, with typed identity.** A `Skeleton` is a
list of `Joint`s, parents before children, and a `root` matrix that the root
joints hang from (the transform of everything above the skin in the model). A
`Joint` has an optional parent, a rest `JointTransform` (translation, rotation
as a `gac::Rotor`, per-axis scale) and an inverse bind matrix. A joint is named
by `JointId`, its position in its skeleton, never by a string: glTF node names
are labels for editors, not identity. A skeleton is at most `MAX_JOINTS` (256)
joints, which is what the skinned draw's palette and vertex indices are sized
for; one more is `SkeletonError::TooManyJoints`. A parent that does not come
first is `SkeletonError::ParentNotBefore`, so a tree can neither be cyclic nor
depend on evaluation order.

**D3 — a clip is channels of keyframes.** A `Channel` aims one `Track` at one
`JointId`. A track animates one property, `Translation`, `Rotation` (a unit
quaternion, `[x, y, z, w]`) or `Scale`, from `Keys`: strictly increasing times
and values, in one of glTF's three `Interpolation` modes. `Step` holds,
`Linear` blends (rotations by `Rotor::slerp`, along the shorter arc), and
`CubicSpline` is a Hermite spline whose keys carry an in-tangent, a value and an
out-tangent. Before the first key a track holds the first value, after the last
the last one. A `Clip` is as long as its latest key. `Keys::new` refuses what
glTF forbids (no keys, a value count that does not fit the mode, times that are
not finite and increasing) with a `ClipError`, so sampling never indexes out of
range.

**D4 — sampling, poses and joint matrices.** `Clip::sample(time, &mut Pose)`
overwrites the properties its channels animate in a `Pose` (one local
transform per joint, starting at `Pose::rest`) and leaves the rest.
`Skeleton::joint_matrices(&Pose)` returns, per joint, its model-space matrix
times its inverse bind matrix: what takes a bind-pose vertex to where the pose
puts it. `Playback` (time, speed, `Repeat::Once` or `Loop`) advances along a
clip without owning it, so one clip drives many playbacks. `gac::Rotor` gains
`from_quaternion`, `to_quaternion` and `slerp`, since quaternions are how glTF
stores rotations and a rotor is the same thing.

**D5 — what a glTF skin brings.** `asset-domain` carries it as plain decoded
data, typed by position, with no dependency on `animation-domain` (the
application converts, as it does for images): `ModelAsset::skeletons` (one
`ModelSkeleton` per glTF skin, joints parents first with rest translation,
rotation, scale and inverse bind matrix, plus the `root` transform above the
skin), `ModelAsset::clips` (a `ModelClip` per glTF animation that moves a
joint, each `ModelChannel` aimed at a `(skeleton, joint)` pair with its
`ModelProperty`, `ModelInterpolation`, times and flat values) and
`ModelPart::skin` (`PartSkin`: skeleton index and four joints and weights per
vertex, as `JOINTS_0`/`WEIGHTS_0`). `asset-gltf` reorders each skin's joints
parents first (glTF lists them in any order) and remaps `JOINTS_0` and
animation targets to that order; weights are renormalised, and a vertex with
none follows its first joint. glTF says the transform of a skinned mesh's node
is ignored, so a skinned part keeps its vertices as authored instead of taking
the node's world matrix. A `JOINTS_0` index past the skin's joints is
`GltfError::OutOfBounds`. Only joints are animated: animation of other nodes,
morph targets and several skins per part are not read, and a skin whose
roots hang from different parents uses the first root's parent as `root`.
Sample files (CesiumMan, Fox, BrainStem, RiggedSimple) decode with their
skeletons and clips.

**D6 — drawing a skin (change after that).** A skinned `Renderable3d` carries
its joint matrices. The renderer skins on the GPU in a compute pass that writes
a skinned vertex buffer per skinned instance, which the existing main, shadow
and bloom passes then draw unchanged, so no pipeline gains a skinned variant.

## Consequences

- One new crate and the edge `animation-domain -> gac` (documented in
  `docs/dependency-rules.md`, enforced by `./build.sh check-deps`).
- Models with skeletons can be animated by anything that holds a `Skeleton`, a
  `Clip` and a `Playback`, not only by the renderer.
- A skeleton over 256 joints is refused rather than truncated.
- Cubic-spline rotations are blended as raw quaternions and renormalised, as
  glTF specifies, not as arcs.
