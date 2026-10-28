//! The ground grid and its axes.

use meridian_gac::Vec3;

use super::ribbon::Line;
use crate::editor_renderer::EditorCamera;

/// Lines on each side of the origin.
const HALF_COUNT: i32 = 20;
/// Every this many lines one is a major line.
const MAJOR_EVERY: i32 = 5;

/// What a set of lines is drawn like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct LineStyle {
    pub(super) color: [f32; 3],
    pub(super) on_top: bool,
}

const MINOR: LineStyle = LineStyle {
    color: [0.2, 0.2, 0.22],
    on_top: false,
};
const MAJOR: LineStyle = LineStyle {
    color: [0.35, 0.35, 0.38],
    on_top: false,
};
const AXIS_X: LineStyle = LineStyle {
    color: [0.85, 0.15, 0.15],
    on_top: false,
};
const AXIS_Z: LineStyle = LineStyle {
    color: [0.2, 0.3, 0.9],
    on_top: false,
};

/// The grid around the origin, its spacing a power of ten that suits how
/// far the camera is, in the four styles it uses.
pub(super) fn grid_lines(camera: &EditorCamera) -> Vec<(LineStyle, Vec<Line>)> {
    let step = 10.0_f32.powf((camera.distance / 2.0).max(0.1).log10().floor());
    let extent = step * f32::from(i16::try_from(HALF_COUNT).unwrap_or(20));
    let (mut minor, mut major) = (Vec::new(), Vec::new());
    for index in (-HALF_COUNT..=HALF_COUNT).filter(|&i| i != 0) {
        let at = step * f32::from(i16::try_from(index).unwrap_or(0));
        let into = if index % MAJOR_EVERY == 0 {
            &mut major
        } else {
            &mut minor
        };
        into.push(line(
            Vec3::new(at, 0.0, -extent),
            Vec3::new(at, 0.0, extent),
            1.0,
        ));
        into.push(line(
            Vec3::new(-extent, 0.0, at),
            Vec3::new(extent, 0.0, at),
            1.0,
        ));
    }
    let x = line(
        Vec3::new(-extent, 0.0, 0.0),
        Vec3::new(extent, 0.0, 0.0),
        2.0,
    );
    let z = line(
        Vec3::new(0.0, 0.0, -extent),
        Vec3::new(0.0, 0.0, extent),
        2.0,
    );
    vec![
        (MINOR, minor),
        (MAJOR, major),
        (AXIS_X, vec![x]),
        (AXIS_Z, vec![z]),
    ]
}

fn line(from: Vec3, to: Vec3, width: f32) -> Line {
    Line { from, to, width }
}
