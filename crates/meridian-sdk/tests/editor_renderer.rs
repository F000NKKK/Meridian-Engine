//! The editor renderer draws a world and thumbnails without a window. Skipped
//! where no GPU adapter exists.

use std::io::Write as _;

use meridian_ecs::{Transform, World};
use meridian_gac::{Motor3, Scale3, Vec3};
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

#[test]
fn a_material_file_is_previewed_and_a_damaged_one_is_refused() {
    use meridian_sdk::{BuiltinAssets, ThumbnailError};

    let Some(mut renderer) = renderer() else {
        return;
    };
    let mut binder = BuiltinAssets::default();
    let file = b"{ format: 1, base_color: [1, 0.77, 0.34, 1], metallic: 1, roughness: 0.3 }";
    let pixels = renderer
        .thumbnail_material_file(file, &mut binder, 64)
        .expect("a preview");
    assert!(varied(&pixels));
    let err = renderer
        .thumbnail_material_file(b"{ format: 1, sheen: 2 }", &mut binder, 64)
        .unwrap_err();
    assert!(matches!(err, ThumbnailError::Material(_)), "{err}");
}

/// A unit cube mesh and a default material registered with the renderer.
fn cube_assets(renderer: &mut EditorRenderer) -> (MeshHandle, MaterialHandle) {
    let session = renderer.session_mut();
    let mesh = session
        .render
        .meshes
        .register(cube_mesh_source(1.0))
        .expect("cube mesh is valid");
    let material = session.render.materials.register(Material::default());
    (mesh, material)
}

fn spawn_cube(
    world: &mut World,
    assets: (MeshHandle, MaterialHandle),
    at: [f32; 3],
) -> meridian_ecs::Entity {
    let entity = world.spawn();
    let motor = Motor3::translation(Vec3::new(at[0], at[1], at[2]));
    world.insert(entity, Transform { motor });
    world.insert(
        entity,
        MeshRenderer {
            mesh: assets.0,
            material: assets.1,
            billboard: false,
        },
    );
    entity
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

#[test]
fn pick_returns_the_nearest_entity_under_the_point() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let assets = cube_assets(&mut renderer);
    let mut world = World::new();
    let origin = spawn_cube(&mut world, assets, [0.0, 0.0, 0.0]);
    let far = spawn_cube(&mut world, assets, [0.0, 0.0, -4.0]);
    let side = spawn_cube(&mut world, assets, [6.0, 0.0, 0.0]);
    // From +z along -z the cube at the origin hides the one behind it.
    let centre = renderer.pick(&world, &looking([0.0; 3]), 0.0, 0.0);
    assert_eq!(centre, Some(origin));
    assert_ne!(centre, Some(far));
    let aside = renderer.pick(&world, &looking([6.0, 0.0, 0.0]), 0.0, 0.0);
    assert_eq!(aside, Some(side));
    assert_eq!(renderer.pick(&world, &looking([0.0; 3]), 0.95, 0.95), None);
    // Left and right of the screen are -x and +x.
    let between = looking([3.0, 0.0, 0.0]);
    let reach = 3.0 / (12.0 * (between.fov_y * 0.5).tan() * (96.0 / 64.0));
    assert_eq!(renderer.pick(&world, &between, -reach, 0.0), Some(origin));
    assert_eq!(renderer.pick(&world, &between, reach, 0.0), Some(side));
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
fn the_grid_and_a_selection_box_are_drawn_on_request() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let assets = cube_assets(&mut renderer);
    let mut world = World::new();
    let cube = spawn_cube(&mut world, assets, [0.0, 1.0, 0.0]);
    let camera = EditorCamera {
        target: [0.0, 0.5, 0.0],
        pitch: 0.5,
        distance: 8.0,
        ..EditorCamera::default()
    };
    let selected = [cube];
    let mut frame = |grid: bool, selection: &[meridian_ecs::Entity]| {
        let options = ViewOptions { grid, selection };
        renderer
            .render_with(&world, &camera, &options)
            .expect("a frame")
            .to_vec()
    };
    let plain = frame(false, &[]);
    assert_ne!(plain, frame(true, &[]), "the grid shows");
    let outlined = frame(false, &selected);
    assert_eq!(orange_pixels(&plain), 0);
    assert!(
        orange_pixels(&outlined) > 20,
        "the box shows in the selection colour"
    );
    // Drawing marks over and over registers nothing that stays.
    for _ in 0..20 {
        frame(true, &selected);
    }
}

#[test]
fn a_scale_widens_what_is_picked_drawn_and_outlined() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let assets = cube_assets(&mut renderer);
    let mut world = World::new();
    let wide = spawn_cube(&mut world, assets, [0.0; 3]);
    let camera = looking([0.0; 3]);
    // A unit cube reaches 0.5 either side; at x = 2 only the scaled one is hit.
    let reach = 2.0 / (12.0 * (camera.fov_y * 0.5).tan() * (96.0 / 64.0));
    assert_eq!(renderer.pick(&world, &camera, reach, 0.0), None);
    let before = renderer.render(&world, &camera).expect("a frame").to_vec();
    world.insert(wide, Scale3::new(6.0, 1.0, 1.0).unwrap());
    assert_eq!(renderer.pick(&world, &camera, reach, 0.0), Some(wide));
    let after = renderer.render(&world, &camera).expect("a frame").to_vec();
    assert_ne!(before, after, "the scaled cube fills more of the frame");
    let options = ViewOptions {
        grid: false,
        selection: &[wide],
    };
    let outlined = renderer
        .render_with(&world, &camera, &options)
        .expect("a frame")
        .to_vec();
    assert!(orange_pixels(&outlined) > 20, "the box follows the scale");
}

#[test]
fn a_child_is_picked_where_its_scaled_parent_puts_it() {
    let Some(mut renderer) = renderer() else {
        return;
    };
    let assets = cube_assets(&mut renderer);
    let mut world = World::new();
    let parent = world.spawn();
    world.insert(parent, Transform::default());
    world.insert(parent, Scale3::new(6.0, 1.0, 1.0).unwrap());
    // Locally the child's cube spans x = -0.2 to 0.8; the parent stretches it
    // to a cube 6 wide centred at 1.8, which reaches x = 2.
    let child = spawn_cube(&mut world, assets, [0.3, 0.0, 0.0]);
    world.insert(child, meridian_ecs::Parent(parent));
    let camera = looking([0.0; 3]);
    let reach = 2.0 / (12.0 * (camera.fov_y * 0.5).tan() * (96.0 / 64.0));
    assert_eq!(renderer.pick(&world, &camera, reach, 0.0), None);
    meridian_ecs::propagate_transforms(&mut world);
    assert_eq!(renderer.pick(&world, &camera, reach, 0.0), Some(child));
}
