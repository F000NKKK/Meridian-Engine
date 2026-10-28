//! The ground grid and its axes.

use meridian_gac::Vec3;

use super::ribbon::Line;
use crate::editor_renderer::EditorCamera;

/// Lines on each side of the view's centre.
const HALF_COUNT: i32 = 10;
/// Every this many lines one is a major line.
const MAJOR_EVERY: i32 = 5;
/// The grid fades out in this many steps of opacity.
const FADE_LEVELS: u8 = 4;
/// How far above the ground the lines float, in steps, so that they do not
/// fight the ground's own surface for the depth test.
const LIFT: f32 = 0.004;
/// Below this opacity a piece of a line is not drawn at all.
const MIN_ALPHA: f32 = 0.1;

/// What a set of lines is drawn like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct LineStyle {
    pub(super) color: [f32; 3],
    /// How opaque the lines are, `1.0` for fully.
    pub(super) alpha: f32,
    pub(super) on_top: bool,
}

/// Which kind of grid line a piece belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Minor,
    Major,
    AxisX,
    AxisZ,
}

impl Kind {
    fn color(self) -> [f32; 3] {
        match self {
            Self::Minor => [0.2, 0.2, 0.22],
            Self::Major => [0.35, 0.35, 0.38],
            Self::AxisX => [0.85, 0.15, 0.15],
            Self::AxisZ => [0.2, 0.3, 0.9],
        }
    }

    fn width(self) -> f32 {
        match self {
            Self::Minor | Self::Major => 1.0,
            Self::AxisX | Self::AxisZ => 2.0,
        }
    }
}

/// The grid around the point the camera looks at, its spacing a power of ten
/// that suits how far the camera is. Each line is cut into pieces one step
/// long that fade out with their distance from that point, so the grid ends
/// softly instead of running on to the horizon.
pub(super) fn grid_lines(camera: &EditorCamera) -> Vec<(LineStyle, Vec<Line>)> {
    let step = 10.0_f32.powf((camera.distance / 2.0).max(0.1).log10().floor());
    let radius = step * f32::from(i16::try_from(HALF_COUNT).unwrap_or(10));
    let [target_x, _, target_z] = camera.target;
    let (first_x, first_z) = ((target_x / step).round(), (target_z / step).round());
    let mut groups = Vec::new();
    for offset in -HALF_COUNT..=HALF_COUNT {
        let along = f32::from(i16::try_from(offset).unwrap_or(0));
        let (index_x, index_z) = (first_x + along, first_z + along);
        let lines = [(index_x, Kind::AxisZ, true), (index_z, Kind::AxisX, false)];
        for (index, axis, across_x) in lines {
            let kind = line_kind(index, axis);
            let (at_x, at_z) = (index_x * step, index_z * step);
            for piece in 0..2 * HALF_COUNT {
                let from = f32::from(i16::try_from(piece - HALF_COUNT).unwrap_or(0)) * step;
                let (start, end) = if across_x {
                    (
                        (at_x, first_z * step + from),
                        (at_x, first_z * step + from + step),
                    )
                } else {
                    (
                        (first_x * step + from, at_z),
                        (first_x * step + from + step, at_z),
                    )
                };
                push_piece(
                    &mut groups,
                    kind,
                    (start, end),
                    (target_x, target_z),
                    (radius, step * LIFT),
                );
            }
        }
    }
    groups
}

/// Which kind of line the grid line at `index` (in steps from the origin) is:
/// the axis through the origin, a major line, or a minor one.
fn line_kind(index: f32, axis: Kind) -> Kind {
    // Indices are whole numbers held in floats, so a half is "close enough".
    let every = f32::from(i16::try_from(MAJOR_EVERY).unwrap_or(5));
    let from_major = index.rem_euclid(every);
    if index.abs() < 0.5 {
        axis
    } else if from_major < 0.5 || from_major > every - 0.5 {
        Kind::Major
    } else {
        Kind::Minor
    }
}

/// Adds the piece from `start` to `end` (ground coordinates `(x, z)`) to the
/// group of its kind and opacity, unless it has faded away.
fn push_piece(
    groups: &mut Vec<(LineStyle, Vec<Line>)>,
    kind: Kind,
    (start, end): ((f32, f32), (f32, f32)),
    target: (f32, f32),
    (radius, lift): (f32, f32),
) {
    let middle = (
        start.0.midpoint(end.0) - target.0,
        start.1.midpoint(end.1) - target.1,
    );
    let fade = 1.0 - middle.0.hypot(middle.1) / radius;
    if fade < MIN_ALPHA {
        return;
    }
    let levels = f32::from(FADE_LEVELS);
    let style = LineStyle {
        color: kind.color(),
        alpha: (fade * levels).ceil().min(levels) / levels,
        on_top: false,
    };
    let line = Line {
        from: Vec3::new(start.0, lift, start.1),
        to: Vec3::new(end.0, lift, end.1),
        width: kind.width(),
    };
    match groups.iter_mut().find(|(existing, _)| *existing == style) {
        Some((_, lines)) => lines.push(line),
        None => groups.push((style, vec![line])),
    }
}
