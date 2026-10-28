//! References in a scene file are bound to handles, and models become pieces.
//! Skipped where no GPU adapter exists.

use std::io::Write as _;

use meridian_ecs::{Transform, World};
use meridian_sdk::{
    EditorCamera, EditorRenderer, Material, MaterialHandle, MeshHandle, MeshRenderer, ViewOptions,
    cube_mesh_source,
};

fn renderer() -> Option<EditorRenderer> {
    match EditorRenderer::new(96, 64) {
        Ok(renderer) => Some(renderer),
        Err(err) => {
            drop(writeln!(std::io::stderr(), "skipping: {err}"));
            None
        }
    }
}

fn varied(pixels: &[u8]) -> bool {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .any(|pixel| pixel[..3] != pixels[..3])
}

/// A world loaded from the scene `text`, with `transform` and the rendering
/// components registered.
fn world_of(text: &str) -> World {
    use meridian_data_substrate::{
        ComponentName, ComponentRegistry, NoResolver, TransformCodec, register_rendering,
    };

    let mut registry = ComponentRegistry::new();
    let name = ComponentName::new("transform").expect("a valid name");
    registry
        .register_with::<Transform, TransformCodec>(name)
        .expect("registers once");
    register_rendering(&mut registry).expect("registers once");
    let mut world = World::new();
    registry
        .load_scene(text, &mut world, &mut NoResolver)
        .expect("the scene loads");
    world
}

fn looking(target: [f32; 3]) -> EditorCamera {
    EditorCamera {
        target,
        yaw: 0.0,
        pitch: 0.0,
        distance: 12.0,
        ..EditorCamera::default()
    }
}

fn orange_pixels(pixels: &[u8]) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 200 && p[1] > 60 && p[1] < 170 && p[2] < 80)
        .count()
}

#[test]
fn a_scene_file_with_asset_references_is_bound_and_drawn() {
    use meridian_data_substrate::{ComponentRegistry, NoResolver, register_rendering};
    use meridian_sdk::BuiltinAssets;

    let Some(mut renderer) = renderer() else {
        return;
    };
    let mut registry = ComponentRegistry::new();
    registry
        .register_with::<Transform, meridian_data_substrate::TransformCodec>(
            meridian_data_substrate::ComponentName::new("transform").unwrap(),
        )
        .unwrap();
    register_rendering(&mut registry).unwrap();
    let text = r#"{ format: 1, entities: [
        { id: "ball", components: {
            transform: { motor: { rotor: [1, 0, 0, 0] } },
            mesh_renderer: { mesh: "@asset:builtin/sphere", material: "@asset:builtin/default" } } },
        { id: "lost", components: {
            transform: { motor: { rotor: [1, 0, 0, 0] } },
            mesh_renderer: { mesh: "@asset:meshes/nowhere.obj", material: "@asset:builtin/default" } } },
    ] }"#;
    let mut world = World::new();
    registry
        .load_scene(text, &mut world, &mut NoResolver)
        .expect("the scene loads");
    let unbound = renderer.bind_references(&mut world, &mut BuiltinAssets::default());
    assert_eq!(unbound.len(), 1);
    let ball = world
        .get::<MeshRenderer>(ball_entity(&world))
        .expect("bound");
    let colour = renderer
        .session_mut()
        .render
        .materials
        .get(ball.material)
        .expect("registered")
        .base_color_factor;
    assert!(
        colour[0] > 0.5,
        "the default material is a light grey, not black"
    );
    assert_eq!(unbound[0].missing.as_str(), "meshes/nowhere.obj");
    let pixels = renderer
        .render(&world, &EditorCamera::default())
        .expect("a frame");
    assert!(varied(pixels), "the bound sphere shows up");
}

fn ball_entity(world: &World) -> meridian_ecs::Entity {
    world
        .query::<MeshRenderer>()
        .map(|(entity, _)| entity)
        .next()
        .expect("one entity is bound")
}

/// A binder whose `models/pair.gltf` is two cubes side by side.
struct PairBinder;

