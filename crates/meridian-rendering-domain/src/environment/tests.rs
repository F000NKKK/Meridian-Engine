//! Tests of the environment build.

use super::{EnvironmentError, EnvironmentMap, Sky, direction_to_uv, uv_to_direction};

fn close(a: [f32; 3], b: [f32; 3], tolerance: f32) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tolerance)
}

fn uniform(color: [f32; 3]) -> EnvironmentMap {
    EnvironmentMap::from_equirect(32, 16, &[color; 32 * 16]).unwrap()
}

#[test]
fn directions_and_image_coordinates_round_trip() {
    for (u, v) in [(0.5, 0.5), (0.1, 0.3), (0.9, 0.8), (0.25, 0.6)] {
        let [ru, rv] = direction_to_uv(uv_to_direction(u, v));
        assert!((ru - u).abs() < 1e-4 && (rv - v).abs() < 1e-4, "({u},{v})");
    }
}

#[test]
fn the_image_centre_looks_forward_along_negative_z() {
    let d = uv_to_direction(0.5, 0.5);
    assert!(close(d, [0.0, 0.0, -1.0], 1e-5));
    assert!(close(uv_to_direction(0.5, 0.0), [0.0, 1.0, 0.0], 1e-5));
}

#[test]
fn an_empty_or_miscounted_image_is_refused() {
    assert_eq!(
        EnvironmentMap::from_equirect(0, 4, &[]),
        Err(EnvironmentError::Empty)
    );
    assert_eq!(
        EnvironmentMap::from_equirect(4, 2, &[[0.0; 3]; 7]),
        Err(EnvironmentError::TexelCountMismatch {
            expected: 8,
            actual: 7
        })
    );
}

#[test]
fn a_uniform_environment_is_uniform_at_every_level_and_in_irradiance() {
    let color = [0.4, 0.5, 0.6];
    let map = uniform(color);
    assert_eq!(map.levels().len(), super::map::SPECULAR_LEVELS);
    for level in map.levels() {
        for texel in &level.texels {
            assert!(close(*texel, color, 1e-3), "{texel:?}");
        }
    }
    for normal in [[0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]] {
        assert!(close(map.irradiance_at(normal), color, 0.02), "{normal:?}");
    }
}

#[test]
fn a_sky_is_brighter_above_than_below_and_roughness_blurs_the_sun() {
    let sky = Sky {
        sun_radiance: [40.0, 36.0, 30.0],
        sun_direction: [0.0, 0.5, -1.0],
        ..Sky::default()
    };
    let map = EnvironmentMap::from_sky(&sky).unwrap();
    let up = map.irradiance_at([0.0, 1.0, 0.0]);
    let down = map.irradiance_at([0.0, -1.0, 0.0]);
    assert!(up[2] > down[2]);

    let sun = [0.0, 0.447_213_6, -0.894_427_2];
    let peak = |index: usize| {
        let level = &map.levels()[index];
        level.sample_direction(sun)[0]
    };
    assert!(peak(0) > peak(2) && peak(2) > peak(5), "the sun spreads");
    assert!(peak(5) > 0.0);
}
