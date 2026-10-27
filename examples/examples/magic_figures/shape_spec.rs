//! A shape described by the DSL scene.

use crate::Glow;
use crate::OrbitTag;
use meridian_examples::scene_loader::load_dsl_scene;
use meridian_sdk::dsl::dsl_core::BuiltNode;
use meridian_sdk::dsl::dsl_tag;
use meridian_sdk::{cube_mesh_source, dsl, icosphere_mesh_source, pyramid_mesh_source};

/// This example's own "which file plays for this shape" tag.
#[dsl_tag(name = "Audio")]
#[derive(Debug, Clone, PartialEq)]
struct AudioTag {
    file: String,
}

/// One shape's fixed identity, flattened out of `assets/scenes/magic_figures.mel`
/// by [`load_scene`] — the DSL tree itself is only walked once, at
/// startup.
pub(crate) struct ShapeSpec {
    pub(crate) name: String,
    pub(crate) audio_file: String,
    pub(crate) texture_file: String,
    mesh_shape: String,
    size: f32,
    size2: f32,
    pub(crate) glow_color: [f32; 3],
    pub(crate) phase: f32,
    /// Radians per second around its own local axis — visually
    /// distinguishes "orbiting" from "spinning in place".
    pub(crate) spin_speed: f32,
}

/// Parses `assets/scenes/magic_figures.mel` against the SDK's built-in
/// tags plus this example's own [`Glow`](crate::Glow)/[`OrbitTag`](crate::OrbitTag)/[`AudioTag`] and
/// flattens every `<Entity>` into a [`ShapeSpec`]. The read/parse/
/// error-logging sequence itself is
/// `meridian_examples::scene_loader::load_dsl_scene` — see
/// `physic_figures::load_scene`'s identical use of it.
pub(crate) fn load_scene() -> Vec<ShapeSpec> {
    let mut registry = dsl::default_registry();
    registry.register::<Glow>();
    registry.register::<OrbitTag>();
    registry.register::<AudioTag>();

    let root = load_dsl_scene("assets/scenes/magic_figures.mel", &registry);

    root.children.iter().map(shape_from_node).collect()
}

/// Flattens one `<Entity>` node into a [`ShapeSpec`].
fn shape_from_node(entity_node: &BuiltNode) -> ShapeSpec {
    let entity = entity_node
        .downcast_ref::<dsl::Entity>()
        .unwrap_or_else(|| panic!("<{}> at scene root must be <Entity>", entity_node.tag));

    let mut mesh_shape = String::new();
    let mut size = 0.0f32;
    let mut size2 = 0.0f32;
    let mut texture_file = String::new();
    let mut glow_color = [1.0f32, 1.0, 1.0];
    let mut phase = 0.0f32;
    let mut spin_speed = 0.0f32;
    let mut audio_file = String::new();

    for child in &entity_node.children {
        if let Some(m) = child.downcast_ref::<dsl::Mesh>() {
            mesh_shape = m
                .shape
                .clone()
                .unwrap_or_else(|| panic!("{}: <Mesh> needs a 'shape'", entity.name));
            size = m
                .size
                .unwrap_or_else(|| panic!("{}: <Mesh> needs a 'size'", entity.name));
            size2 = m.size2.unwrap_or(0.0);
        } else if let Some(mat) = child.downcast_ref::<dsl::Material>() {
            texture_file = mat
                .texture
                .clone()
                .unwrap_or_else(|| panic!("{}: <Material> needs a 'texture'", entity.name));
        } else if let Some(g) = child.downcast_ref::<Glow>() {
            glow_color = [g.r, g.g, g.b];
        } else if let Some(o) = child.downcast_ref::<OrbitTag>() {
            phase = o.phase;
            spin_speed = o.spin_speed;
        } else if let Some(a) = child.downcast_ref::<AudioTag>() {
            audio_file = a.file.clone();
        }
    }

    ShapeSpec {
        name: entity.name.clone(),
        audio_file,
        texture_file,
        mesh_shape,
        size,
        size2,
        glow_color,
        phase,
        spin_speed,
    }
}

/// Builds the real `MeshSource` for one [`ShapeSpec`] — see
/// `dsl::Mesh`'s own doc for what `size`/`size2` mean per shape.
pub(crate) fn mesh_source_for(shape: &ShapeSpec) -> meridian_sdk::MeshSource {
    match shape.mesh_shape.as_str() {
        "cube" => cube_mesh_source(shape.size),
        "sphere" => icosphere_mesh_source(2, shape.size),
        "pyramid" => pyramid_mesh_source(shape.size, shape.size2),
        other => panic!("{}: unknown mesh shape '{other}'", shape.name),
    }
}
