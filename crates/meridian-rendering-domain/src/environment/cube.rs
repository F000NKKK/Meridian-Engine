//! A cube map as a source of environment radiance.

use super::{EnvironmentError, EnvironmentMap};

/// Widest equirectangular image a cube is resampled to.
const MAX_EQUIRECT_WIDTH: u32 = 1024;

/// Six square faces of linear radiance, in the order `+x, -x, +y, -y, +z, -z`.
///
/// Faces use the OpenGL and Vulkan cube map orientation, the one standard
/// skybox sets are authored in: each is stored row-major from its top row, and
/// looking at a face from the middle of the cube, `+y` is up on the side
/// faces, and `-z` is the face the equirectangular centre looks at.
#[derive(Debug, Clone, PartialEq)]
pub struct CubeFaces {
    size: u32,
    faces: [Vec<[f32; 3]>; 6],
}

impl CubeFaces {
    /// Faces of `size` by `size` texels each.
    ///
    /// # Errors
    ///
    /// [`EnvironmentError::Empty`] for a zero `size`,
    /// [`EnvironmentError::TexelCountMismatch`] when a face is not
    /// `size * size` texels.
    pub fn new(size: u32, faces: [Vec<[f32; 3]>; 6]) -> Result<Self, EnvironmentError> {
        if size == 0 {
            return Err(EnvironmentError::Empty);
        }
        let expected = size as usize * size as usize;
        match faces.iter().find(|face| face.len() != expected) {
            Some(face) => Err(EnvironmentError::TexelCountMismatch {
                expected,
                actual: face.len(),
            }),
            None => Ok(Self { size, faces }),
        }
    }

    /// Radiance looking along `direction` (need not be normalised, not zero),
    /// bilinear inside the face the direction hits.
    #[must_use]
    pub fn sample(&self, direction: [f32; 3]) -> [f32; 3] {
        let (face, s, t) = project(direction);
        let edge = self.size as f32;
        let x = (s * edge - 0.5).clamp(0.0, edge - 1.0);
        let y = (t * edge - 0.5).clamp(0.0, edge - 1.0);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let last = self.size as usize - 1;
        let (x0, y0) = (x0 as usize, y0 as usize);
        let at = |column: usize, row: usize| {
            self.faces[face][row.min(last) * self.size as usize + column.min(last)]
        };
        let corners = [
            (at(x0, y0), (1.0 - fx) * (1.0 - fy)),
            (at(x0 + 1, y0), fx * (1.0 - fy)),
            (at(x0, y0 + 1), (1.0 - fx) * fy),
            (at(x0 + 1, y0 + 1), fx * fy),
        ];
        let mut sum = [0.0; 3];
        for (texel, weight) in corners {
            for (channel, value) in sum.iter_mut().zip(texel) {
                *channel += value * weight;
            }
        }
        sum
    }

    /// These faces resampled as an equirectangular image of `width` by
    /// `width / 2` texels (see [`uv_to_direction`](super::uv_to_direction)).
    #[must_use]
    pub fn to_equirect(&self, width: u32) -> Vec<[f32; 3]> {
        let height = width / 2;
        let mut texels = Vec::with_capacity(width as usize * height as usize);
        for row in 0..height {
            for column in 0..width {
                let u = (column as f32 + 0.5) / width as f32;
                let v = (row as f32 + 0.5) / height as f32;
                texels.push(self.sample(super::uv_to_direction(u, v)));
            }
        }
        texels
    }
}

/// The face `direction` hits and the position `(s, t)` in it, both `0..=1`
/// with `t` counted from the face's top row: the cube map table of the
/// OpenGL and Vulkan specifications.
fn project(direction: [f32; 3]) -> (usize, f32, f32) {
    let [dx, dy, dz] = direction;
    let (ax, ay, az) = (dx.abs(), dy.abs(), dz.abs());
    // The face, the major axis' length, then the in-face coordinates `across`
    // (to the right) and `down`.
    let (face, major, across, down) = if ax >= ay && ax >= az {
        if dx > 0.0 {
            (0, ax, -dz, -dy)
        } else {
            (1, ax, dz, -dy)
        }
    } else if ay >= az {
        if dy > 0.0 {
            (2, ay, dx, dz)
        } else {
            (3, ay, dx, -dz)
        }
    } else if dz > 0.0 {
        (4, az, dx, -dy)
    } else {
        (5, az, -dx, -dy)
    };
    (
        face,
        (across / major + 1.0) * 0.5,
        (down / major + 1.0) * 0.5,
    )
}

impl EnvironmentMap {
    /// Builds a map from the radiance of a cube map, resampled to an
    /// equirectangular image first.
    ///
    /// # Errors
    ///
    /// Never in practice: the resampled image has a valid size, so this only
    /// forwards [`EnvironmentMap::from_equirect`]'s error type.
    pub fn from_cube(cube: &CubeFaces) -> Result<Self, EnvironmentError> {
        let width = (cube.size * 4).min(MAX_EQUIRECT_WIDTH);
        Self::from_equirect(width, width / 2, &cube.to_equirect(width))
    }
}
