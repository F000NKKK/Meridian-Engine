# ADR 047: skeletal animation vocabulary

## Status

Accepted. Gives [ADR 030](030-gltf-static-models-through-an-asset-adapter.md)'s
"skeleton and animation vocabulary" a home, and answers the open question of
[ADR 007](007-batch-transforms-via-compute.md) (an `animation-core`). The
vocabulary and its math (D1 to D4), loading skins from glTF (D5) and drawing
them (D6) have landed.

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

**D6 — drawing a skin.** A mesh is skinned when its `MeshSource` carries a
`VertexSkin` (four joints and weights per vertex, which the registry checks
against the vertex count); a `Renderable3d` of it is posed when it carries a
`JointPalette` (one matrix per joint, shared by `Arc`), and drawn rigid when it
carries none. The renderer poses on the GPU: a compute kernel in
`rendering-wgpu` reads the mesh's bind-pose vertices (its vertex buffer is
also a storage buffer), the joints and weights and the palette, and writes a
posed copy of the vertices into a fresh `VertexStorage` buffer for that
instance. A posed instance is a batch of its own (`BatchKey::own`), because
its vertices are its own; the batch carries the posed buffer, and the main,
shadow and bloom passes draw it with their ordinary pipelines, so no pipeline
gains a skinned variant. The dispatches are recorded in `render_shadow_pass`,
before any pass that draws the batches. A posed instance has no bounds, so it
is never culled (its box is not its bind pose's). The kernel blends the four
matrices by weight; normals are turned by the same blend and renormalised,
which is exact for rigid joints and uniform scale. The application poses the
model: `sdk`'s `ModelRig` holds the `Skeleton`s and `Clip`s of a loaded model
(`AssetCache::load_model_gltf` returns a `LoadedModel` of pieces and rig) and
`ModelRig::palettes(animation, time)` samples them, so a viewer advances a
`Playback` each frame and assigns each piece its skeleton's palette
(`gltf_viewer` plays `MERIDIAN_ANIMATION`, the first animation by default).

## Consequences

- One new crate and the edge `animation-domain -> gac` (documented in
  `docs/dependency-rules.md`, enforced by `./build.sh check-deps`).
- Models with skeletons can be animated by anything that holds a `Skeleton`, a
  `Clip` and a `Playback`, not only by the renderer.
- A skeleton over 256 joints is refused rather than truncated.
- Every posed instance costs one compute dispatch per frame. Its palette and
  posed-vertex buffers and bind group live in a slot, found by the instance's
  place among the frame's skinned instances and kept while the same mesh is
  drawn with a palette of the same size; slots the frame did not use are freed.
- A posed instance is not instanced, culled or batched with others.
- The sdk gains the edge `sdk -> animation-domain` (context `runtime ->
  animation`).
- Cubic-spline rotations are blended as raw quaternions and renormalised, as
  glTF specifies, not as arcs.
- Animation of nodes without a skin goes through the same path: the decoder
  gathers the mesh nodes an animation channel moves, directly or through an
  ancestor, with their ancestors into one more skeleton (identity inverse
  bind, rest pose from the node's transform), and binds each such part to its
  node's joint with weight 1, vertices left in node space. Scale animation
  works, and the renderer needs nothing new. A model needing more than 256
  such nodes keeps them static.
- Morph targets are plain data in asset-domain (`PartMorph`: per-vertex offset
  targets and the starting weights; `ModelWeightsChannel`: weight keys of one
  part, in `ModelClip::weights`). The glTF decoder keeps offsets as authored
  for skinned parts and carries them through the node's linear transform for
  rigid ones; a node's weights override its mesh's. A weights channel moves
  no node, so it does not make the node a joint. At most 64 targets per
  primitive. Playing them is left to the animation and rendering crates.
- A mesh with morph targets (`MeshSource::morph`) is posed by the same
  compute kernel as a skinned one, by a renderable's `morph_weights`: the
  weighted offsets are added to the base position and normal, then the joint
  palette (if any) is applied. Offsets sit in one storage buffer and the
  kernel reads its target count, vertex count, skin flag and weights from a
  small control block. A renderable whose weights are all zero draws the mesh
  rigid. Meshes with morph targets, like posed ones, are never culled or
  batched with other instances.
