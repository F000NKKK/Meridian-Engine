//! Extraction of backgrounds into sprites.
use meridian_memory::Handle;
use meridian_rendering_domain::TextureHandle;
use meridian_ui_domain::layout::value::{Length, Visibility};
use meridian_ui_domain::properties::element::{BACKGROUND, HEIGHT, MARGIN, VISIBILITY, WIDTH};
use meridian_ui_domain::{
    Color, ElementKind, FixedAdvanceText, NodeId, Size, Thickness, UiError, UiTree, compute_layout,
};
use meridian_ui_scene2d::{UiPaint, extract_scene};

fn paint() -> UiPaint {
    UiPaint {
        fill: TextureHandle::new(Handle {
            index: 7,
            generation: 0,
        }),
    }
}

fn solid(tree: &mut UiTree, parent: Option<NodeId>, size: (f32, f32), color: Color) -> NodeId {
    solid_of(tree, ElementKind::Border, parent, size, color)
}

fn solid_of(
    tree: &mut UiTree,
    kind: ElementKind,
    parent: Option<NodeId>,
    size: (f32, f32),
    color: Color,
) -> NodeId {
    let node = tree.create(kind);
    tree.set(node, WIDTH, Length::Px(size.0))
        .expect("test setup");
    tree.set(node, HEIGHT, Length::Px(size.1))
        .expect("test setup");
    tree.set(node, BACKGROUND, color).expect("test setup");
    if let Some(parent) = parent {
        tree.append_child(parent, node).expect("test setup");
    }
    node
}

fn extract(tree: &mut UiTree, root: NodeId) -> meridian_rendering_domain::Scene2d {
    let surface = Size::new(200.0, 100.0);
    compute_layout(tree, root, surface, &FixedAdvanceText::default()).expect("test setup");
    extract_scene(tree, root, surface, paint()).expect("test setup")
}

#[test]
fn camera_is_the_surface_in_pixels_with_y_down() {
    let mut tree = UiTree::new();
    let root = solid(&mut tree, None, (10.0, 10.0), Color::WHITE);
    let scene = extract(&mut tree, root);
    assert_eq!(scene.camera.extents, [0.0, 200.0, 100.0, 0.0]);
}

#[test]
fn a_background_becomes_a_tinted_sprite_of_the_fill_texture() {
    let mut tree = UiTree::new();
    let red = Color::new(1.0, 0.0, 0.0, 1.0);
    let root = solid(&mut tree, None, (40.0, 20.0), red);
    let scene = extract(&mut tree, root);
    assert_eq!(scene.sprites.len(), 1);
    let sprite = &scene.sprites[0];
    assert_eq!(sprite.size, [40.0, 20.0]);
    assert_eq!(sprite.tint, red.to_array());
    assert_eq!(sprite.texture, paint().fill);
    assert_eq!(sprite.source_rect, None);
}

#[test]
fn transparent_and_empty_elements_draw_nothing() {
    let mut tree = UiTree::new();
    let root = solid(&mut tree, None, (40.0, 20.0), Color::TRANSPARENT);
    solid(&mut tree, Some(root), (0.0, 20.0), Color::WHITE);
    assert!(extract(&mut tree, root).sprites.is_empty());
}

#[test]
fn children_draw_over_parents_and_later_siblings_over_earlier_ones() {
    let mut tree = UiTree::new();
    let root = solid_of(
        &mut tree,
        ElementKind::Grid,
        None,
        (100.0, 100.0),
        Color::BLACK,
    );
    let first = solid(&mut tree, Some(root), (10.0, 10.0), Color::WHITE);
    solid(&mut tree, Some(first), (5.0, 5.0), Color::WHITE);
    solid(&mut tree, Some(root), (10.0, 10.0), Color::WHITE);
    let scene = extract(&mut tree, root);
    let layers: Vec<i32> = scene.sprites.iter().map(|sprite| sprite.layer).collect();
    assert_eq!(layers, vec![0, 1, 2, 3]);
    let sizes: Vec<[f32; 2]> = scene.sprites.iter().map(|sprite| sprite.size).collect();
    assert_eq!(
        sizes,
        vec![[100.0, 100.0], [10.0, 10.0], [5.0, 5.0], [10.0, 10.0]]
    );
}

#[test]
fn hidden_and_collapsed_elements_hide_their_subtree() {
    let mut tree = UiTree::new();
    let root = solid_of(
        &mut tree,
        ElementKind::Grid,
        None,
        (100.0, 100.0),
        Color::BLACK,
    );
    let hidden = solid(&mut tree, Some(root), (10.0, 10.0), Color::WHITE);
    solid(&mut tree, Some(hidden), (5.0, 5.0), Color::WHITE);
    let collapsed = solid(&mut tree, Some(root), (10.0, 10.0), Color::WHITE);
    tree.set(hidden, VISIBILITY, Visibility::Hidden).unwrap();
    tree.set(collapsed, VISIBILITY, Visibility::Collapsed)
        .unwrap();
    assert_eq!(extract(&mut tree, root).sprites.len(), 1);
}

#[test]
fn sprite_frames_are_quad_centers_in_surface_pixels() {
    let mut tree = UiTree::new();
    let root = solid(&mut tree, None, (100.0, 100.0), Color::BLACK);
    let child = solid(&mut tree, Some(root), (20.0, 10.0), Color::WHITE);
    tree.set(child, MARGIN, Thickness::new(30.0, 40.0, 0.0, 0.0))
        .unwrap();
    let scene = extract(&mut tree, root);
    let center = scene.sprites[1]
        .frame
        .transform_point(meridian_gac::Vec3::new(0.0, 0.0, 0.0));
    assert_eq!((center.x, center.y, center.z), (40.0, 45.0, 0.0));
}

#[test]
fn a_stale_root_is_an_error() {
    let mut tree = UiTree::new();
    let root = tree.create(ElementKind::Border);
    tree.remove(root).unwrap();
    let result = extract_scene(&tree, root, Size::new(10.0, 10.0), paint());
    assert!(matches!(result, Err(UiError::StaleNode(_))));
}
