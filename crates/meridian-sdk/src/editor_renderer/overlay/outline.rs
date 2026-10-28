//! The box drawn around a selected entity.

use meridian_ecs::{Entity, World};
use meridian_gac::Vec3;
use meridian_rendering_domain::{MeshRegistry, MeshRenderer};

use super::grid::LineStyle;
use super::ribbon::Line;
use crate::editor_renderer::picking::{frame_of, scale_of};

const SELECTED: LineStyle = LineStyle {
    color: [1.0, 0.25, 0.0],
    on_top: true,
};

/// The twelve edges of the box around each of `selection`'s meshes, at their
/// current frames.
pub(super) fn selection_lines(
    world: &World,
    meshes: &MeshRegistry,
    selection: &[Entity],
) -> Option<(LineStyle, Vec<Line>)> {
    let lines: Vec<Line> = selection
        .iter()
        .filter_map(|&entity| entity_box(world, meshes, entity))
        .flatten()
        .collect();
    (!lines.is_empty()).then_some((SELECTED, lines))
}

fn entity_box(world: &World, meshes: &MeshRegistry, entity: Entity) -> Option<Vec<Line>> {
    let renderer = world.get::<MeshRenderer>(entity)?;
    let bounds = meshes.get(renderer.mesh)?.bounds()?;
    let frame = frame_of(world, entity)?;
    let scale = scale_of(world, entity);
    let corner = |bits: u8| {
        let pick = |bit: u8, low: f32, high: f32| if bits & bit == 0 { low } else { high };
        frame.transform_point(scale.apply(Vec3::new(
            pick(1, bounds.min.x, bounds.max.x),
            pick(2, bounds.min.y, bounds.max.y),
            pick(4, bounds.min.z, bounds.max.z),
        )))
    };
    let mut edges = Vec::with_capacity(12);
    for from in 0..8_u8 {
        for bit in [1_u8, 2, 4] {
            if from & bit == 0 {
                edges.push(Line {
                    from: corner(from),
                    to: corner(from | bit),
                    width: 2.0,
                });
            }
        }
    }
    Some(edges)
}
