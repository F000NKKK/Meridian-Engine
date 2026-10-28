//! The shadow of a point or spot light, which share the atlas slots (ADR 045).

use meridian_gac::Vec3;

use super::{MAX_LOCAL_SHADOWS, PointShadow, SpotShadow};
use crate::scene::Scene3d;

/// A shadow cast by a light that sits in the scene: a cube around a point
/// light (ADR 041) or one view of a spot light (ADR 045). Both take one of the
/// view's [`MAX_LOCAL_SHADOWS`] slots, nearest light first.
#[derive(Debug, Clone, Copy, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "plain data copied into a fixed array of slots; boxing the cube would drop `Copy`"
)]
pub enum LocalShadow {
    /// The six faces of a point light.
    Point(PointShadow),
    /// The single view of a spot light.
    Spot(SpotShadow),
}

impl LocalShadow {
    /// The point and spot lights of `scene` nearest to its camera, up to
    /// [`MAX_LOCAL_SHADOWS`], nearest first, then `None`s. A light whose range
    /// leaves no room between the near plane and the far one is not
    /// considered.
    #[must_use]
    pub fn nearest(scene: &Scene3d) -> [Option<Self>; MAX_LOCAL_SHADOWS] {
        let eye = scene.camera.frame.transform_point(Vec3::ZERO);
        let mut lights: Vec<Self> = scene
            .lights
            .iter()
            .enumerate()
            .filter_map(|(index, light)| {
                PointShadow::of_light(index, light)
                    .map(Self::Point)
                    .or_else(|| SpotShadow::of_light(index, light).map(Self::Spot))
            })
            .collect();
        lights.sort_by(|a, b| {
            let (da, db) = (
                (a.position() - eye).length_squared(),
                (b.position() - eye).length_squared(),
            );
            da.total_cmp(&db)
        });
        let mut nearest = lights.into_iter();
        core::array::from_fn(|_| nearest.next())
    }

    /// The light's world position.
    #[must_use]
    pub fn position(&self) -> Vec3 {
        match self {
            Self::Point(point) => point.position,
            Self::Spot(spot) => spot.position,
        }
    }

    /// The light's range: what is farther is unshadowed.
    #[must_use]
    pub fn far(&self) -> f32 {
        match self {
            Self::Point(point) => point.far,
            Self::Spot(spot) => spot.far,
        }
    }

    /// Index of the light in the scene's lights.
    #[must_use]
    pub fn caster(&self) -> u32 {
        match self {
            Self::Point(point) => point.caster,
            Self::Spot(spot) => spot.caster,
        }
    }
}
