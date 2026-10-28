//! The editor renderer draws a world and thumbnails without a window. Skipped
//! where no GPU adapter exists.

use std::io::Write as _;

use meridian_ecs::{Transform, World};
use meridian_sdk::{EditorCamera, EditorRenderer, Material, MeshRenderer, cube_mesh_source};

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

#[test]
fn a_world_with_a_cube_renders_a_framed_picture() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let session = renderer.session_mut();
    let mesh = session
        .render
        .meshes
        .register(cube_mesh_source(1.0))
        .expect("cube mesh is valid");
    let material = session.render.materials.register(Material {
        base_color_factor: [0.8, 0.2, 0.2, 1.0],
        ..Material::default()
    });
    let mut world = World::new();
    let cube = world.spawn();
    world.insert(cube, Transform::default());
    world.insert(
        cube,
        MeshRenderer {
            mesh,
            material,
            billboard: false,
        },
    );
    let pixels = renderer
        .render(&world, &EditorCamera::default())
        .expect("the frame draws");
    assert_eq!(pixels.len(), 96 * 64 * 4);
    assert!(varied(pixels), "the cube must show against the sky");
}

#[test]
fn a_material_thumbnail_is_square_and_restores_the_viewport_size() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let pixels = renderer
        .thumbnail_material(Material::default(), 32)
        .expect("the sphere draws");
    assert_eq!(pixels.len(), 32 * 32 * 4);
    assert!(varied(&pixels));
    assert_eq!(renderer.size(), (96, 64));
}

#[test]
fn an_empty_scene_has_no_thumbnail() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    assert!(
        renderer
            .thumbnail_scene(meridian_sdk::Scene3d::default(), 16)
            .is_none()
    );
}

#[test]
fn a_thumbnail_subject_is_drawn_or_refused_with_a_reason() {
    use meridian_sdk::{ThumbnailError, ThumbnailSubject};

    let Some(mut renderer) = renderer() else {
        return;
    };
    let pixels = renderer
        .thumbnail(ThumbnailSubject::Color([0.1, 0.5, 0.9, 1.0]), 24)
        .expect("a colour sphere draws");
    assert_eq!(pixels.len(), 24 * 24 * 4);
    let missing = renderer.thumbnail(ThumbnailSubject::Model("no/such/model.glb".into()), 24);
    assert!(matches!(missing, Err(ThumbnailError::Load(_))));
    let empty = renderer.thumbnail(ThumbnailSubject::Scene(Box::default()), 24);
    assert!(matches!(empty, Err(ThumbnailError::Empty)));
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

#[test]
fn a_metal_thumbnail_has_a_dark_backdrop_and_a_bright_reflection() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let gold = Material {
        base_color_factor: [1.0, 0.77, 0.34, 1.0],
        metallic: 1.0,
        roughness: 0.25,
        ..Material::default()
    };
    let pixels = renderer
        .thumbnail(meridian_sdk::ThumbnailSubject::Material(gold), 96)
        .expect("a thumbnail");
    let luma = |at: usize| u32::from(pixels[at * 4]) + u32::from(pixels[at * 4 + 1]);
    assert!(luma(0) < 150, "the backdrop is dark");
    let brightest = (0..96 * 96).map(luma).max().unwrap_or(0);
    assert!(brightest > 350, "the key light shows in the metal");
    // The viewport sky comes back after a thumbnail.
    let world = World::new();
    assert!(renderer.render(&world, &EditorCamera::default()).is_some());
}
