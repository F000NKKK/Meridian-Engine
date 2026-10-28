//! The cascaded shadow volumes of one view.

use meridian_gac::Vec3;

use super::cascade_fit::{CascadeFit, fit_cascade, texels};
use super::cascade_frustum::ViewVolume;
use super::directional_shadow_camera;
use super::shadow::SHADOW_VOLUME_HALF_EXTENT;
use crate::{Camera, Frustum};

/// How many cascades split the view distance; the adapter lays their maps
/// out as a 2 x 2 atlas.
pub const SHADOW_CASCADES: usize = 4;
/// The farthest view distance that receives shadows (ADR 037).
pub const MAX_SHADOW_DISTANCE: f32 = 100.0;
/// Blend of logarithmic (1.0) and uniform (0.0) split spacing.
const SPLIT_LAMBDA: f32 = 0.75;
/// One orthographic shadow volume per cascade, near to far, plus what the
/// lit shaders need to pick and bias them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowCascades {
    /// World to clip space of each cascade's light.
    pub view_proj: [[[f32; 4]; 4]; SHADOW_CASCADES],
    /// View distance at which each cascade ends; a surface farther than the
    /// last one is unshadowed.
    pub far_edges: [f32; SHADOW_CASCADES],
    /// Normalised depth of one shadow texel's world size, per cascade: the
    /// unit the shaders' depth bias is counted in.
    pub bias_per_texel: [f32; SHADOW_CASCADES],
}

impl ShadowCascades {
    /// Cascades fitted to `camera`'s view frustum for a light travelling along
    /// `light_direction`, each rendered into a tile of `tile_size` texels. A
    /// camera without a perspective projection gets the fixed volume of
    /// [`Self::fixed`].
    #[must_use]
    pub fn fit(camera: &Camera, light_direction: Vec3, tile_size: u32) -> Self {
        let Some(volume) = ViewVolume::of(camera) else {
            return Self::fixed(light_direction, tile_size);
        };
        let far = volume.far.min(MAX_SHADOW_DISTANCE);
        let edges = split_distances(volume.near, far);
        let mut start = volume.near;
        let fits = edges.map(|end| {
            let corners = volume.slice_corners(camera, start, end);
            start = end;
            fit_cascade(light_direction, &corners, tile_size)
        });
        Self::from_fits(&fits, edges)
    }

    /// One fixed volume of [`SHADOW_VOLUME_HALF_EXTENT`] around the origin,
    /// used by every cascade: the single shadow map of before cascades.
    #[must_use]
    pub fn fixed(light_direction: Vec3, tile_size: u32) -> Self {
        let camera = directional_shadow_camera(light_direction, Vec3::ZERO);
        let extent = 2.0 * SHADOW_VOLUME_HALF_EXTENT;
        let fit = CascadeFit {
            view_proj: camera.view_projection_matrix(),
            texel_size: extent / texels(tile_size),
            depth_range: 2.0 * extent,
        };
        Self::from_fits(&[fit; SHADOW_CASCADES], [f32::MAX; SHADOW_CASCADES])
    }

    /// The volume each cascade renders, for culling casters against.
    #[must_use]
    pub fn frusta(&self) -> [Frustum; SHADOW_CASCADES] {
        self.view_proj.map(Frustum::from_view_projection)
    }

    fn from_fits(fits: &[CascadeFit; SHADOW_CASCADES], far_edges: [f32; SHADOW_CASCADES]) -> Self {
        Self {
            view_proj: fits.map(|fit| fit.view_proj),
            far_edges,
            bias_per_texel: fits.map(|fit| fit.texel_size / fit.depth_range),
        }
    }
}

/// View distances at which the cascades end: a blend of logarithmic and
/// uniform spacing between `near` and `far`, the last at `far`.
#[expect(clippy::cast_precision_loss, reason = "cascade indices are below 4")]
fn split_distances(near: f32, far: f32) -> [f32; SHADOW_CASCADES] {
    std::array::from_fn(|index| {
        let fraction = (index + 1) as f32 / SHADOW_CASCADES as f32;
        let logarithmic = near * (far / near).powf(fraction);
        let uniform = near + (far - near) * fraction;
        SPLIT_LAMBDA * logarithmic + (1.0 - SPLIT_LAMBDA) * uniform
    })
}
