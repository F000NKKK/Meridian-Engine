//! Which entity is under a point of the viewport.

use meridian_ecs::{Entity, GlobalTransform, Parent, Transform, World};
use meridian_gac::{Motor3, Scale3, Vec3};
use meridian_rendering_domain::{MeshRenderer, MeshSource};

use super::{EditorCamera, EditorRenderer, ModelPieces};

/// A ray: where it starts and the unit direction it points.
#[derive(Debug, Clone, Copy)]
pub(super) struct Ray {
    pub(super) origin: Vec3,
    pub(super) direction: Vec3,
}

impl EditorCamera {
    /// The ray through the viewport point `(ndc_x, ndc_y)` (each in `[-1, 1]`,
    /// `+y` up) of a frame of `aspect` (width over height).
    pub(super) fn ray(&self, aspect: f32, ndc_x: f32, ndc_y: f32) -> Ray {
        let frame = self.camera(aspect).frame;
        let half_height = (self.fov_y * 0.5).tan();
        // A camera frame looks along local +X, with +Y up and +Z to the right.
        let local = Vec3::new(1.0, ndc_y * half_height, ndc_x * half_height * aspect);
        Ray {
            origin: self.eye(),
            direction: frame.transform_vector(local).normalize(),
        }
    }
}

impl EditorRenderer {
    /// The entity drawn at the viewport point `(ndc_x, ndc_y)` (each in
    /// `[-1, 1]`, `+y` up, the point's place in the frame `render` draws), the
    /// nearest one when several overlap; `None` over empty space. Tests the ray
    /// against the triangles of each entity's mesh at its current frame.
    #[must_use]
    pub fn pick(
        &self,
        world: &World,
        camera: &EditorCamera,
        ndc_x: f32,
        ndc_y: f32,
    ) -> Option<Entity> {
        let ray = camera.ray(self.aspect(), ndc_x, ndc_y);
        let meshes = &self.session.render.meshes;
        world
            .query::<MeshRenderer>()
            .filter_map(|(entity, renderer)| {
                let source = meshes.get(renderer.mesh)?;
                let placed = Placed {
                    frame: frame_of(world, entity)?,
                    scale: scale_of(world, entity),
                };
                let distance = hit_distance(source, placed, ray)?;
                Some((entity, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(entity, _)| owner_of(world, entity))
    }
}

/// The entity a click on `piece` means: the entity whose reference named a
/// model when `piece` is one of its pieces, else `piece` itself.
fn owner_of(world: &World, piece: Entity) -> Entity {
    world
        .get::<Parent>(piece)
        .map(|parent| parent.0)
        .filter(|owner| world.get::<ModelPieces>(*owner).is_some())
        .unwrap_or(piece)
}

/// An entity's world frame, as the renderer draws it.
pub(super) fn frame_of(world: &World, entity: Entity) -> Option<Motor3> {
    let local = world.get::<Transform>(entity)?;
    Some(
        world
            .get::<GlobalTransform>(entity)
            .map_or(local.motor, |global| global.motor),
    )
}

/// An entity's own scale, the unit one when it has none.
pub(super) fn scale_of(world: &World, entity: Entity) -> Scale3 {
    world.get::<Scale3>(entity).copied().unwrap_or_default()
}

/// Where a mesh stands in the world: scaled along its axes, then moved.
#[derive(Clone, Copy)]
struct Placed {
    frame: Motor3,
    scale: Scale3,
}

/// How far along `ray` it meets the nearest triangle of `source` placed as
/// `placed`, if it does. The distance is in units of the ray's direction,
/// which the mapping into mesh space keeps comparable between entities.
fn hit_distance(source: &MeshSource, placed: Placed, ray: Ray) -> Option<f32> {
    let inverse = placed.frame.inverse();
    let local = Ray {
        origin: placed.scale.unapply(inverse.transform_point(ray.origin)),
        direction: placed
            .scale
            .unapply(inverse.transform_vector(ray.direction)),
    };
    let point = |index: u32| {
        let [x, y, z] = *source.positions.get(index as usize)?;
        Some(Vec3::new(x, y, z))
    };
    source
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .filter_map(|tri| triangle_hit(local, point(tri[0])?, point(tri[1])?, point(tri[2])?))
        .min_by(f32::total_cmp)
}

/// Möller–Trumbore, both faces.
fn triangle_hit(ray: Ray, first: Vec3, second: Vec3, third: Vec3) -> Option<f32> {
    let (edge1, edge2) = (second - first, third - first);
    let pivot = ray.direction.cross(edge2);
    let det = edge1.dot(pivot);
    if det.abs() < 1e-9 {
        return None;
    }
    let inverse = 1.0 / det;
    let to_origin = ray.origin - first;
    let along1 = to_origin.dot(pivot) * inverse;
    let offset = to_origin.cross(edge1);
    let along2 = ray.direction.dot(offset) * inverse;
    let distance = edge2.dot(offset) * inverse;
    (along1 >= 0.0 && along2 >= 0.0 && along1 + along2 <= 1.0 && distance > 0.0).then_some(distance)
}
