//! High-level rendering: render graph, scene extraction, visibility/culling, lighting, materials, camera and post-processing.

use meridian_gac_core::{Motor3, Projection};
use meridian_resource_core::ResourceId;

/// Marker types distinguishing `ResourceId`s of different graphics resource
/// kinds — see docs/adr/006-resource-core-separation.md.
pub struct TextureMarker;
pub struct MeshMarker;
pub struct ShaderMarker;

pub type TextureHandle = ResourceId<TextureMarker>;
pub type MeshHandle = ResourceId<MeshMarker>;
pub type ShaderHandle = ResourceId<ShaderMarker>;

/// A node in the render graph: declares its resource reads/writes; the
/// graph derives execution order from that, not from manual sequencing.
#[derive(Debug, Clone, Default)]
pub struct RenderPass {
    pub name: &'static str,
}

/// An automatically-ordered set of render passes for one frame.
#[derive(Debug, Clone, Default)]
pub struct RenderGraph {
    pub passes: Vec<RenderPass>,
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

/// A surface's shading inputs.
#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub albedo: TextureHandle,
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_gac_core::Vec3;

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
}
