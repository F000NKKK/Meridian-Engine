//! The studio environment thumbnails are lit and reflected with: a dark
//! room with a few bright soft boxes, so metals have something to mirror.

use core::f32::consts::{PI, TAU};

use meridian_rendering_domain::{EnvironmentMap, uv_to_direction};

const WIDTH: u16 = 256;
const HEIGHT: u16 = 128;

/// One rectangular light in the room: where it hangs (azimuth and elevation
/// of its centre, radians), its half sizes, and its radiance.
struct SoftBox {
    azimuth: f32,
    elevation: f32,
    half_width: f32,
    half_height: f32,
    radiance: [f32; 3],
}

const BOXES: [SoftBox; 4] = [
    // Key: large, warm, in front of the object, left and above.
    SoftBox {
        azimuth: 2.5,
        elevation: 1.0,
        half_width: 0.45,
        half_height: 0.32,
        radiance: [10.0, 9.4, 8.5],
    },
    // Fill: cool and dimmer, from the right.
    SoftBox {
        azimuth: -2.2,
        elevation: 0.25,
        half_width: 0.35,
        half_height: 0.45,
        radiance: [2.0, 2.4, 3.1],
    },
    // Rim: a thin strip behind, to outline the silhouette.
    SoftBox {
        azimuth: 0.0,
        elevation: 0.45,
        half_width: 0.6,
        half_height: 0.12,
        radiance: [7.0, 7.0, 8.0],
    },
    // Top: an overhead panel.
    SoftBox {
        azimuth: 1.0,
        elevation: 1.35,
        half_width: 0.7,
        half_height: 0.2,
        radiance: [2.0, 2.0, 2.0],
    },
];

/// The studio as an environment map.
///
/// # Panics
///
/// Never: the image has a fixed valid size.
pub(super) fn studio_environment() -> EnvironmentMap {
    let mut texels = Vec::with_capacity(usize::from(WIDTH) * usize::from(HEIGHT));
    for row in 0..HEIGHT {
        for column in 0..WIDTH {
            let u = (f32::from(column) + 0.5) / f32::from(WIDTH);
            let v = (f32::from(row) + 0.5) / f32::from(HEIGHT);
            texels.push(radiance(uv_to_direction(u, v)));
        }
    }
    EnvironmentMap::from_equirect(u32::from(WIDTH), u32::from(HEIGHT), &texels)
        .expect("the studio image is well formed")
}

fn radiance(direction: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = direction;
    let azimuth = x.atan2(-z) + PI;
    let elevation = y.clamp(-1.0, 1.0).asin();
    let mut colour = room(elevation);
    for light in &BOXES {
        let weight = light.weight(azimuth, elevation);
        for (channel, emitted) in colour.iter_mut().zip(light.radiance) {
            *channel += emitted * weight;
        }
    }
    colour
}

/// The dark walls: lighter at the horizon, darker above and below.
fn room(elevation: f32) -> [f32; 3] {
    let t = (elevation.abs() / (PI / 2.0)).clamp(0.0, 1.0);
    let level = if elevation >= 0.0 {
        0.14 * (1.0 - t) + 0.04 * t
    } else {
        0.06 * (1.0 - t) + 0.02 * t
    };
    [level, level * 1.02, level * 1.1]
}

impl SoftBox {
    /// `1` inside the box, fading to `0` over its soft edge.
    fn weight(&self, azimuth: f32, elevation: f32) -> f32 {
        let across = wrapped(azimuth - (self.azimuth + PI)).abs();
        let up = (elevation - self.elevation).abs();
        edge(across, self.half_width) * edge(up, self.half_height)
    }
}

/// The smooth falloff of one axis: full strength inside `half`, nothing a
/// fifth of `half` further out.
fn edge(offset: f32, half: f32) -> f32 {
    let soft = half * 0.2 + 1e-4;
    ((half + soft - offset) / (2.0 * soft)).clamp(0.0, 1.0)
}

fn wrapped(angle: f32) -> f32 {
    (angle + PI).rem_euclid(TAU) - PI
}
