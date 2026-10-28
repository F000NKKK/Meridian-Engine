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
    assert_eq!(unbound[0].missing.as_str(), "meshes/nowhere.obj");
    let pixels = renderer
        .render(&world, &EditorCamera::default())
        .expect("a frame");
    assert!(varied(pixels), "the bound sphere shows up");
}
