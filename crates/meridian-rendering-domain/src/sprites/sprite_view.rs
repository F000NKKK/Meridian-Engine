//! What a prepared sprite scene is drawn into.

use crate::scene::Camera2d;

/// The camera a sprite scene is seen through and the pixel size of the
/// target it is drawn into (needed to turn clip rectangles into scissors).
#[derive(Debug, Clone, Copy)]
pub struct SpriteView {
    /// Maps camera units to the target.
    pub camera: Camera2d,
    /// Width and height of the target in pixels.
    pub viewport: [u32; 2],
}
