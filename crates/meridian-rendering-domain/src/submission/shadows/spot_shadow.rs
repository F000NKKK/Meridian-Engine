//! The shadow map of a spot light (ADR 045).

use meridian_gac::{Motor3, Projection, Vec3};

use crate::scene::Light;
use crate::{Camera, Frustum, look_at_rotor};

/// The near plane of a spot light's map, in world units.
pub const SPOT_SHADOW_NEAR: f32 = 0.05;
/// The widest half-angle a spot cone (and so its map) is given, in radians:
/// about 80 degrees, which keeps the perspective projection finite.
pub const SPOT_MAX_OUTER_ANGLE: f32 = 1.4;

/// The cosines of a spot light's inner and outer half-angles, with the outer
/// one limited to [`SPOT_MAX_OUTER_ANGLE`] and the inner one to the outer.
#[must_use]
pub fn spot_cone(inner_angle: f32, outer_angle: f32) -> (f32, f32) {
    let outer = outer_angle.clamp(0.01, SPOT_MAX_OUTER_ANGLE);
    let inner = inner_angle.clamp(0.0, outer);
    (inner.cos(), outer.cos())
}

/// A spot light that casts a shadow in a view: where it is, where it points,
/// how far it reaches and its one perspective view, which spans the cone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpotShadow {
    /// World to clip space of the light's view.
    pub view_proj: [[f32; 4]; 4],
    /// The light's world position.
    pub position: Vec3,
    /// The light's range: what is farther is unshadowed.
    pub far: f32,
    /// Index of the light in the scene's lights.
    pub caster: u32,
}

impl SpotShadow {
    /// The shadow map of `light`, the `index`th of the scene's lights, when
    /// it is a spot light with room for one.
    pub(super) fn of_light(index: usize, light: &Light) -> Option<Self> {
        let Light::Spot {
            position,
            direction,
            range,
            outer_angle,
            ..
        } = light
        else {
            return None;
        };
        if *range <= SPOT_SHADOW_NEAR {
            return None;
        }
        let position = position.transform_point(Vec3::ZERO);
        let heading = direction.transform_vector(Vec3::X);
        let half_angle = outer_angle.clamp(0.01, SPOT_MAX_OUTER_ANGLE);
        let projection = Projection::perspective(2.0 * half_angle, 1.0, SPOT_SHADOW_NEAR, *range);
        let rotor = look_at_rotor(position, position + heading);
        let view_proj = Camera {
            frame: Motor3::from_rotation_translation(rotor, position),
            projection,
        }
        .view_projection_matrix();
        Some(Self {
            view_proj,
            position,
            far: *range,
            caster: u32::try_from(index).ok()?,
        })
    }

    /// The volume the map renders, for culling casters against.
    #[must_use]
    pub fn frustum(&self) -> Frustum {
        Frustum::from_view_projection(self.view_proj)
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::project;
    use super::super::{LocalShadow, SpotShadow};
    use super::*;
    use crate::Scene3d;

    fn spot(position: Vec3, direction: Motor3, outer_angle: f32) -> Light {
        Light::Spot {
            position: Motor3::translation(position),
            direction,
            color: [1.0; 3],
            intensity: 1.0,
            range: 30.0,
            inner_angle: 0.2,
            outer_angle,
        }
    }

    fn shadow_of(light: Light) -> SpotShadow {
        let scene = Scene3d {
            lights: vec![light],
            ..Scene3d::default()
        };
        match LocalShadow::nearest(&scene)[0] {
            Some(LocalShadow::Spot(spot)) => spot,
            other => panic!("expected a spot shadow, got {other:?}"),
        }
    }

    #[test]
    fn the_map_looks_along_the_light_direction_and_ends_at_the_cone_edge() {
        let at = Vec3::new(1.0, 2.0, 3.0);
        let shadow = shadow_of(spot(at, Motor3::identity(), 0.6));
        let [x, y, z] = project(shadow.view_proj, at + Vec3::X * 10.0);
        assert!(x.abs() < 1e-4 && y.abs() < 1e-4, "axis: ({x}, {y})");
        assert!((0.0..1.0).contains(&z), "depth {z}");
        let edge = at + Vec3::new(10.0, 0.0, 10.0 * 0.6f32.tan());
        let [rim, ..] = project(shadow.view_proj, edge);
        assert!((rim.abs() - 1.0).abs() < 1e-3, "the cone edge is at {rim}");
    }

    #[test]
    fn a_cone_wider_than_the_limit_is_clamped() {
        let (inner, outer) = spot_cone(3.0, 3.0);
        assert!((outer - SPOT_MAX_OUTER_ANGLE.cos()).abs() < 1e-6);
        assert!(inner >= outer, "the inner cone stays inside the outer one");
    }

    #[test]
    fn a_spot_and_a_point_light_share_the_nearest_slots() {
        let far_spot = spot(Vec3::new(9.0, 0.0, 0.0), Motor3::identity(), 0.5);
        let near_point = Light::Point {
            position: Motor3::translation(Vec3::new(2.0, 0.0, 0.0)),
            color: [1.0; 3],
            intensity: 1.0,
            range: 10.0,
        };
        let scene = Scene3d {
            lights: vec![far_spot, near_point],
            ..Scene3d::default()
        };
        let slots = LocalShadow::nearest(&scene);
        assert!(matches!(slots[0], Some(LocalShadow::Point(_))));
        assert!(matches!(slots[1], Some(LocalShadow::Spot(_))));
        assert_eq!(slots[1].map(|s| s.caster()), Some(0));
        assert!(slots[2].is_none());
    }
}
