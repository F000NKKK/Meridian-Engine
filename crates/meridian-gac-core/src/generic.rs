//! Generic geometric-algebra engine — GaFlavor and everything built
//! on it (ScalarLike/VectorLike/BivectorLike/RotorLike/MotorLike,
//! Frame/Shape/Aabb/Sphere/Obb/Cone/Plane/ConvexVolume/Projection).
//!
//! Deliberately its own module, not defined directly at the crate
//! root next to `pub use float_ga::*;`: a locally-defined item
//! always shadows a glob-imported same-named item in Rust, so
//! putting `Aabb<F>` here-at-root would have silently shadowed
//! `float_ga::Aabb`'s re-export and broken every existing
//! `meridian_gac_core::Aabb` call site (no generic parameter
//! supplied) across the workspace. Keeping the generic engine in
//! its own namespace means `float_ga`/`fixed_ga`'s thin aliases
//! are the *only* things that reach the crate root under the
//! familiar unprefixed names — see `crate::float_ga`/`crate::fixed_ga`.

use core::ops::{Add, Div, Mul, Neg, Sub};

/// A scalar type usable throughout the generic (non-GPU-dispatch-bound)
/// half of this crate — implemented for [`meridian_numeric_core::Scalar`]
/// (`f32`) and [`meridian_numeric_core::Fixed`]. Every method is a thin forward to that type's own
/// inherent method (Rust resolves an inherent method over a
/// same-named trait method, so `self.sqrt()` inside `impl ScalarLike for
/// Scalar` calls `f32::sqrt`, not itself) — this trait exists to *name*
/// the operations generic code needs, not to reimplement them.
pub trait ScalarLike:
    Copy
    + core::fmt::Debug
    + PartialEq
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
{
    const ZERO: Self;
    const ONE: Self;
    /// A small tolerance for degenerate (near-zero) guards — the same
    /// role `meridian_numeric_core::EPSILON` plays for `f32` and
    /// `fixed_ga`'s own internal tolerance plays for `Fixed`.
    const EPSILON: Self;
    /// The largest finite representable value — used as a "no candidate
    /// yet" sentinel by algorithms that scan for a minimum (e.g. SAT's
    /// least-overlap axis search), not for arithmetic.
    const MAX: Self;

    /// For authoring constants (`0.4` for a moment-of-inertia formula,
    /// `9.81` for gravity, ...) generically. Not meant for hot per-frame
    /// paths on the `Fixed` side (see `Fixed::from_num`'s own doc
    /// comment).
    fn from_f64(v: f64) -> Self;
    fn sqrt(self) -> Self;
    fn tan(self) -> Self;
    fn sin_cos(self) -> (Self, Self);
    fn signum(self) -> Self;
    fn abs(self) -> Self;
    fn min(self, other: Self) -> Self;
    fn max(self, other: Self) -> Self;
    fn clamp(self, lo: Self, hi: Self) -> Self;
}

/// A 3D vector type usable generically — implemented for
/// [`crate::float_ga::Vec3`] and [`crate::fixed_ga::FixedVec3`]. See [`ScalarLike`]'s
/// doc comment for why every method is a thin forward, not a
/// reimplementation.
pub trait VectorLike:
    Copy
    + core::fmt::Debug
    + PartialEq
    + Add<Output = Self>
    + Sub<Output = Self>
    + Neg<Output = Self>
    + Mul<<Self as VectorLike>::Scalar, Output = Self>
{
    type Scalar: ScalarLike;

    const ZERO: Self;

    fn new(x: Self::Scalar, y: Self::Scalar, z: Self::Scalar) -> Self;
    fn x(self) -> Self::Scalar;
    fn y(self) -> Self::Scalar;
    fn z(self) -> Self::Scalar;
    fn dot(self, rhs: Self) -> Self::Scalar;
    fn cross(self, rhs: Self) -> Self;
    fn length(self) -> Self::Scalar;
    fn length_squared(self) -> Self::Scalar;
    fn normalize(self) -> Self;
}

