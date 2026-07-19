//! High-level rendering: render graph, scene extraction, visibility/culling, lighting, materials, camera and post-processing.

use std::collections::{HashMap, VecDeque};

use meridian_gac_core::{Motor3, Projection, Vec3};
use meridian_resource_core::ResourceId;

/// Marker types distinguishing `ResourceId`s of different graphics resource
/// kinds — see docs/adr/006-resource-core-separation.md.
pub struct TextureMarker;
pub struct MeshMarker;
pub struct ShaderMarker;

pub type TextureHandle = ResourceId<TextureMarker>;
pub type MeshHandle = ResourceId<MeshMarker>;
pub type ShaderHandle = ResourceId<ShaderMarker>;

/// Identifies a resource within one [`RenderGraph`] — assigned by whoever
/// builds the graph (e.g. an index into a per-frame resource table), not
/// `resource-core`'s persistent `ResourceId`: graph resources are
/// transient and frame-scoped, often not existing (an intermediate render
/// target) until the graph itself creates them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraphResourceId(pub u32);

/// A node in the render graph: declares its resource reads/writes; the
/// graph derives execution order from that, not from manual sequencing.
#[derive(Debug, Clone, Default)]
pub struct RenderPass {
    pub name: &'static str,
    pub reads: Vec<GraphResourceId>,
    pub writes: Vec<GraphResourceId>,
}

impl RenderPass {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            reads: Vec::new(),
            writes: Vec::new(),
        }
    }

    pub fn reading(mut self, resource: GraphResourceId) -> Self {
        self.reads.push(resource);
        self
    }

    pub fn writing(mut self, resource: GraphResourceId) -> Self {
        self.writes.push(resource);
        self
    }
}

/// Why [`RenderGraph::execution_order`] couldn't derive a valid order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderGraphError {
    /// Two passes both declared a write to the same resource — this graph
    /// has no rule for which write a later read should see, so it refuses
    /// to guess rather than silently picking one (e.g. insertion order).
    MultipleWriters {
        resource: GraphResourceId,
        first_writer: usize,
        second_writer: usize,
    },
    /// The reads/writes declared form a cycle (A reads what B writes, B
    /// reads what A writes) — no valid execution order exists.
    Cycle,
}

