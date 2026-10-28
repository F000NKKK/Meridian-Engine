//! Tests of the environment build.

use super::{CubeFaces, EnvironmentError, EnvironmentMap, Sky, direction_to_uv, uv_to_direction};

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

/// Faces of 4 x 4 texels, each filled with its own colour.
fn coloured_cube() -> CubeFaces {
    let colours = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [1.0, 0.0, 1.0],
    ];
    CubeFaces::new(4, colours.map(|colour| vec![colour; 16])).unwrap()
}

#[test]
fn each_axis_direction_reads_its_own_face() {
    let cube = coloured_cube();
    let axes = [
        ([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        ([-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, -1.0, 0.0], [1.0, 1.0, 0.0]),
        ([0.0, 0.0, 1.0], [0.0, 1.0, 1.0]),
        ([0.0, 0.0, -1.0], [1.0, 0.0, 1.0]),
    ];
    for (direction, colour) in axes {
        assert!(close(cube.sample(direction), colour, 1e-5), "{direction:?}");
    }
}

#[test]
fn faces_keep_their_orientation_in_the_equirect_image() {
    // Up in the cube is up in the image: the top row reads the +y face.
    let equirect = coloured_cube().to_equirect(16);
    assert!(close(equirect[0], [0.0, 0.0, 1.0], 1e-5));
    assert!(close(equirect[equirect.len() - 1], [1.0, 1.0, 0.0], 1e-5));
    // The image centre looks along -z: the -z face.
    let centre = equirect[4 * 16 + 8];
    assert!(close(centre, [1.0, 0.0, 1.0], 1e-5), "{centre:?}");
}

#[test]
fn a_gradient_across_a_face_is_sampled_bilinearly() {
    // Left half of the +z face is black, right half white (x grows with s).
    let texels: Vec<[f32; 3]> = (0..16)
        .map(|i| if i % 4 < 2 { [0.0; 3] } else { [1.0; 3] })
        .collect();
    let black = || vec![[0.0; 3]; 16];
    let faces = CubeFaces::new(4, [black(), black(), black(), black(), texels, black()]).unwrap();
    assert!(faces.sample([-0.5, 0.0, 1.0])[0] < 0.1);
    assert!(faces.sample([0.5, 0.0, 1.0])[0] > 0.9);
    assert!((faces.sample([0.0, 0.0, 1.0])[0] - 0.5).abs() < 1e-4);
}

#[test]
fn a_cube_builds_a_map_that_lights_from_the_right_side() {
    let map = EnvironmentMap::from_cube(&coloured_cube()).unwrap();
    let up = map.irradiance_at([0.0, 1.0, 0.0]);
    let down = map.irradiance_at([0.0, -1.0, 0.0]);
    assert!(up[2] > up[0] && up[2] > up[1], "{up:?}");
    assert!(down[0] > down[2] && down[1] > down[2], "{down:?}");
}

#[test]
fn malformed_cubes_are_refused() {
    let face = || vec![[0.0; 3]; 4];
    let faces = || [face(), face(), face(), face(), face(), face()];
    assert_eq!(CubeFaces::new(0, faces()), Err(EnvironmentError::Empty));
    let mut short = faces();
    short[3].pop();
    assert_eq!(
        CubeFaces::new(2, short),
        Err(EnvironmentError::TexelCountMismatch {
            expected: 4,
            actual: 3
        })
    );
}