/// A rotation-generator bivector type usable generically —
/// implemented for [`crate::float_ga::Bivector3`] and
/// [`crate::fixed_ga::FixedBivector3`].
pub trait BivectorLike:
    Copy
    + core::fmt::Debug
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<<Self as BivectorLike>::Scalar, Output = Self>
{
    type Scalar: ScalarLike;
    type Vector: VectorLike<Scalar = Self::Scalar>;
    type Rotor: RotorLike<Scalar = Self::Scalar, Vector = Self::Vector>;

    const ZERO: Self;

    fn wedge(a: Self::Vector, b: Self::Vector) -> Self;
    fn length(self) -> Self::Scalar;
    fn exp(self) -> Self::Rotor;
}

/// A pure-rotation type usable generically — implemented for
/// [`crate::float_ga::Rotor`] and [`crate::fixed_ga::FixedRotor`].
pub trait RotorLike: Copy + core::fmt::Debug {
    type Scalar: ScalarLike;
    type Vector: VectorLike<Scalar = Self::Scalar>;

    fn identity() -> Self;
    fn from_axis_angle(axis: Self::Vector, angle: Self::Scalar) -> Self;
    fn compose(self, rhs: Self) -> Self;
    fn reverse(self) -> Self;
    fn transform_vector(self, v: Self::Vector) -> Self::Vector;
}

/// A rigid-motion (rotation + translation) type usable generically —
/// implemented for [`crate::float_ga::Motor3`] and [`crate::fixed_ga::FixedMotor3`].
pub trait MotorLike: Copy + core::fmt::Debug {
    type Scalar: ScalarLike;
    type Vector: VectorLike<Scalar = Self::Scalar>;
    type Rotor: RotorLike<Scalar = Self::Scalar, Vector = Self::Vector>;

    fn identity() -> Self;
    fn translation(t: Self::Vector) -> Self;
    fn from_rotation_translation(rotor: Self::Rotor, t: Self::Vector) -> Self;
    fn compose(self, rhs: Self) -> Self;
    fn inverse(self) -> Self;
    fn transform_point(self, p: Self::Vector) -> Self::Vector;
    fn transform_vector(self, v: Self::Vector) -> Self::Vector;
}

/// One complete "flavor" of the algebra — the associated-type bundle
/// that lets generic code (this crate's own primitives below,
/// `physics-core`'s engine, ...) be written once against `F: GaFlavor`
/// instead of once per scalar type. [`crate::float_ga::FloatFlavor`] and
/// [`crate::fixed_ga::FixedFlavor`] are the two flavors that exist today; a
/// third is exactly as easy to add as implementing this trait (plus the
/// five component traits above) for a new scalar/vector/bivector/rotor/
/// motor set.
pub trait GaFlavor: Copy + Default + core::fmt::Debug + PartialEq + 'static {
    type Scalar: ScalarLike;
    type Vector: VectorLike<Scalar = Self::Scalar>;
    type Bivector: BivectorLike<Scalar = Self::Scalar, Vector = Self::Vector, Rotor = Self::Rotor>;
    type Rotor: RotorLike<Scalar = Self::Scalar, Vector = Self::Vector>;
    type Motor: MotorLike<Scalar = Self::Scalar, Vector = Self::Vector, Rotor = Self::Rotor>;
}

/// A named reference frame: origin + basis, expressed as a motor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame<F: GaFlavor> {
    pub motor: F::Motor,
}

impl<F: GaFlavor> Default for Frame<F> {
    fn default() -> Self {
        Self {
            motor: F::Motor::identity(),
        }
    }
}

/// A convex shape describable by its support function: the point of the
/// shape farthest along a given direction. This is the standard
/// convex-geometry interface (the same one GJK/EPA-style algorithms are
/// built on) that lets a plane or [`ConvexVolume`] test *any* shape
/// without a hardcoded list of shape variants, and lets a new shape (a
/// capsule, a convex hull) work with every existing plane/volume test for
/// free just by implementing this one method — no double-dispatch, no
/// shape x shape combinatorial explosion.
pub trait Shape<F: GaFlavor> {
    fn support(&self, direction: F::Vector) -> F::Vector;
}

