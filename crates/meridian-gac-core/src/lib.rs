//! Geometric Algebra Core — vectors, bivectors, multivectors, rotors and motors; the shared spatial math foundation for every other subsystem.
//!
//! Two flavors, in their own modules: [`float_ga`] (`f32`, the default,
//! GPU-dispatchable path — re-exported at the crate root, so
//! `meridian_gac_core::Vec3`/`Motor3`/etc. resolve to it unchanged) and
//! [`fixed_ga`] (`Fixed`, deterministic, opt-in — used by
//! `physics-core`'s `DeterministicBody` only). See [`fixed_ga`]'s module
//! doc comment for why both exist instead of one generic implementation.
//!
//! The geometric primitives below (`Aabb`, `Sphere`, `Obb`, `Cone`,
//! `Plane`, `Shape`, `ConvexVolume`, `Projection`, `Frame`) build on
//! [`float_ga`] only — they're not (yet) needed by the deterministic
//! path, which today only exercises `DeterministicBody`'s pose/velocity
//! integration, not broad/narrow-phase collision. See docs/gac-design.md
//! and [ADR 001](../../../docs/adr/001-geometric-algebra-as-spatial-model.md).

pub mod fixed_ga;
pub mod float_ga;

pub use float_ga::*;

use meridian_numeric_core::Scalar;

/// A named reference frame: origin + basis, expressed as a motor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Frame {
    pub motor: Motor3,
}

/// A convex shape describable by its support function: the point of the
/// shape farthest along a given direction. This is the standard
/// convex-geometry interface (the same one GJK/EPA-style algorithms are
/// built on) that lets a plane or [`ConvexVolume`] test *any* shape without
/// a hardcoded list of shape variants, and lets a new shape (a capsule, a
/// convex hull, ...) work with every existing plane/volume test for free
/// just by implementing this one method — no double-dispatch, no shape x
/// shape combinatorial explosion.
pub trait Shape {
    fn support(&self, direction: Vec3) -> Vec3;
}

/// A single point, treated as a degenerate shape: its own support point
/// regardless of direction. Lets a single position be tested against a
/// [`Plane`]/[`ConvexVolume`] with the same generic code as any other
/// [`Shape`].
impl Shape for Vec3 {
    fn support(&self, _direction: Vec3) -> Vec3 {
        *self
    }
}

/// An axis-aligned bounding box: plain spatial-extent math with no domain
/// meaning of its own, shared by every subsystem that needs a cheap
/// overlap/culling test (`physics-core`'s broad phase, `graphics-core`'s
/// frustum culling, ...). Lives here rather than in one of those crates so
/// neither re-derives it independently — the same reason `Vec3`/`Motor3`
/// live here instead of in whichever subsystem needed them first (see
/// "Consumers" above). The axis-aligned of the two box variants — see
/// [`Obb`] for the oriented one; a cube is either with equal extents on
/// every axis, not a separate type.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn from_sphere(center: Vec3, radius: Scalar) -> Self {
        let r = Vec3::new(radius, radius, radius);
        Self {
            min: center - r,
            max: center + r,
        }
    }

    /// An axis-aligned cube: equal half-extent on every axis.
    pub fn cube(center: Vec3, half_extent: Scalar) -> Self {
        Self::from_sphere(center, half_extent)
    }

    pub fn overlaps(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }
}

impl Shape for Aabb {
    /// Picks the corner furthest along `direction` per axis independently
    /// — this is the exact "positive vertex" trick frustum/AABB tests use
    /// by hand; expressing it as a support function is what lets the same
    /// trick generalize to [`ConvexVolume::intersects`] for every shape.
    fn support(&self, direction: Vec3) -> Vec3 {
        Vec3::new(
            if direction.x >= 0.0 {
                self.max.x
            } else {
                self.min.x
            },
            if direction.y >= 0.0 {
                self.max.y
            } else {
                self.min.y
            },
            if direction.z >= 0.0 {
                self.max.z
            } else {
                self.min.z
            },
        )
    }
}

/// A sphere: center + radius. The simplest curved primitive, and the one
/// most subsystems reach for first — `physics-core`'s only collider shape
/// today, a natural bounding volume for `graphics-core` culling later.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: Scalar,
}

impl Shape for Sphere {
    fn support(&self, direction: Vec3) -> Vec3 {
        self.center + direction.normalize() * self.radius
    }
}

