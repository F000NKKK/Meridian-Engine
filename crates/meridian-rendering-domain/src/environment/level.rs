//! One equirectangular image of radiance.

use super::direction::direction_to_uv;

/// An equirectangular image of linear radiance, one `[r, g, b]` per texel,
/// row-major from the top row. Values may exceed `1.0`.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentLevel {
    /// Texels per row.
    pub width: u32,
    /// Rows.
    pub height: u32,
    /// `width * height` texels.
    pub texels: Vec<[f32; 3]>,
}

impl EnvironmentLevel {
    /// The texel at column `x` (wrapping around the horizon) and row `y`
    /// (clamped at the poles).
    fn texel(&self, x: i64, y: i64) -> [f32; 3] {
        let column = x.rem_euclid(i64::from(self.width)) as usize;
        let row = y.clamp(0, i64::from(self.height) - 1) as usize;
        self.texels[row * self.width as usize + column]
    }

    /// Bilinear radiance at image coordinate `(u, v)`.
    #[must_use]
    pub fn sample(&self, u: f32, v: f32) -> [f32; 3] {
        let x = u * self.width as f32 - 0.5;
        let y = v * self.height as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let corners = [
            (self.texel(x0, y0), (1.0 - fx) * (1.0 - fy)),
            (self.texel(x0 + 1, y0), fx * (1.0 - fy)),
            (self.texel(x0, y0 + 1), (1.0 - fx) * fy),
            (self.texel(x0 + 1, y0 + 1), fx * fy),
        ];
        let mut sum = [0.0; 3];
        for (texel, weight) in corners {
            for (channel, value) in sum.iter_mut().zip(texel) {
                *channel += value * weight;
            }
        }
        sum
    }

    /// Bilinear radiance looking along unit `direction`.
    #[must_use]
    pub fn sample_direction(&self, direction: [f32; 3]) -> [f32; 3] {
        let [u, v] = direction_to_uv(direction);
        self.sample(u, v)
    }

    /// This image at half the width and height (at least one texel), each
    /// texel the mean of the 2x2 block it covers.
    #[must_use]
    pub fn halved(&self) -> Self {
        let width = (self.width / 2).max(1);
        let height = (self.height / 2).max(1);
        let mut texels = Vec::with_capacity((width * height) as usize);
        for row in 0..i64::from(height) {
            for column in 0..i64::from(width) {
                let block = [
                    self.texel(column * 2, row * 2),
                    self.texel(column * 2 + 1, row * 2),
                    self.texel(column * 2, row * 2 + 1),
                    self.texel(column * 2 + 1, row * 2 + 1),
                ];
                texels.push([0, 1, 2].map(|c| block.iter().map(|t| t[c]).sum::<f32>() * 0.25));
            }
        }
        Self {
            width,
            height,
            texels,
        }
    }
}