// A blanket `impl<F: GaFlavor> Shape<F> for F::Vector` (treating a point
// as a degenerate shape) is deliberately *not* here: Rust's coherence
// checker can't prove `F::Vector` is disjoint from `Aabb<F>`/`Sphere<F>`/
// etc. for every possible future `GaFlavor` impl, so it conflicts with
// the `Shape<F>` impls below. `float_ga`/`fixed_ga` each provide their
// own concrete `impl Shape<FloatFlavor> for Vec3`/
// `impl Shape<FixedFlavor> for FixedVec3` instead — the same
// "a point is a degenerate shape" behavior, just per-flavor.

/// An axis-aligned bounding box: plain spatial-extent math with no domain
/// meaning of its own, shared by every subsystem that needs a cheap
/// overlap/culling test (`physics-core`'s broad phase, `graphics-core`'s
/// frustum culling, ...). The axis-aligned of the two box variants — see
/// [`Obb`] for the oriented one; a cube is either with equal extents on
/// every axis, not a separate type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb<F: GaFlavor> {
    pub min: F::Vector,
    pub max: F::Vector,
}

impl<F: GaFlavor> Default for Aabb<F> {
    fn default() -> Self {
        Self {
            min: F::Vector::ZERO,
            max: F::Vector::ZERO,
        }
    }
}

impl<F: GaFlavor> Aabb<F> {
    pub fn from_sphere(center: F::Vector, radius: F::Scalar) -> Self {
        let r = F::Vector::new(radius, radius, radius);
        Self {
            min: center - r,
            max: center + r,
        }
    }

    /// An axis-aligned cube: equal half-extent on every axis.
    pub fn cube(center: F::Vector, half_extent: F::Scalar) -> Self {
        Self::from_sphere(center, half_extent)
    }

    pub fn overlaps(&self, other: &Aabb<F>) -> bool {
        self.min.x() <= other.max.x()
            && self.max.x() >= other.min.x()
            && self.min.y() <= other.max.y()
            && self.max.y() >= other.min.y()
            && self.min.z() <= other.max.z()
            && self.max.z() >= other.min.z()
    }
}

impl<F: GaFlavor> Shape<F> for Aabb<F> {
    /// Picks the corner furthest along `direction` per axis independently
    /// — this is the exact "positive vertex" trick frustum/AABB tests use
    /// by hand; expressing it as a support function is what lets the same
    /// trick generalize to [`ConvexVolume::intersects`] for every shape.
    fn support(&self, direction: F::Vector) -> F::Vector {
        F::Vector::new(
            if direction.x() >= F::Scalar::ZERO {
                self.max.x()
            } else {
                self.min.x()
            },
            if direction.y() >= F::Scalar::ZERO {
                self.max.y()
            } else {
                self.min.y()
            },
            if direction.z() >= F::Scalar::ZERO {
                self.max.z()
            } else {
                self.min.z()
            },
        )
    }
}

/// A sphere: center + radius. The simplest curved primitive, and the one
/// most subsystems reach for first — `physics-core`'s only default
/// collider shape, a natural bounding volume for `graphics-core` culling
/// later.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sphere<F: GaFlavor> {
    pub center: F::Vector,
    pub radius: F::Scalar,
}

impl<F: GaFlavor> Default for Sphere<F> {
    fn default() -> Self {
        Self {
            center: F::Vector::ZERO,
            radius: F::Scalar::ZERO,
        }
    }
}

impl<F: GaFlavor> Shape<F> for Sphere<F> {
    fn support(&self, direction: F::Vector) -> F::Vector {
        self.center + direction.normalize() * self.radius
    }
}

/// An oriented (rotated) box — the other of the two box variants; see
/// [`Aabb`] for the axis-aligned one. A cube is either with equal
/// `half_extents` on every axis, not a separate type. Position *and*
/// orientation are one `frame: F::Motor`, not a separate center/rotation
/// pair — the same convention every rigid pose in the workspace uses
/// (`RigidBody`, `Camera`, `Listener`/`Emitter`; see docs/gac-design.md),
/// so a physics `RigidBody`'s own `frame` can be used here directly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obb<F: GaFlavor> {
    pub frame: F::Motor,
    pub half_extents: F::Vector,
}

