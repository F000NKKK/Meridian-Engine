//! The weights a morphed renderable is drawn with.

use meridian_ecs::Component;

/// The weight of each morph target of the mesh of the entity's `MeshRenderer`
/// for this frame, in the order of its targets (see
/// [`Renderable3d::morph_weights`](crate::Renderable3d::morph_weights)).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MorphWeights(pub Vec<f32>);

impl Component for MorphWeights {}

#[cfg(test)]
mod tests {
    use meridian_ecs::{Transform, World};

    use super::*;
    use crate::scene::{MaterialHandle, MeshRenderer, extract_scene3d};
    use crate::{JointPalette, MeshHandle};

    #[test]
    fn extract_scene3d_carries_the_pose_components_of_an_entity() {
        let mut world = World::new();
        let posed = world.spawn();
        world.insert(posed, Transform::default());
        world.insert(
            posed,
            MeshRenderer {
                mesh: MeshHandle::new(meridian_memory::Handle::default()),
                material: MaterialHandle::new(meridian_memory::Handle::default()),
                billboard: false,
            },
        );
        world.insert(posed, JointPalette::new(vec![[[1.0; 4]; 4]]));
        world.insert(posed, MorphWeights(vec![0.5, 0.25]));
        let still = world.spawn();
        world.insert(still, Transform::default());
        world.insert(
            still,
            MeshRenderer {
                mesh: MeshHandle::new(meridian_memory::Handle::default()),
                material: MaterialHandle::new(meridian_memory::Handle::default()),
                billboard: false,
            },
        );

        let scene = extract_scene3d(&world, crate::Camera::default());
        let with_palette: Vec<_> = scene
            .renderables
            .iter()
            .filter(|renderable| renderable.palette.is_some())
            .collect();
        assert_eq!(with_palette.len(), 1);
        assert_eq!(with_palette[0].morph_weights, [0.5, 0.25]);
        assert!(
            scene
                .renderables
                .iter()
                .any(|renderable| renderable.palette.is_none()
                    && renderable.morph_weights.is_empty())
        );
    }
}
