//! Fits one orthographic shadow volume around a slice of the view frustum.

use meridian_gac::{Motor3, Projection, Vec3, mat4_mul};

use crate::{Camera, look_at_rotor};

/// How far toward the light the volume reaches past the slice, so casters
/// outside the view still throw shadows into it.
const CASTER_MARGIN: f32 = 50.0;
/// Texels kept clear at the tile's edge for the shader's filter and clamp.
const EDGE_TEXELS: f32 = 4.0;

/// A shadow volume fitted to one cascade slice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CascadeFit {
    /// World to clip space of the light, texel-snapped.
    pub(super) view_proj: [[f32; 4]; 4],
    /// World-space size of one shadow texel.
    pub(super) texel_size: f32,
    /// World-space depth the volume spans along the light.
    pub(super) depth_range: f32,
}

/// Fits the smallest rotation-stable volume around `corners` as seen along
/// `light_direction`, for a square tile of `tile_size` texels.
///
/// The volume is a bounding sphere's box, so turning the camera never
/// changes its size, and its origin is snapped to whole texels so moving the
/// camera does not make shadow edges crawl.
pub(super) fn fit_cascade(
    light_direction: Vec3,
    corners: &[Vec3; 8],
    tile_size: u32,
) -> CascadeFit {
    let tile = texels(tile_size);
    let center = corners.iter().fold(Vec3::ZERO, |sum, &c| sum + c) * (1.0 / 8.0);
    let radius = corners
        .iter()
        .map(|&c| (c - center).length())
        .fold(0.0, f32::max);
    let radius = (radius * 16.0).ceil() / 16.0;
    let half = radius * tile / (tile - 2.0 * EDGE_TEXELS);
    let forward = light_direction.normalize();
    let eye = center - forward * (half + CASTER_MARGIN);
    let depth_range = 2.0 * half + CASTER_MARGIN;
    let light = Camera {
        frame: Motor3::from_rotation_translation(look_at_rotor(eye, center), eye),
        projection: Projection::orthographic((-half, half), (-half, half), (0.0, depth_range)),
    };
    let texel_size = 2.0 * half / tile;
    CascadeFit {
        view_proj: snap_to_texels(light.view_projection_matrix(), tile),
        texel_size,
        depth_range,
    }
}

/// Shifts `view_proj` in light space so the world origin lands on a whole
/// texel of a `tile`-texel map.
fn snap_to_texels(view_proj: [[f32; 4]; 4], tile: f32) -> [[f32; 4]; 4] {
    // The origin's clip position is the last column of an affine matrix.
    let [x, y, _, w] = view_proj[3];
    let texels_x = x / w * tile * 0.5;
    let texels_y = y / w * tile * 0.5;
    let mut shift = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    shift[3][0] = (texels_x.round() - texels_x) * 2.0 / tile;
    shift[3][1] = (texels_y.round() - texels_y) * 2.0 / tile;
    mat4_mul(shift, view_proj)
}

/// A tile's side as a float; tiles are far smaller than `f32` can count exactly.
#[expect(clippy::cast_precision_loss, reason = "tile sizes are far below 2^24")]
pub(super) fn texels(tile_size: u32) -> f32 {
    tile_size as f32
}