impl<F: GaFlavor> Default for Obb<F> {
    fn default() -> Self {
        Self {
            frame: F::Motor::identity(),
            half_extents: F::Vector::ZERO,
        }
    }
}

impl<F: GaFlavor> Obb<F> {
    /// An oriented cube: equal half-extent on every axis.
    pub fn cube(frame: F::Motor, half_extent: F::Scalar) -> Self {
        Self {
            frame,
            half_extents: F::Vector::new(half_extent, half_extent, half_extent),
        }
    }
}

impl<F: GaFlavor> Shape<F> for Obb<F> {
    fn support(&self, direction: F::Vector) -> F::Vector {
        // Rotate the query direction into the box's local (axis-aligned)
        // space, pick the local corner, then carry that corner back out
        // to world space through the full frame (rotation *and*
        // translation) — the same idea as Aabb::support, done in a frame
        // where the box actually is axis-aligned.
        let local_direction = self.frame.inverse().transform_vector(direction);
        let local_support = F::Vector::new(
            self.half_extents.x() * local_direction.x().signum(),
            self.half_extents.y() * local_direction.y().signum(),
            self.half_extents.z() * local_direction.z().signum(),
        );
        self.frame.transform_point(local_support)
    }
}

/// A right circular cone: apex at `apex`, opening toward `apex + axis *
/// height` (`axis` must be unit length), `half_angle` (radians) between
/// the axis and the cone's slanted surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cone<F: GaFlavor> {
    pub apex: F::Vector,
    pub axis: F::Vector,
    pub half_angle: F::Scalar,
    pub height: F::Scalar,
}

impl<F: GaFlavor> Shape<F> for Cone<F> {
    /// A cone's convex hull is its apex plus its base disk's rim; the
    /// support point is whichever of {apex, the rim point closest to
    /// `direction`} is farther along `direction`. The rim point is found
    /// by projecting `direction` onto the base plane (perpendicular to
    /// `axis`) and walking out to the base radius in that direction —
    /// exactly how a disk's own support function works.
    fn support(&self, direction: F::Vector) -> F::Vector {
        let base_center = self.apex + self.axis * self.height;
        let base_radius = self.height * self.half_angle.tan();

        let along_axis = direction.dot(self.axis);
        let perpendicular = direction - self.axis * along_axis;
        let perpendicular_len = perpendicular.length();

        let rim_point = if perpendicular_len > F::Scalar::EPSILON {
            base_center + perpendicular * (base_radius / perpendicular_len)
        } else {
            base_center
        };

        if direction.dot(self.apex) >= direction.dot(rim_point) {
            self.apex
        } else {
            rim_point
        }
    }
}

/// A half-space `normal . p + d >= 0` — a point satisfying this is on the
/// plane's "inside". Another plain geometric primitive with no domain
/// meaning of its own (`graphics-core`'s `Frustum`/[`ConvexVolume`] is a
/// handful of these; a future physics ground plane or clipping pass would
/// be another consumer), so it lives here rather than being redefined per
/// subsystem — see [`Aabb`]'s doc comment for the same reasoning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane<F: GaFlavor> {
    pub normal: F::Vector,
    pub d: F::Scalar,
}

impl<F: GaFlavor> Plane<F> {
    /// Rescales `normal`/`d` so `normal` is unit length, without changing
    /// which half-space the plane represents. Needed before
    /// [`distance`](Self::distance) is a true Euclidean distance rather
    /// than just a correctly-signed value.
    pub fn normalize(self) -> Self {
        let len = self.normal.length();
        Plane {
            normal: F::Vector::new(
                self.normal.x() / len,
                self.normal.y() / len,
                self.normal.z() / len,
            ),
            d: self.d / len,
        }
    }

    /// Signed distance from `p` to the plane: positive on the "inside"
    /// half-space, negative on the other side.
    pub fn distance(&self, p: F::Vector) -> F::Scalar {
        self.normal.dot(p) + self.d
    }

