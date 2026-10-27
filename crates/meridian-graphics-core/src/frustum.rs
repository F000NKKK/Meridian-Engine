//! A view frustum for culling.

use meridian_gac_core::FloatFlavor;
use meridian_gac_core::Shape;
use meridian_gac_core::{ConvexVolume, Plane, Vec3};

/// The six half-spaces bounding a camera's visible volume, extracted from
/// a view-projection matrix via the Gribb/Hartmann method: each plane's
/// coefficients are a linear combination of the matrix's rows, chosen so
/// that "inside" matches the clip-space bounds `Projection` documents
/// (`-w <= x,y <= w`, `0 <= z <= w`). A thin wrapper around `gac-core`'s
/// [`ConvexVolume`]: extracting exactly six planes from a `Projection` is
/// what's camera-specific and belongs here; the underlying "is this shape
/// inside every plane" test is generic and lives once in `gac-core` (see
/// docs/graphics-design.md) — [`Frustum::intersects`] works for *any*
/// [`Shape`], not just an AABB.
#[derive(Debug, Clone)]
pub struct Frustum {
    volume: ConvexVolume,
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
            volume: ConvexVolume::new(vec![
                make(combine(w, x, 1.0)),  // left:   x + w >= 0
                make(combine(w, x, -1.0)), // right:  w - x >= 0
                make(combine(w, y, 1.0)),  // bottom: y + w >= 0
                make(combine(w, y, -1.0)), // top:    w - y >= 0
                make(z),                   // near:   z >= 0
                make(combine(w, z, -1.0)), // far:    w - z >= 0
            ]),
        }
    }

    /// Conservative test: `false` means `shape` is fully outside at least
    /// one plane (definitely not visible); `true` means it's inside every
    /// plane's half-space (visible, or a false positive near a corner —
    /// standard shape/frustum trade-off, cheaper than exact separation).
    pub fn intersects<S: Shape<FloatFlavor>>(&self, shape: &S) -> bool {
        self.volume.intersects(shape)
    }
}
