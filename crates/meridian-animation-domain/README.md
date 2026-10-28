# meridian-animation-domain

Skeletal animation, CPU only ([ADR 047](../../docs/adr/047-skeletal-animation-vocabulary.md)).
A `Skeleton` is a tree of joints with rest transforms and inverse bind
matrices. A `Clip` is a set of keyframed translation, rotation and scale
channels, each aimed at one `JointId`, in glTF's three interpolation modes.
Sampling a clip into a `Pose` and asking the skeleton for its joint matrices
gives what a skinned mesh is drawn with. No GPU, no I/O, no scene types: the
renderer takes the matrices as plain data.