    /// Whether `shape` is at least partially on this plane's "inside"
    /// half-space: `false` only when `shape` is entirely on the outside.
    /// A shape is fully outside iff *every* point of it has negative
    /// distance, i.e. iff even its best-case point — the one farthest
    /// *along* the normal, `shape.support(normal)` — is still outside; if
    /// that best-case point is inside, the shape can't be fully excluded.
    /// Generic over any [`Shape`], not just [`Aabb`] — this is what makes
    /// [`ConvexVolume::intersects`] work for every shape without the
    /// plane needing to know what kind of shape it's testing.
    pub fn contains<S: Shape<F>>(&self, shape: &S) -> bool {
        self.distance(shape.support(self.normal)) >= F::Scalar::ZERO
    }
}

/// An arbitrary convex region defined as an intersection of half-spaces —
/// the generalization of a camera frustum (always exactly 6 planes) to any
/// number of planes, so it can describe *any* convex bounding volume, not
/// just a camera's view volume. Works against any [`Shape`] via
/// [`Plane::contains`], not a hardcoded shape list.
#[derive(Debug, Clone)]
pub struct ConvexVolume<F: GaFlavor> {
    pub planes: Vec<Plane<F>>,
}

impl<F: GaFlavor> Default for ConvexVolume<F> {
    fn default() -> Self {
        Self { planes: Vec::new() }
    }
}

impl<F: GaFlavor> ConvexVolume<F> {
    pub fn new(planes: Vec<Plane<F>>) -> Self {
        Self { planes }
    }

    /// Conservative test: `false` means `shape` is fully outside at least
    /// one plane (definitely not overlapping this volume); `true` means
    /// it's on the inside half-space of every plane (overlapping, or a
    /// false positive near a corner — the standard convex-volume trade-off,
    /// cheaper than exact separation).
    pub fn intersects<S: Shape<F>>(&self, shape: &S) -> bool {
        self.planes.iter().all(|plane| plane.contains(shape))
    }
}

/// A camera/projective mapping: view-space (right-handed, looking down
/// `-Z`, `+X` right, `+Y` up) to clip space. Column-major, column-vector
/// convention (`M * v`), depth range `[0, 1]` — matches wgpu/DX12/Metal,
/// not classic OpenGL's `[-1, 1]`. This is the one hardcoded convention
/// choice here: `graphics-core`'s `Camera` is responsible for turning a
/// `Motor3`/`FixedMotor3` world frame (whose local-forward axis is a
/// per-subsystem convention, not a `gac-core` one) into this view space
/// before applying a `Projection` — see docs/graphics-design.md. Generic
/// like every other primitive here — most consumers will only ever build
/// one with `F = FloatFlavor` (a GPU pipeline needs `f32` uniforms
/// regardless of which flavor computed the camera's pose, via
/// `to_float_lossy`), but nothing stops a caller from deriving one
/// deterministically in `Fixed` first if that's useful.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection<F: GaFlavor>(pub [[F::Scalar; 4]; 4]);

impl<F: GaFlavor> Default for Projection<F> {
    fn default() -> Self {
        let mut m = [[F::Scalar::ZERO; 4]; 4];
        for (i, row) in m.iter_mut().enumerate() {
            row[i] = F::Scalar::ONE;
        }
        Self(m)
    }
}

impl<F: GaFlavor> Projection<F> {
    /// A right-handed perspective projection. `fov_y_radians` is the full
    /// vertical field of view; `aspect` is width/height; `near`/`far` are
    /// positive view-space distances (`0 < near < far`).
    ///
    /// Derivation (standard perspective projection, re-derived here rather
    /// than copied from a specific library so it can be cross-checked
    /// against hand-computed values in tests): a point `(x, y, z)` in view
    /// space (`z` negative in front of the camera) projects to
    /// `x' = x * f / aspect`, `y' = y * f`, where `f = cot(fov_y / 2)`; the
    /// clip-space `w` is `-z`; depth is mapped to `[0, 1]` via
    /// `z' = far * (z + near) / (z * (far - near))` after the perspective
    /// divide, which requires `z_row = [0, 0, far / (near - far), -1]` and
    /// `w_row = [0, 0, near * far / (near - far), 0]` in the matrix below.
    pub fn perspective(
        fov_y_radians: F::Scalar,
        aspect: F::Scalar,
        near: F::Scalar,
        far: F::Scalar,
    ) -> Self {
        let zero = F::Scalar::ZERO;
        let one = F::Scalar::ONE;
        let two = one + one;
        let f = one / (fov_y_radians / two).tan();
        Self([
            [f / aspect, zero, zero, zero],
            [zero, f, zero, zero],
            [zero, zero, far / (near - far), -one],
            [zero, zero, near * far / (near - far), zero],
        ])
    }