/// An automatically-ordered set of render passes for one frame. Passes
/// declare resource reads/writes ([`RenderPass::reading`]/
/// [`RenderPass::writing`]); [`execution_order`](Self::execution_order)
/// derives a valid pass order from those declarations — a producer always
/// runs before every pass that reads what it wrote — the same
/// dependency-declared-not-hand-sequenced idea as `task-core`'s `JobGraph`
/// (Kahn's algorithm), applied to resource conflicts instead of explicit
/// job dependencies. A resource nobody writes (e.g. an externally-supplied
/// swapchain texture) constrains nothing; it's available from the start.
#[derive(Debug, Clone, Default)]
pub struct RenderGraph {
    pub passes: Vec<RenderPass>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a pass, returning its index for use elsewhere (e.g. matching
    /// `execution_order`'s output back to which pass ran).
    pub fn add_pass(&mut self, pass: RenderPass) -> usize {
        self.passes.push(pass);
        self.passes.len() - 1
    }

    /// Derives a valid execution order (pass indices into `self.passes`)
    /// from each pass's declared reads/writes.
    pub fn execution_order(&self) -> Result<Vec<usize>, RenderGraphError> {
        let n = self.passes.len();

        let mut writer: HashMap<GraphResourceId, usize> = HashMap::new();
        for (i, pass) in self.passes.iter().enumerate() {
            for &resource in &pass.writes {
                if let Some(&first_writer) = writer.get(&resource) {
                    return Err(RenderGraphError::MultipleWriters {
                        resource,
                        first_writer,
                        second_writer: i,
                    });
                }
                writer.insert(resource, i);
            }
        }

        let mut indegree = vec![0usize; n];
        let mut dependents: Vec<Vec<usize>> = (0..n).map(|_| Vec::new()).collect();
        for (i, pass) in self.passes.iter().enumerate() {
            for &resource in &pass.reads {
                if let Some(&producer) = writer.get(&resource)
                    && producer != i
                {
                    indegree[i] += 1;
                    dependents[producer].push(i);
                }
            }
        }

        let mut queue: VecDeque<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
        let mut order = Vec::with_capacity(n);
        while let Some(i) = queue.pop_front() {
            order.push(i);
            for &d in &dependents[i] {
                indegree[d] -= 1;
                if indegree[d] == 0 {
                    queue.push_back(d);
                }
            }
        }

        if order.len() != n {
            return Err(RenderGraphError::Cycle);
        }
        Ok(order)
    }
}

/// Remaps `gac-core`'s local camera axes (forward `+X`, up `+Y`, right
/// `+Z` — the same listener-local convention `audio-core` already commits
/// to, reused here so a character's camera and ears agree on "forward"
/// without either subsystem inventing its own axis convention) onto
/// `Projection`'s documented view space (forward `-Z`, up `+Y`, right
/// `+X`). `gac-core` itself has no "forward" concept (see
/// docs/gac-design.md); this remap is a graphics-specific convention
/// choice and stays in `graphics-core`, not `gac-core`.
const LOCAL_TO_VIEW_REMAP: [[f32; 4]; 4] = [
    [0.0, 0.0, -1.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Column-major 4x4 matrix multiply (`lhs * rhs`, column-vector
/// convention). Kept local to `graphics-core` rather than promoted to
/// `gac-core`/`numeric-core`: it's plain matrix arithmetic on the raw
/// arrays `Motor3::to_mat4`/`Projection` already return, needed here to
/// compose a view-projection matrix. If a second crate needs generic mat4
/// multiply, that's the signal to move it down into `numeric-core`, not
/// before.
fn mat4_mul(lhs: [[f32; 4]; 4], rhs: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for (col, rhs_col) in rhs.iter().enumerate() {
        for row in 0..4 {
            out[col][row] = (0..4).map(|k| lhs[k][row] * rhs_col[k]).sum();
        }
    }
    out
}

/// A camera's view + projection: `frame` is the camera's world transform
/// (local-forward `+X`, see [`LOCAL_TO_VIEW_REMAP`]), `projection` is the
/// view-to-clip mapping built via [`Projection::perspective`] or
/// [`Projection::orthographic`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Camera {
    pub frame: Motor3,
    pub projection: Projection,
}

impl Camera {
    /// World-space to view-space matrix: the camera's world frame
    /// inverted (world-to-camera-local), then remapped from `gac-core`'s
    /// local forward-`+X` convention to `Projection`'s forward-`-Z` view
    /// space.
    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        mat4_mul(LOCAL_TO_VIEW_REMAP, self.frame.inverse().to_mat4())
    }

    /// World-space to clip-space matrix: `projection * view`.
    pub fn view_projection_matrix(&self) -> [[f32; 4]; 4] {
        mat4_mul(self.projection.0, self.view_matrix())
    }
}

/// An axis-aligned bounding box in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

/// A half-space `normal . p + d >= 0` — a point satisfying this is on the
/// frustum's inside of the plane.
#[derive(Debug, Clone, Copy)]
struct Plane {
    normal: Vec3,
    d: f32,
}

impl Plane {
    fn normalize(self) -> Self {
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

    fn distance(&self, p: Vec3) -> f32 {
        self.normal.dot(p) + self.d
    }
}

/// The six half-spaces bounding a camera's visible volume, extracted from
/// a view-projection matrix via the Gribb/Hartmann method: each plane's
/// coefficients are a linear combination of the matrix's rows, chosen so
/// that "inside" matches the clip-space bounds `Projection` documents
/// (`-w <= x,y <= w`, `0 <= z <= w`). Plane extraction, not object
/// culling, is what ties this to a specific `Projection`/`Camera` — the
/// AABB test below is generic once the planes exist.
#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    pub fn from_view_projection(m: [[f32; 4]; 4]) -> Self {
        // Row i of a column-major matrix m[col][row]: coefficients that,
        // dotted with (x, y, z, 1), produce clip-space's i-th component.
        let row = |i: usize| [m[0][i], m[1][i], m[2][i], m[3][i]];
        let (x, y, z, w) = (row(0), row(1), row(2), row(3));
        let combine = |a: [f32; 4], b: [f32; 4], sign: f32| {
            [
                a[0] + sign * b[0],
                a[1] + sign * b[1],
                a[2] + sign * b[2],
                a[3] + sign * b[3],
            ]
        };
        let make = |c: [f32; 4]| {
            Plane {
                normal: Vec3::new(c[0], c[1], c[2]),
                d: c[3],
            }
            .normalize()
        };
        Frustum {
            planes: [
                make(combine(w, x, 1.0)),  // left:   x + w >= 0
                make(combine(w, x, -1.0)), // right:  w - x >= 0
                make(combine(w, y, 1.0)),  // bottom: y + w >= 0
                make(combine(w, y, -1.0)), // top:    w - y >= 0
                make(z),                   // near:   z >= 0
                make(combine(w, z, -1.0)), // far:    w - z >= 0
            ],
        }
    }

    /// Conservative test: `false` means the AABB is fully outside at least
    /// one plane (definitely not visible); `true` means it's inside every
    /// plane's half-space (visible, or a false positive near a corner —
    /// standard AABB/frustum trade-off, cheaper than exact separation).
    pub fn intersects_aabb(&self, aabb: Aabb) -> bool {
        for plane in &self.planes {
            let positive = Vec3::new(
                if plane.normal.x >= 0.0 {
                    aabb.max.x
                } else {
                    aabb.min.x
                },
                if plane.normal.y >= 0.0 {
                    aabb.max.y
                } else {
                    aabb.min.y
                },
                if plane.normal.z >= 0.0 {
                    aabb.max.z
                } else {
                    aabb.min.z
                },
            );
            if plane.distance(positive) < 0.0 {
                return false;
            }
        }
        true
    }
}

/// A surface's shading inputs.
#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub albedo: TextureHandle,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transform_via_matrix(m: [[f32; 4]; 4], p: Vec3) -> Vec3 {
        Vec3::new(
            m[0][0] * p.x + m[1][0] * p.y + m[2][0] * p.z + m[3][0],
            m[0][1] * p.x + m[1][1] * p.y + m[2][1] * p.z + m[3][1],
            m[0][2] * p.x + m[1][2] * p.y + m[2][2] * p.z + m[3][2],
        )
    }

    fn assert_vec3_approx(a: Vec3, b: Vec3) {
        assert!((a - b).length() < 1e-4, "expected {b:?}, got {a:?}");
    }

    #[test]
    fn identity_camera_puts_forward_point_on_negative_view_z() {
        let camera = Camera {
            frame: Motor3::identity(),
            projection: Projection::default(),
        };
        // World +X is this camera's local forward (see LOCAL_TO_VIEW_REMAP);
        // view space must see it straight ahead, on -Z, with x = y = 0.
        let view_point = transform_via_matrix(camera.view_matrix(), Vec3::new(5.0, 0.0, 0.0));
        assert_vec3_approx(view_point, Vec3::new(0.0, 0.0, -5.0));
    }

    #[test]
    fn translated_camera_sees_world_origin_at_its_own_distance() {
        let camera = Camera {
            frame: Motor3::translation(Vec3::new(10.0, 0.0, 0.0)),
            projection: Projection::default(),
        };
        // Camera sits at world (10,0,0) facing world +X (local forward);
        // the world origin is now directly behind it, i.e. +Z in view space.
        let view_point = transform_via_matrix(camera.view_matrix(), Vec3::ZERO);
        assert_vec3_approx(view_point, Vec3::new(0.0, 0.0, 10.0));
    }

    #[test]
    fn view_projection_matches_perspective_projection_of_view_space() {
        let camera = Camera {
            frame: Motor3::identity(),
            projection: Projection::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0),
        };
        let world_point = Vec3::new(5.0, 0.0, 0.0);
        let vp = camera.view_projection_matrix();
        let direct = mat4_mul(camera.projection.0, camera.view_matrix());
        assert_eq!(vp, direct);

        // A point straight ahead on the forward axis must land at clip
        // x = y = 0 (dead center of the screen) regardless of depth.
        let m = vp;
        let p = world_point;
        let clip = [
            m[0][0] * p.x + m[1][0] * p.y + m[2][0] * p.z + m[3][0],
            m[0][1] * p.x + m[1][1] * p.y + m[2][1] * p.z + m[3][1],
        ];
        assert!(clip[0].abs() < 1e-4 && clip[1].abs() < 1e-4);
    }

    fn test_frustum() -> Frustum {
        // Identity camera facing world +X, 90-degree vertical FOV, square
        // aspect, near=0.1, far=100 — an easy frustum to reason about by
        // hand: at depth d in front of the camera, the visible half-width
        // and half-height are both exactly d (tan(45 deg) == 1).
        let camera = Camera {
            frame: Motor3::identity(),
            projection: Projection::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0),
        };
        Frustum::from_view_projection(camera.view_projection_matrix())
    }

    #[test]
    fn frustum_contains_a_box_directly_ahead() {
        let frustum = test_frustum();
        let aabb = Aabb {
            min: Vec3::new(9.0, -0.5, -0.5),
            max: Vec3::new(11.0, 0.5, 0.5),
        };
        assert!(frustum.intersects_aabb(aabb));
    }

    #[test]
    fn frustum_rejects_a_box_behind_the_camera() {
        let frustum = test_frustum();
        let aabb = Aabb {
            min: Vec3::new(-11.0, -0.5, -0.5),
            max: Vec3::new(-9.0, 0.5, 0.5),
        };
        assert!(!frustum.intersects_aabb(aabb));
    }

    #[test]
    fn frustum_rejects_a_box_beyond_the_far_plane() {
        let frustum = test_frustum();
        let aabb = Aabb {
            min: Vec3::new(200.0, -0.5, -0.5),
            max: Vec3::new(201.0, 0.5, 0.5),
        };
        assert!(!frustum.intersects_aabb(aabb));
    }

    #[test]
    fn frustum_rejects_a_box_far_outside_the_side_planes() {
        let frustum = test_frustum();
        // At depth 10 the half-width is exactly 10 (90-degree FOV), so a
        // box sitting entirely beyond z=15 (local right, see
        // LOCAL_TO_VIEW_REMAP) is well outside the right plane.
        let aabb = Aabb {
            min: Vec3::new(9.0, -0.5, 15.0),
            max: Vec3::new(11.0, 0.5, 16.0),
        };
        assert!(!frustum.intersects_aabb(aabb));
    }

    #[test]
    fn frustum_accepts_a_box_straddling_the_right_edge() {
        let frustum = test_frustum();
        // Half-width at depth 10 is 10, so a box straddling x=10 (world
        // right) only partially crosses the boundary — its near corner
        // (in local +Z / right terms, x=9..11 here maps to depth 9..11,
        // but the straddling axis is local Z which becomes view X) must
        // still be reported visible (conservative AABB test).
        let aabb = Aabb {
            min: Vec3::new(9.0, -0.5, 8.0),
            max: Vec3::new(11.0, 0.5, 12.0),
        };
        assert!(frustum.intersects_aabb(aabb));
    }

    #[test]
    fn render_graph_orders_linear_producer_consumer_chain() {
        let shadow_map = GraphResourceId(0);
        let hdr_color = GraphResourceId(1);
        let ldr_color = GraphResourceId(2);

        let mut graph = RenderGraph::new();
        let lighting = graph.add_pass(
            RenderPass::new("lighting")
                .reading(shadow_map)
                .writing(hdr_color),
        );
        let shadow = graph.add_pass(RenderPass::new("shadow").writing(shadow_map));
        let tonemap = graph.add_pass(
            RenderPass::new("tonemap")
                .reading(hdr_color)
                .writing(ldr_color),
        );

        let order = graph.execution_order().unwrap();
        assert_eq!(order.len(), 3);
        let pos = |i: usize| order.iter().position(|&p| p == i).unwrap();
        assert!(
            pos(shadow) < pos(lighting),
            "shadow must run before lighting"
        );
        assert!(
            pos(lighting) < pos(tonemap),
            "lighting must run before tonemap"
        );
    }

    #[test]
    fn render_graph_leaves_independent_passes_unordered_between_each_other() {
        let a_out = GraphResourceId(0);
        let b_out = GraphResourceId(1);

        let mut graph = RenderGraph::new();
        graph.add_pass(RenderPass::new("a").writing(a_out));
        graph.add_pass(RenderPass::new("b").writing(b_out));

        // Neither pass reads what the other writes, so any order derived
        // is valid — the graph must still terminate and include both.
        let order = graph.execution_order().unwrap();
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn render_graph_treats_externally_supplied_resources_as_available() {
        // "backbuffer" has no writer inside this graph (e.g. the
        // swapchain image) — a pass reading it must not be blocked.
        let backbuffer = GraphResourceId(0);
        let mut graph = RenderGraph::new();
        graph.add_pass(RenderPass::new("present").reading(backbuffer));
        assert_eq!(graph.execution_order().unwrap(), vec![0]);
    }

    #[test]
    fn render_graph_rejects_two_passes_writing_the_same_resource() {
        let target = GraphResourceId(0);
        let mut graph = RenderGraph::new();
        graph.add_pass(RenderPass::new("a").writing(target));
        graph.add_pass(RenderPass::new("b").writing(target));

        let err = graph.execution_order().unwrap_err();
        assert_eq!(
            err,
            RenderGraphError::MultipleWriters {
                resource: target,
                first_writer: 0,
                second_writer: 1,
            }
        );
    }

    #[test]
    fn render_graph_rejects_a_read_write_cycle() {
        let a_res = GraphResourceId(0);
        let b_res = GraphResourceId(1);
        let mut graph = RenderGraph::new();
        graph.add_pass(RenderPass::new("a").reading(b_res).writing(a_res));
        graph.add_pass(RenderPass::new("b").reading(a_res).writing(b_res));

        assert_eq!(
            graph.execution_order().unwrap_err(),
            RenderGraphError::Cycle
        );
    }
}
