//! An entity described by the DSL scene.

use meridian_examples::scene_loader::load_dsl_scene;
use meridian_sdk::dsl::dsl_core::BuiltNode;
use meridian_sdk::{
    ColliderShape, Vec3, cube_mesh_source, dsl, ground_mesh_source, icosphere_mesh_source,
    pyramid_mesh_source,
};

/// One `<Entity>` from `assets/scenes/physic_figures.mel`, flattened
/// out of its typed `<Transform>`/`<Mesh>`/`<Material>`/`<RigidBody>`
/// children into the plain fields this example actually needs — the
/// DSL tree itself is only walked once, here, in [`load_scene`].
pub(crate) struct SceneEntity {
    pub(crate) position: Vec3,
    pub(crate) mesh_shape: String,
    size: f32,
    size2: f32,
    pub(crate) texture: String,
    pub(crate) mass: f32,
    pub(crate) collider: ColliderShape,
}

/// Parses `assets/scenes/physic_figures.mel` against
/// `meridian_sdk::dsl::default_registry()` (this scene only uses
/// built-in tags — no custom `#[dsl_tag]` needed here, unlike
/// `magic_figures`) and flattens every `<Entity>` into a [`SceneEntity`].
/// The read/parse/error-logging sequence itself is
/// `meridian_examples::scene_loader::load_dsl_scene` — shared with
/// every other example, since it's identical regardless of which tags
/// a scene uses.
pub(crate) fn load_scene() -> Vec<SceneEntity> {
    let registry = dsl::default_registry();
    let root = load_dsl_scene("assets/scenes/physic_figures.mel", &registry);

    root.children.iter().map(entity_from_node).collect()
}

/// Flattens one `<Entity>` node into a [`SceneEntity`].
fn entity_from_node(entity_node: &BuiltNode) -> SceneEntity {
    let entity = entity_node
        .downcast_ref::<dsl::Entity>()
        .unwrap_or_else(|| panic!("<{}> at scene root must be <Entity>", entity_node.tag));

    let mut position = Vec3::ZERO;
    let mut mesh_shape = String::new();
    let mut size = 0.0f32;
    let mut size2 = 0.0f32;
    let mut texture = String::new();
    let mut mass = 0.0f32;
    let mut collider = ColliderShape::Sphere { radius: 0.0 };

    for child in &entity_node.children {
        if let Some(t) = child.downcast_ref::<dsl::Transform>() {
            position = Vec3::new(t.x, t.y, t.z);
        } else if let Some(m) = child.downcast_ref::<dsl::Mesh>() {
            mesh_shape = m
                .shape
                .clone()
                .unwrap_or_else(|| panic!("{}: <Mesh> needs a 'shape'", entity.name));
            size = m
                .size
                .unwrap_or_else(|| panic!("{}: <Mesh> needs a 'size'", entity.name));
            size2 = m.size2.unwrap_or(0.0);
        } else if let Some(mat) = child.downcast_ref::<dsl::Material>() {
            texture = mat
                .texture
                .clone()
                .unwrap_or_else(|| panic!("{}: <Material> needs a 'texture'", entity.name));
        } else if let Some(rb) = child.downcast_ref::<dsl::RigidBody>() {
            mass = rb.mass;
            collider = collider_of(rb, &entity.name);
        }
    }

    SceneEntity {
        position,
        mesh_shape,
        size,
        size2,
        texture,
        mass,
        collider,
    }
}

/// The collider a `<RigidBody>` describes.
fn collider_of(rb: &dsl::RigidBody, entity_name: &str) -> ColliderShape {
    match rb.shape.as_str() {
        "sphere" => ColliderShape::Sphere {
            radius: rb
                .radius
                .unwrap_or_else(|| panic!("{entity_name}: sphere <RigidBody> needs 'radius'")),
        },
        "cuboid" => ColliderShape::Cuboid {
            half_extents: Vec3::new(
                rb.hx
                    .unwrap_or_else(|| panic!("{entity_name}: cuboid <RigidBody> needs 'hx'")),
                rb.hy
                    .unwrap_or_else(|| panic!("{entity_name}: cuboid <RigidBody> needs 'hy'")),
                rb.hz
                    .unwrap_or_else(|| panic!("{entity_name}: cuboid <RigidBody> needs 'hz'")),
            ),
        },
        other => panic!("{entity_name}: unknown RigidBody shape '{other}'"),
    }
}

/// Builds the real `MeshSource` for one [`SceneEntity`], dispatching on
/// its `mesh_shape`/`size`/`size2` — see `dsl::Mesh`'s own doc comment
/// for what each shape's `size`/`size2` mean.
pub(crate) fn mesh_source_for(entity: &SceneEntity) -> meridian_sdk::MeshSource {
    match entity.mesh_shape.as_str() {
        "ground" => ground_mesh_source(entity.size, entity.size2),
        "sphere" => icosphere_mesh_source(2, entity.size),
        "cube" => cube_mesh_source(entity.size),
        "pyramid" => pyramid_mesh_source(entity.size, entity.size2),
        other => panic!("unknown mesh shape '{other}'"),
    }
}
