//! Quadratic curves to short straight lines.

use crate::Point;

/// Largest distance, in pixels, a line may stray from its curve.
const TOLERANCE: f32 = 0.1;
const MAX_STEPS: u16 = 64;

/// The points after `from` along the curve `from -> control -> to`,
/// ending exactly at `to`; consecutive points are joined by lines.
pub(super) fn quad(from: Point, control: Point, to: Point) -> impl Iterator<Item = Point> {
    let deviation = (from.x - 2.0 * control.x + to.x).hypot(from.y - 2.0 * control.y + to.y) / 4.0;
    let steps = steps_for(deviation);
    (1..=steps).map(move |step| {
        let t = f32::from(step) / f32::from(steps);
        let u = 1.0 - t;
        Point::new(
            u * u * from.x + 2.0 * u * t * control.x + t * t * to.x,
            u * u * from.y + 2.0 * u * t * control.y + t * t * to.y,
        )
    })
}

/// Subdividing a quadratic into `n` lines divides its deviation by `n²`.
fn steps_for(deviation: f32) -> u16 {
    let mut steps = 1_u16;
    while steps < MAX_STEPS && deviation / (f32::from(steps) * f32::from(steps)) > TOLERANCE {
        steps += 1;
    }
    steps
}