    /// A right-handed orthographic projection over the view-space box
    /// `[left, right] x [bottom, top] x [-far, -near]`, depth mapped to
    /// `[0, 1]`.
    pub fn orthographic(
        left: F::Scalar,
        right: F::Scalar,
        bottom: F::Scalar,
        top: F::Scalar,
        near: F::Scalar,
        far: F::Scalar,
    ) -> Self {
        let zero = F::Scalar::ZERO;
        let one = F::Scalar::ONE;
        let two = one + one;
        Self([
            [two / (right - left), zero, zero, zero],
            [zero, two / (top - bottom), zero, zero],
            [zero, zero, -one / (far - near), zero],
            [
                -(right + left) / (right - left),
                -(top + bottom) / (top - bottom),
                -near / (far - near),
                one,
            ],
        ])
    }
}

/// A single mesh face: the ordered vertex indices of a polygon (not
/// necessarily a triangle — arbitrary point count) into some [`Mesh`]'s
/// `vertices`. Scalar-flavor-independent (indices are plain `usize`), so
/// this lives once, not once per [`GaFlavor`] the way [`Mesh`] itself
/// does (it holds flavor-typed vertex positions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Face {
    pub indices: Vec<usize>,
}

impl Face {
    pub fn triangle(a: usize, b: usize, c: usize) -> Self {
        Self {
            indices: vec![a, b, c],
        }
    }

    /// Fan-triangulates this face: `(indices[0], indices[i], indices[i + 1])`
    /// for every `i` in `1..indices.len() - 1`. Correct for convex faces
    /// — every triangle and quad this workspace's own mesh producers
    /// generate is convex; a non-convex n-gon would need real
    /// ear-clipping, not implemented since nothing here produces one.
    pub fn triangles(&self) -> impl Iterator<Item = (usize, usize, usize)> + '_ {
        let first = self.indices[0];
        self.indices[1..]
            .windows(2)
            .map(move |w| (first, w[0], w[1]))
    }
}

/// An indexed polygon mesh: vertex positions (in some [`GaFlavor`]) plus
/// face topology (flavor-independent). Shared by any subsystem that
/// needs mesh data as *geometry* — `physics-core`'s soft-body particle/
/// spring setup ([`icosphere`] specifically), a future `graphics-core`
/// procedural-mesh path, ... — the same "one primitive, reused, not
/// duplicated per consumer" reasoning as [`Aabb`]/[`Sphere`]/[`Shape`]
/// above. This is deliberately *not* the same type as
/// `asset-core::MeshData`: that type is always `f32`, always
/// pre-triangulated, decoded-from-file-bytes data already shaped for GPU
/// upload (see that crate's own module doc and
/// [dependency-rules.md](../../../docs/dependency-rules.md) rule 4);
/// `Mesh<F>` is for geometry *authored or generated as geometry* (a
/// procedural icosphere, a physics simulation's own topology), generic
/// over scalar flavor the way everything else in this module is.
#[derive(Debug, Clone)]
pub struct Mesh<F: GaFlavor> {
    pub vertices: Vec<F::Vector>,
    pub faces: Vec<Face>,
}

impl<F: GaFlavor> Mesh<F> {
    /// Every edge across every face's fan-triangulation
    /// ([`Face::triangles`]), each returned exactly once regardless of
    /// how many faces share it.
    pub fn unique_edges(&self) -> Vec<(usize, usize)> {
        let mut seen = std::collections::HashSet::new();
        let mut edges = Vec::new();
        for face in &self.faces {
            for (a, b, c) in face.triangles() {
                for (x, y) in [(a, b), (b, c), (c, a)] {
                    let key = if x < y { (x, y) } else { (y, x) };
                    if seen.insert(key) {
                        edges.push(key);
                    }
                }
            }
        }
        edges
    }
}

