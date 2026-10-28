//! The cube of shadow maps around a point light (ADR 041).

use meridian_gac::{Motor3, Projection, Vec3};

use crate::scene::{Light, Scene3d};
use crate::{Camera, Frustum, look_at_rotor};

/// How many faces surround a point light.
pub const POINT_SHADOW_FACES: usize = 6;
/// The near plane of every face, in world units.
pub const POINT_SHADOW_NEAR: f32 = 0.05;
/// The face directions, matching the order the shader picks them in:
/// `+x`, `-x`, `+y`, `-y`, `+z`, `-z`.
const FACE_DIRECTIONS: [Vec3; POINT_SHADOW_FACES] = [
    Vec3::X,
    Vec3::new(-1.0, 0.0, 0.0),
    Vec3::Y,
    Vec3::new(0.0, -1.0, 0.0),
    Vec3::Z,
    Vec3::new(0.0, 0.0, -1.0),
];

/// The point light that casts a shadow in a view: where it is, how far it
/// reaches and the six perspective views of its shadow cube.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointShadow {
    /// World to clip space of each face, in the order of [`POINT_SHADOW_FACES`].
    pub view_proj: [[[f32; 4]; 4]; POINT_SHADOW_FACES],
    /// The light's world position.
    pub position: Vec3,
    /// The light's range: what is farther is unshadowed.
    pub far: f32,
    /// Index of the light in the scene's lights.
    pub caster: u32,
}

impl PointShadow {
    /// The scene's first point light, `None` when it has none or its range
    /// leaves no room between the near plane and the far one.
    #[must_use]
    pub fn for_scene(scene: &Scene3d) -> Option<Self> {
        let (index, light) = scene
            .lights
            .iter()
            .enumerate()
            .find(|(_, light)| matches!(light, Light::Point { .. }))?;
        let Light::Point {
            position, range, ..
        } = light
        else {
            return None;
        };
        if *range <= POINT_SHADOW_NEAR {
            return None;
        }
        let position = position.transform_point(Vec3::ZERO);
        let projection =
            Projection::perspective(core::f32::consts::FRAC_PI_2, 1.0, POINT_SHADOW_NEAR, *range);
        let view_proj = FACE_DIRECTIONS.map(|direction| {
            let rotor = look_at_rotor(position, position + direction);
            Camera {
                frame: Motor3::from_rotation_translation(rotor, position),
                projection,
            }
            .view_projection_matrix()
        });
        Some(Self {
            view_proj,
            position,
            far: *range,
            caster: u32::try_from(index).ok()?,
        })
    }

    /// The volume each face renders, for culling casters against.
    #[must_use]
    pub fn frusta(&self) -> [Frustum; POINT_SHADOW_FACES] {
        self.view_proj.map(Frustum::from_view_projection)
    }
}
