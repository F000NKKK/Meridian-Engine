//! Resolving track definitions to pixel sizes.

use crate::layout::value::GridLength;

/// Resolves `definitions` along one axis. `auto` holds the largest child
/// extent per track. `Star` tracks share what `available` leaves after the
/// `Px` and `Auto` tracks; when `available` is unbounded they size like
/// `Auto`.
pub(super) fn resolve(definitions: &[GridLength], auto: &[f32], available: f32) -> Vec<f32> {
    let fixed: f32 = definitions
        .iter()
        .zip(auto)
        .map(|(definition, content)| match definition {
            GridLength::Px(pixels) => *pixels,
            GridLength::Auto => *content,
            GridLength::Star(_) => 0.0,
        })
        .sum();
    let weights: f32 = definitions.iter().map(star_weight).sum();
    let left = (available - fixed).max(0.0);
    definitions
        .iter()
        .zip(auto)
        .map(|(definition, content)| match definition {
            GridLength::Px(pixels) => *pixels,
            GridLength::Auto => *content,
            GridLength::Star(weight) if available.is_finite() && weights > 0.0 => {
                left * weight / weights
            }
            GridLength::Star(_) => *content,
        })
        .collect()
}

fn star_weight(definition: &GridLength) -> f32 {
    match definition {
        GridLength::Star(weight) => *weight,
        _ => 0.0,
    }
}

/// Start position of each track.
pub(super) fn offsets(sizes: &[f32]) -> Vec<f32> {
    sizes
        .iter()
        .scan(0.0_f32, |position, size| {
            let start = *position;
            *position += size;
            Some(start)
        })
        .collect()
}