/// An oriented (rotated) box — the other of the two box variants; see
/// [`Aabb`] for the axis-aligned one. A cube is either with equal
/// `half_extents` on every axis, not a separate type. Position *and*
/// orientation are one `frame: Motor3`, not a separate center/rotation
/// pair — the same convention every rigid pose in the workspace uses
/// (`RigidBody`, `Camera`, `Listener`/`Emitter`; see docs/gac-design.md),
/// so a physics `RigidBody`'s own `frame` can be used here directly.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Obb {
    pub frame: Motor3,
    pub half_extents: Vec3,
}

impl Obb {
    /// An oriented cube: equal half-extent on every axis.
    pub fn cube(frame: Motor3, half_extent: Scalar) -> Self {
        Self {
            frame,
            half_extents: Vec3::new(half_extent, half_extent, half_extent),
        }
    }
}

impl Shape for Obb {
    fn support(&self, direction: Vec3) -> Vec3 {
        // Rotate the query direction into the box's local (axis-aligned)
        // space, pick the local corner, then carry that corner back out
        // to world space through the full frame (rotation *and*
        // translation) — the same idea as Aabb::support, done in a frame
        // where the box actually is axis-aligned.
        let local_direction = self.frame.inverse().transform_vector(direction);
        let local_support = Vec3::new(
            self.half_extents.x * local_direction.x.signum(),
            self.half_extents.y * local_direction.y.signum(),
            self.half_extents.z * local_direction.z.signum(),
        );
        self.frame.transform_point(local_support)
    }
}

/// A right circular cone: apex at `apex`, opening toward `apex + axis *
/// height` (`axis` must be unit length), `half_angle` (radians) between
/// the axis and the cone's slanted surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cone {
    pub apex: Vec3,
    pub axis: Vec3,
    pub half_angle: Scalar,
    pub height: Scalar,
}