impl meridian_sdk::AssetBinder for PairBinder {
    fn mesh(
        &mut self,
        _session: &mut meridian_sdk::GraphicsSession,
        _key: &meridian_data_substrate::AssetKey,
    ) -> Option<MeshHandle> {
        None
    }

    fn material(
        &mut self,
        _session: &mut meridian_sdk::GraphicsSession,
        _key: &meridian_data_substrate::AssetKey,
    ) -> Option<MaterialHandle> {
        None
    }

    fn model(
        &mut self,
        session: &mut meridian_sdk::GraphicsSession,
        key: &meridian_data_substrate::AssetKey,
    ) -> Option<meridian_sdk::assets::LoadedModel> {
        (key.as_str() == "models/pair.gltf").then(|| {
            let mesh = session
                .render
                .meshes
                .register(cube_mesh_source(1.0))
                .expect("cube mesh is valid");
            let piece = meridian_sdk::assets::ModelPiece {
                mesh,
                material: Material::default(),
                skeleton: None,
            };
            meridian_sdk::assets::LoadedModel {
                pieces: vec![piece, piece],
                rig: std::sync::Arc::default(),
            }
        })
    }
}

#[test]
fn a_reference_to_a_model_draws_its_pieces_and_a_click_means_the_owner() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let text = r#"{ format: 1, entities: [ { id: "pair", components: {
        transform: { motor: { rotor: [1, 0, 0, 0] } },
        mesh_renderer: { mesh: "@asset:models/pair.gltf", material: "@asset:builtin/default" } } } ] }"#;
    let mut world = world_of(text);
    assert_eq!(
        renderer.bind_references(&mut world, &mut PairBinder),
        Vec::new()
    );
    assert_eq!(
        renderer.bind_references(&mut world, &mut PairBinder),
        Vec::new()
    );
    let owner = world
        .query::<meridian_sdk::ModelPieces>()
        .map(|(entity, _)| entity)
        .next()
        .expect("the owner is marked");
    assert_eq!(world.query::<MeshRenderer>().count(), 2, "bound once");
    let camera = looking([0.0; 3]);
    assert_eq!(renderer.pick(&world, &camera, 0.0, 0.0), Some(owner));
    // The pieces inherit the owner's scale: a click 2 to the side hits only then.
    let reach = 2.0 / (12.0 * (camera.fov_y * 0.5).tan() * (96.0 / 64.0));
    assert_eq!(renderer.pick(&world, &camera, reach, 0.0), None);
    world.insert(owner, meridian_sdk::Scale3::new(6.0, 1.0, 1.0).unwrap());
    meridian_ecs::propagate_transforms(&mut world);
    assert_eq!(renderer.pick(&world, &camera, reach, 0.0), Some(owner));
    let options = ViewOptions {
        grid: false,
        selection: &[owner],
    };
    let outlined = renderer
        .render_with(&world, &camera, &options)
        .expect("a frame")
        .to_vec();
    assert!(orange_pixels(&outlined) > 20, "the pieces are boxed");
}

#[test]
fn a_gltf_file_in_a_folder_is_bound_to_its_owner() {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/assets");
    let Some(mut renderer) = renderer() else {
        return;
    };
    let text = r#"{ format: 1, entities: [ { id: "fox", components: {
        transform: { motor: { rotor: [1, 0, 0, 0] } },
        mesh_renderer: { mesh: "@asset:models/Fox.glb", material: "@asset:builtin/default" } } },
        { id: "gone", components: {
        transform: { motor: { rotor: [1, 0, 0, 0] } },
        mesh_renderer: { mesh: "@asset:models/none.glb", material: "@asset:builtin/default" } } } ] }"#;
    let mut world = world_of(text);
    let mut folder = meridian_sdk::GltfFolder::new(assets);
    let unbound = renderer.bind_references(&mut world, &mut folder);
    assert_eq!(unbound.len(), 1, "only the missing file stays unbound");
    assert_eq!(unbound[0].missing.as_str(), "models/none.glb");
    assert_eq!(world.query::<meridian_sdk::ModelPieces>().count(), 1);
    assert!(
        world.query::<MeshRenderer>().count() >= 1,
        "the fox has pieces"
    );
}
