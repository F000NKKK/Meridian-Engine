//! Rasterizing outlines into coverage bitmaps.

use meridian_text_domain::{Contour, FontError, Outline, Point, Segment, rasterize};

fn polygon(points: &[(f32, f32)]) -> Contour {
    let at = |&(x, y): &(f32, f32)| Point::new(x, y);
    Contour {
        start: at(&points[0]),
        segments: points[1..]
            .iter()
            .map(|p| Segment::Line { to: at(p) })
            .collect(),
    }
}

fn outline(contours: Vec<Contour>) -> Outline {
    Outline { contours }
}

fn square(side: f32) -> Outline {
    outline(vec![polygon(&[
        (0.0, 0.0),
        (side, 0.0),
        (side, side),
        (0.0, side),
    ])])
}

/// Total coverage in pixels, i.e. the area the bitmap fills.
fn area(coverage: &[u8]) -> f32 {
    coverage.iter().map(|&c| f32::from(c)).sum::<f32>() / 255.0
}

#[test]
fn an_empty_outline_gives_an_empty_bitmap() {
    let bitmap = rasterize(&Outline::default(), 1.0, Point::new(0.0, 0.0)).expect("raster");
    assert_eq!((bitmap.width, bitmap.height), (0, 0));
    assert!(bitmap.coverage.is_empty());
}

#[test]
fn a_pixel_aligned_square_is_fully_covered() {
    let bitmap = rasterize(&square(10.0), 1.0, Point::new(0.0, 0.0)).expect("raster");
    assert_eq!((bitmap.width, bitmap.height), (10, 10));
    assert_eq!((bitmap.left, bitmap.top), (0, 10));
    assert!(bitmap.coverage.iter().all(|&c| c == 255));
}

#[test]
fn scale_multiplies_the_size() {
    let bitmap = rasterize(&square(10.0), 0.5, Point::new(0.0, 0.0)).expect("raster");
    assert_eq!((bitmap.width, bitmap.height), (5, 5));
    assert!(bitmap.coverage.iter().all(|&c| c == 255));
}

#[test]
fn a_half_pixel_shift_splits_the_edge_columns() {
    let bitmap = rasterize(&square(10.0), 1.0, Point::new(0.5, 0.0)).expect("raster");
    assert_eq!(bitmap.width, 11);
    let first_row = &bitmap.coverage[..11];
    assert_eq!(first_row[0], 128);
    assert!(first_row[1..10].iter().all(|&c| c == 255));
    assert_eq!(first_row[10], 128);
    assert!((area(&bitmap.coverage) - 100.0).abs() < 0.5);
}

#[test]
fn a_triangle_covers_half_its_bounding_box() {
    let triangle = outline(vec![polygon(&[(0.0, 0.0), (20.0, 0.0), (0.0, 20.0)])]);
    let bitmap = rasterize(&triangle, 1.0, Point::new(0.0, 0.0)).expect("raster");
    assert!((area(&bitmap.coverage) - 200.0).abs() < 1.0);
    // The right angle sits at the bottom left; the top right corner is empty.
    assert_eq!(bitmap.coverage[19], 0);
    assert_eq!(bitmap.coverage[19 * 20], 255);
}

#[test]
fn a_wide_shallow_edge_spreads_over_many_pixels() {
    let wedge = outline(vec![polygon(&[(0.0, 0.0), (40.0, 0.0), (40.0, 3.0)])]);
    let bitmap = rasterize(&wedge, 1.0, Point::new(0.0, 0.0)).expect("raster");
    assert!((area(&bitmap.coverage) - 60.0).abs() < 0.5);
}

#[test]
fn winding_direction_does_not_change_the_fill() {
    let clockwise = rasterize(&square(8.0), 1.0, Point::new(0.0, 0.0)).expect("raster");
    let reversed = outline(vec![polygon(&[
        (0.0, 0.0),
        (0.0, 8.0),
        (8.0, 8.0),
        (8.0, 0.0),
    ])]);
    let counter = rasterize(&reversed, 1.0, Point::new(0.0, 0.0)).expect("raster");
    assert_eq!(clockwise.coverage, counter.coverage);
}

#[test]
fn an_opposite_inner_contour_cuts_a_hole() {
    let ring = outline(vec![
        polygon(&[(0.0, 0.0), (12.0, 0.0), (12.0, 12.0), (0.0, 12.0)]),
        polygon(&[(4.0, 4.0), (4.0, 8.0), (8.0, 8.0), (8.0, 4.0)]),
    ]);
    let bitmap = rasterize(&ring, 1.0, Point::new(0.0, 0.0)).expect("raster");
    assert_eq!(bitmap.coverage[6 * 12 + 6], 0);
    assert_eq!(bitmap.coverage[0], 255);
    assert!((area(&bitmap.coverage) - (144.0 - 16.0)).abs() < 0.5);
}

#[test]
fn a_quadratic_bulge_adds_two_thirds_of_its_triangle() {
    // A half-disc-like cap: the chord (0,0)-(20,0) with control (10, 20).
    // The parabola's area is 2/3 of the control triangle: 2/3 * 200.
    let cap = Contour {
        start: Point::new(0.0, 0.0),
        segments: vec![Segment::Quad {
            control: Point::new(10.0, 20.0),
            to: Point::new(20.0, 0.0),
        }],
    };
    let bitmap = rasterize(&outline(vec![cap]), 1.0, Point::new(0.0, 0.0)).expect("raster");
    assert!((area(&bitmap.coverage) - 133.3).abs() < 1.5);
}

#[test]
fn negative_coordinates_move_the_bitmap_origin() {
    let shifted = outline(vec![polygon(&[
        (-3.0, -2.0),
        (5.0, -2.0),
        (5.0, 4.0),
        (-3.0, 4.0),
    ])]);
    let bitmap = rasterize(&shifted, 1.0, Point::new(0.0, 0.0)).expect("raster");
    assert_eq!((bitmap.left, bitmap.top), (-3, 4));
    assert_eq!((bitmap.width, bitmap.height), (8, 6));
}

#[test]
fn absurd_sizes_are_refused() {
    assert_eq!(
        rasterize(&square(10.0), 1.0e9, Point::new(0.0, 0.0)),
        Err(FontError::BitmapTooLarge)
    );
    assert_eq!(
        rasterize(&square(10.0), f32::NAN, Point::new(0.0, 0.0)),
        Err(FontError::BitmapTooLarge)
    );
}