impl Shape for Cone {
    /// A cone's convex hull is its apex plus its base disk's rim; the
    /// support point is whichever of {apex, the rim point closest to
    /// `direction`} is farther along `direction`. The rim point is found
    /// by projecting `direction` onto the base plane (perpendicular to
    /// `axis`) and walking out to the base radius in that direction —
    /// exactly how a disk's own support function works.
    fn support(&self, direction: Vec3) -> Vec3 {
        let base_center = self.apex + self.axis * self.height;
        let base_radius = self.height * self.half_angle.tan();

        let along_axis = direction.dot(self.axis);
        let perpendicular = direction - self.axis * along_axis;
        let perpendicular_len = perpendicular.length();

        let rim_point = if perpendicular_len > meridian_numeric_core::EPSILON {
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
pub struct Plane {
    pub normal: Vec3,
    pub d: Scalar,
}

impl Plane {
    /// Rescales `normal`/`d` so `normal` is unit length, without changing
    /// which half-space the plane represents. Needed before
    /// [`distance`](Self::distance) is a true Euclidean distance rather
    /// than just a correctly-signed value.
    pub fn normalize(self) -> Self {
        let len = self.normal.length();
        Plane {
            normal: Vec3::new(
                self.normal.x / len,
                self.normal.y / len,
                self.normal.z / len,
            ),
            d: self.d / len,
        }
    }

    /// Signed distance from `p` to the plane: positive on the "inside"
    /// half-space, negative on the other side.
    pub fn distance(&self, p: Vec3) -> Scalar {
        self.normal.dot(p) + self.d
    }

    /// Whether `shape` is at least partially on this plane's "inside"
    /// half-space: `false` only when `shape` is entirely on the outside.
    /// A shape is fully outside iff *every* point of it has negative
    /// distance, i.e. iff even its best-case point — the one farthest
    /// *along* the normal, `shape.support(normal)` — is still outside; if
    /// that best-case point is inside, the shape can't be fully excluded.
    /// Generic over any [`Shape`], not just [`Aabb`] — this is what makes
    /// [`ConvexVolume::intersects`] work for every shape without the plane
    /// needing to know what kind of shape it's testing.
    pub fn contains<S: Shape>(&self, shape: &S) -> bool {
        self.distance(shape.support(self.normal)) >= 0.0
    }
}

/// An arbitrary convex region defined as an intersection of half-spaces —
/// the generalization of a camera frustum (always exactly 6 planes) to any
/// number of planes, so it can describe *any* convex bounding volume, not
/// just a camera's view volume. Works against any [`Shape`] via
/// [`Plane::contains`], not a hardcoded shape list.
#[derive(Debug, Clone, Default)]
pub struct ConvexVolume {
    pub planes: Vec<Plane>,
}

impl ConvexVolume {
    pub fn new(planes: Vec<Plane>) -> Self {
        Self { planes }
    }

    /// Conservative test: `false` means `shape` is fully outside at least
    /// one plane (definitely not overlapping this volume); `true` means
    /// it's on the inside half-space of every plane (overlapping, or a
    /// false positive near a corner — the standard convex-volume trade-off,
    /// cheaper than exact separation).
    pub fn intersects<S: Shape>(&self, shape: &S) -> bool {
        self.planes.iter().all(|plane| plane.contains(shape))
    }
}

/// A camera/projective mapping: view-space (right-handed, looking down
/// `-Z`, `+X` right, `+Y` up) to clip space. Column-major, column-vector
/// convention (`M * v`), depth range `[0, 1]` — matches wgpu/DX12/Metal,
/// not classic OpenGL's `[-1, 1]`. This is the one hardcoded convention
/// choice here: `graphics-core`'s `Camera` is responsible for turning a
/// `Motor3` world frame (whose local-forward axis is a per-subsystem
/// convention, not a `gac-core` one) into this view space before applying
/// a `Projection` — see docs/graphics-design.md.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection(pub [[Scalar; 4]; 4]);

impl Default for Projection {
    fn default() -> Self {
        let mut m = [[0.0; 4]; 4];
        for (i, row) in m.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        Self(m)
    }
}

impl Projection {
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
    pub fn perspective(fov_y_radians: Scalar, aspect: Scalar, near: Scalar, far: Scalar) -> Self {
        let f = 1.0 / (fov_y_radians * 0.5).tan();
        Self([
            [f / aspect, 0.0, 0.0, 0.0],
            [0.0, f, 0.0, 0.0],
            [0.0, 0.0, far / (near - far), -1.0],
            [0.0, 0.0, near * far / (near - far), 0.0],
        ])
    }

    /// A right-handed orthographic projection over the view-space box
    /// `[left, right] x [bottom, top] x [-far, -near]`, depth mapped to
    /// `[0, 1]`.
    pub fn orthographic(
        left: Scalar,
        right: Scalar,
        bottom: Scalar,
        top: Scalar,
        near: Scalar,
        far: Scalar,
    ) -> Self {
        Self([
            [2.0 / (right - left), 0.0, 0.0, 0.0],
            [0.0, 2.0 / (top - bottom), 0.0, 0.0],
            [0.0, 0.0, -1.0 / (far - near), 0.0],
            [
                -(right + left) / (right - left),
                -(top + bottom) / (top - bottom),
                -near / (far - near),
                1.0,
            ],
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::PI;

    fn assert_vec3_approx(a: Vec3, b: Vec3) {
        assert!(
            a.approx_eq(b),
            "expected {b:?} to approximately equal {a:?}"
        );
    }

    /// Independent oracle for `Projection::perspective`: a point placed
    /// exactly on the frustum's side/top planes at a known depth must land
    /// on clip-space's `+-w` boundary (NDC `+-1` after the perspective
    /// divide), computed from plain trigonometry, not the matrix itself.
    #[test]
    fn perspective_projects_frustum_boundary_points_to_ndc_edges() {
        let fov_y = PI / 2.0; // 90 degrees
        let aspect = 16.0 / 9.0;
        let near = 0.1;
        let far = 100.0;
        let proj = Projection::perspective(fov_y, aspect, near, far);

        let depth = 10.0_f32;
        let half_height = depth * (fov_y * 0.5).tan();
        let half_width = half_height * aspect;

        let project = |p: [Scalar; 4]| -> [Scalar; 4] {
            let m = proj.0;
            [
                m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0] * p[3],
                m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1] * p[3],
                m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2] * p[3],
                m[0][3] * p[0] + m[1][3] * p[1] + m[2][3] * p[2] + m[3][3] * p[3],
            ]
        };

        let top_edge = project([0.0, half_height, -depth, 1.0]);
        assert!((top_edge[1] / top_edge[3] - 1.0).abs() < 1e-4);
        let right_edge = project([half_width, 0.0, -depth, 1.0]);
        assert!((right_edge[0] / right_edge[3] - 1.0).abs() < 1e-4);

        let at_near = project([0.0, 0.0, -near, 1.0]);
        assert!(
            (at_near[2] / at_near[3]).abs() < 1e-5,
            "near plane maps to depth 0"
        );
        let at_far = project([0.0, 0.0, -far, 1.0]);
        assert!(
            (at_far[2] / at_far[3] - 1.0).abs() < 1e-5,
            "far plane maps to depth 1"
        );
    }

    #[test]
    fn orthographic_maps_box_corners_to_ndc_cube() {
        let proj = Projection::orthographic(-2.0, 2.0, -1.0, 1.0, 0.5, 10.0);
        let m = proj.0;
        let project = |p: [Scalar; 4]| -> [Scalar; 4] {
            [
                m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2] + m[3][0] * p[3],
                m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2] + m[3][1] * p[3],
                m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2] + m[3][2] * p[3],
                m[0][3] * p[0] + m[1][3] * p[1] + m[2][3] * p[2] + m[3][3] * p[3],
            ]
        };
        // Orthographic w stays 1: no perspective divide needed.
        let near_corner = project([-2.0, -1.0, -0.5, 1.0]);
        assert!((near_corner[0] - -1.0).abs() < 1e-5);
        assert!((near_corner[1] - -1.0).abs() < 1e-5);
        assert!((near_corner[2] - 0.0).abs() < 1e-5);
        let far_corner = project([2.0, 1.0, -10.0, 1.0]);
        assert!((far_corner[0] - 1.0).abs() < 1e-5);
        assert!((far_corner[1] - 1.0).abs() < 1e-5);
        assert!((far_corner[2] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn point_support_is_always_itself() {
        let p = Vec3::new(3.0, -2.0, 7.0);
        assert_eq!(p.support(Vec3::X), p);
        assert_eq!(p.support(-Vec3::Z), p);
    }

    #[test]
    fn sphere_support_is_center_plus_radius_along_direction() {
        let sphere = Sphere {
            center: Vec3::new(1.0, 2.0, 3.0),
            radius: 5.0,
        };
        assert_vec3_approx(sphere.support(Vec3::X), Vec3::new(6.0, 2.0, 3.0));
        assert_vec3_approx(sphere.support(-Vec3::X), Vec3::new(-4.0, 2.0, 3.0));
    }

    #[test]
    fn sphere_support_degenerate_direction_returns_center() {
        // Vec3::normalize's documented behavior for a near-zero vector is
        // to return it unchanged, so a zero query direction degenerates to
        // "no displacement" rather than a divide-by-zero.
        let sphere = Sphere {
            center: Vec3::new(1.0, 2.0, 3.0),
            radius: 5.0,
        };
        assert_vec3_approx(sphere.support(Vec3::ZERO), sphere.center);
    }

    #[test]
    fn aabb_cube_has_equal_half_extents() {
        let cube = Aabb::cube(Vec3::new(1.0, 1.0, 1.0), 2.0);
        assert_vec3_approx(cube.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_vec3_approx(cube.max, Vec3::new(3.0, 3.0, 3.0));
    }

    #[test]
    fn aabb_support_matches_frustum_style_positive_vertex() {
        let aabb = Aabb {
            min: Vec3::new(-1.0, -2.0, -3.0),
            max: Vec3::new(4.0, 5.0, 6.0),
        };
        assert_vec3_approx(aabb.support(Vec3::new(1.0, 1.0, 1.0)), aabb.max);
        assert_vec3_approx(aabb.support(Vec3::new(-1.0, -1.0, -1.0)), aabb.min);
    }

    #[test]
    fn obb_with_identity_frame_matches_aabb_support() {
        let center = Vec3::new(2.0, 0.0, -1.0);
        let half_extents = Vec3::new(1.0, 2.0, 3.0);
        let obb = Obb {
            frame: Motor3::translation(center),
            half_extents,
        };
        let aabb = Aabb {
            min: center - half_extents,
            max: center + half_extents,
        };
        for direction in [Vec3::X, -Vec3::X, Vec3::Y, Vec3::new(1.0, 1.0, 1.0)] {
            assert_vec3_approx(obb.support(direction), aabb.support(direction));
        }
    }

    #[test]
    fn obb_rotated_90_degrees_about_z_swaps_x_and_y_extents() {
        // A box rotated 90 degrees about Z: its local +X half-extent
        // (3.0) now points along world +Y, so querying support along
        // world +Y must reach out by 3.0, not the local y half-extent
        // (1.0). Checking only the Y component deliberately: a query
        // exactly along one local axis lands on a box *edge* (every
        // combination of the other two axes' signs is an equally valid
        // support point there), so the other components are a tie-break
        // detail, not a claim this test should make.
        let obb = Obb {
            frame: Motor3::rotation(Vec3::Z, PI / 2.0),
            half_extents: Vec3::new(3.0, 1.0, 1.0),
        };
        assert!((obb.support(Vec3::Y).y - 3.0).abs() < 1e-4);
    }

    #[test]
    fn obb_cube_has_equal_half_extents() {
        let obb = Obb::cube(Motor3::identity(), 2.0);
        assert_vec3_approx(obb.half_extents, Vec3::new(2.0, 2.0, 2.0));
    }

    fn test_cone() -> Cone {
        // Apex at the origin, opening toward +Z, half-angle 45 degrees
        // (tan == 1, so base_radius == height — easy to check by hand).
        Cone {
            apex: Vec3::ZERO,
            axis: Vec3::Z,
            half_angle: PI / 4.0,
            height: 2.0,
        }
    }

    #[test]
    fn cone_support_backward_along_axis_is_the_apex() {
        let cone = test_cone();
        assert_vec3_approx(cone.support(-Vec3::Z), Vec3::ZERO);
    }

    #[test]
    fn cone_support_forward_along_axis_is_on_the_base_rim() {
        let cone = test_cone();
        // Zero lateral component: Cone::support's degenerate branch
        // returns the base center itself, which is the farthest point
        // straight down the axis.
        assert_vec3_approx(cone.support(Vec3::Z), Vec3::new(0.0, 0.0, 2.0));
    }

    #[test]
    fn cone_support_sideways_reaches_the_base_rim_not_the_apex() {
        let cone = test_cone();
        // base_radius = height * tan(45deg) = 2.0, so straight sideways
        // (+X) the support point is the rim point (2, 0, 2), clearly
        // farther along +X than the apex at the origin.
        assert_vec3_approx(cone.support(Vec3::X), Vec3::new(2.0, 0.0, 2.0));
    }

    #[test]
    fn plane_contains_uses_the_shapes_best_case_point() {
        // Inside is x >= 0.
        let plane = Plane {
            normal: Vec3::X,
            d: 0.0,
        };
        assert!(plane.contains(&Vec3::new(5.0, 0.0, 0.0)));
        assert!(!plane.contains(&Vec3::new(-5.0, 0.0, 0.0)));

        // A sphere straddling the plane (center just behind it, radius
        // large enough to poke through) must count as "contained" —
        // conservative, not "fully inside".
        let straddling = Sphere {
            center: Vec3::new(-0.5, 0.0, 0.0),
            radius: 1.0,
        };
        assert!(plane.contains(&straddling));

        // A sphere entirely on the excluded side must not.
        let excluded = Sphere {
            center: Vec3::new(-5.0, 0.0, 0.0),
            radius: 1.0,
        };
        assert!(!plane.contains(&excluded));
    }

    #[test]
    fn convex_volume_generalizes_intersects_to_any_shape() {
        // A 2x2x2 cube volume centered at the origin, one plane per face,
        // each normal pointing inward.
        let volume = ConvexVolume::new(vec![
            Plane {
                normal: Vec3::X,
                d: 1.0,
            },
            Plane {
                normal: -Vec3::X,
                d: 1.0,
            },
            Plane {
                normal: Vec3::Y,
                d: 1.0,
            },
            Plane {
                normal: -Vec3::Y,
                d: 1.0,
            },
            Plane {
                normal: Vec3::Z,
                d: 1.0,
            },
            Plane {
                normal: -Vec3::Z,
                d: 1.0,
            },
        ]);

        assert!(volume.intersects(&Sphere {
            center: Vec3::ZERO,
            radius: 0.5,
        }));
        assert!(!volume.intersects(&Sphere {
            center: Vec3::new(10.0, 0.0, 0.0),
            radius: 0.5,
        }));
        assert!(volume.intersects(&Aabb::cube(Vec3::new(0.9, 0.0, 0.0), 0.5)));
        assert!(volume.intersects(&Obb::cube(Motor3::identity(), 0.5)));
    }
}
