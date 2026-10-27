# `meridian-physics-wgpu` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `lib`

Both kernels mirror `SoftBodyIntegrator::step`'s *semantics* exactly
(same force-accumulation-then-integrate-then-collide structure, same
per-spring formulas — see each module's doc comment for the
particle-centric adjacency reformulation that makes this
race-condition-free without atomics) but only the `fixed` kernel
promises bit-exact reproducibility against the CPU path; `float`'s
GPU/CPU results agree numerically, not bit-for-bit (GPU float
summation order can differ from the CPU's, the same reason `Fixed`
exists at all — see `meridian_numeric::Fixed`'s doc comment).

Like [`meridian_gac_wgpu::fixed_wgsl`], buffers (topology,
positions, velocities) are rebuilt every [`soft_body::float::SoftBodyGpuKernel::step`]/
[`soft_body::fixed::FixedSoftBodyGpuKernel::step`] call rather than persisted
across steps — a real, deliberately-deferred gap (see
`meridian_compute_app`'s `HybridKernel` module doc for the same
tradeoff acknowledged there), not an oversight.

## `constraint_solver`

**The fix: graph coloring.** [`color_contacts`] partitions `contacts`
into groups such that no two contacts in the same group share a body
index — contacts in the same color are provably independent (their
writes touch fully disjoint bodies), so they can run concurrently
with zero risk of a race, *and* the result is identical to running
them one at a time in any order (since none of them can observe each
other's writes). Colors themselves still run in sequence, in the
same relative order the contacts were encountered in (greedy
coloring assigns a contact to the first color that doesn't already
touch one of its bodies, scanning colors in creation order) — this
reproduces the *exact* Gauss-Seidel dependency chain the sequential
loop has: whichever contact would have run first still effectively
runs first (in its own color), and any contact that depends on an
earlier one sharing a body is guaranteed to land in a later color
(they can never share a color, by construction), so it always sees
the update. [`ConstraintSolverBatchKernel::step`]'s own test proves
this equivalence directly against the sequential loop, not just
"produces plausible output."

**Per-body locking, not one shared lock over the whole body array.**
Bodies are `Vec<Mutex<RigidBody<F>>>` — each body individually
lockable. Within one color, no two contacts ever touch the same
body (that's the coloring invariant), so no thread ever contends
another thread's lock — real parallelism, not `parallel_for`
serialized behind one coarse `Mutex<Vec<_>>` the way
[`crate::rigid_body::RigidBodyIntegratorKernel`]'s independent
per-body writes still are (a deliberate "correct first, profile
before optimizing further" choice there — this module can do
better *safely*, at zero extra `unsafe`, only because coloring
already proves the disjointness a per-body lock would otherwise
exist to arbitrate).

## `soft_body::csr`

Each half-edge also encodes, in `neighbor`'s top bit, whether *this*
particle is the original [`Spring`](
meridian_physics_domain::soft_body::spring::Spring)'s `a` or
`b` endpoint (see [`decode_neighbor`]) — not a topology detail, a
numerical-correctness one. `Fixed::mul`'s `>>16` truncation rounds
toward negative infinity, so `fixed_mul(-p, q)` is *not* always
exactly `-fixed_mul(p, q)` (they differ by one raw bit whenever the
discarded low bits are nonzero, which is the common case) — meaning
recomputing a spring's direction independently from each endpoint's
own (sign-flipped) `delta` and multiplying is **not** bit-exact with
`SoftBodyIntegrator::step`'s "compute `total` once from `a`'s
direction, then `forces[b] -= total`" (this was found the hard way: an
earlier version of this module recomputed direction/`total` per
endpoint and passed every test except a 60-step bit-exact
reproducibility check, which diverged by one raw bit at step 48 —
`Vector::Sub`/`Neg`, unlike `Fixed::mul`, *are* exact under negation,
since they're plain two's-complement add/negate with no shift/
truncation involved). The fix: both endpoints compute the *same*
canonical `a`-to-`b` direction and `total` (via `is_a`-conditional
subtraction order, still exact), and only the non-`a` endpoint
negates the finished `total` vector — an exact negation, never a
multiply — reproducing `SoftBodyIntegrator::step` bit-for-bit. See
[`crate::soft_body::fixed`]'s kernel for where this canonicalization happens.