/// A unit-radius icosphere (centered at the origin — callers translate/
/// scale as needed): a `subdivisions`-times-subdivided icosahedron (`0`
/// = the base 12-vertex icosahedron, each further level roughly
/// quadruples the triangle count), the standard "start from a regular
/// icosahedron, repeatedly split each triangle into 4 by its edge
/// midpoints re-normalized to the unit sphere" construction. The base 12
/// vertices are all even permutations of `(0, ±1, ±φ)` with the golden
/// ratio `φ`, normalized.
pub fn icosphere<F: GaFlavor>(subdivisions: u32) -> Mesh<F> {
    let phi = (F::Scalar::ONE + F::Scalar::from_f64(5.0).sqrt()) / (F::Scalar::ONE + F::Scalar::ONE);
    let raw: [[F::Scalar; 3]; 12] = {
        let z = F::Scalar::ZERO;
        let o = F::Scalar::ONE;
        [
            [-o, phi, z],
            [o, phi, z],
            [-o, -phi, z],
            [o, -phi, z],
            [z, -o, phi],
            [z, o, phi],
            [z, -o, -phi],
            [z, o, -phi],
            [phi, z, -o],
            [phi, z, o],
            [-phi, z, -o],
            [-phi, z, o],
        ]
    };
    let mut vertices: Vec<F::Vector> = raw
        .iter()
        .map(|v| F::Vector::new(v[0], v[1], v[2]).normalize())
        .collect();

    let mut faces = vec![
        Face::triangle(0, 11, 5),
        Face::triangle(0, 5, 1),
        Face::triangle(0, 1, 7),
        Face::triangle(0, 7, 10),
        Face::triangle(0, 10, 11),
        Face::triangle(1, 5, 9),
        Face::triangle(5, 11, 4),
        Face::triangle(11, 10, 2),
        Face::triangle(10, 7, 6),
        Face::triangle(7, 1, 8),
        Face::triangle(3, 9, 4),
        Face::triangle(3, 4, 2),
        Face::triangle(3, 2, 6),
        Face::triangle(3, 6, 8),
        Face::triangle(3, 8, 9),
        Face::triangle(4, 9, 5),
        Face::triangle(2, 4, 11),
        Face::triangle(6, 2, 10),
        Face::triangle(8, 6, 7),
        Face::triangle(9, 8, 1),
    ];

    for _ in 0..subdivisions {
        let mut midpoint_cache = std::collections::HashMap::new();
        let mut next_faces = Vec::with_capacity(faces.len() * 4);
        for face in &faces {
            let (a, b, c) = (face.indices[0], face.indices[1], face.indices[2]);
            let ab = icosphere_midpoint::<F>(&mut vertices, &mut midpoint_cache, a, b);
            let bc = icosphere_midpoint::<F>(&mut vertices, &mut midpoint_cache, b, c);
            let ca = icosphere_midpoint::<F>(&mut vertices, &mut midpoint_cache, c, a);
            next_faces.push(Face::triangle(a, ab, ca));
            next_faces.push(Face::triangle(b, bc, ab));
            next_faces.push(Face::triangle(c, ca, bc));
            next_faces.push(Face::triangle(ab, bc, ca));
        }
        faces = next_faces;
    }

    Mesh { vertices, faces }
}

/// Returns the index of the unit-sphere midpoint between vertices `i`/`j`
/// — reused from `cache` if this edge was already split (shared between
/// two adjacent triangles), otherwise computed and cached. Helper for
/// [`icosphere`]'s subdivision step.
fn icosphere_midpoint<F: GaFlavor>(
    vertices: &mut Vec<F::Vector>,
    cache: &mut std::collections::HashMap<(usize, usize), usize>,
    i: usize,
    j: usize,
) -> usize {
    let key = if i < j { (i, j) } else { (j, i) };
    if let Some(&existing) = cache.get(&key) {
        return existing;
    }
    let mid = (vertices[i] + vertices[j]).normalize();
    let index = vertices.len();
    vertices.push(mid);
    cache.insert(key, index);
    index
}
