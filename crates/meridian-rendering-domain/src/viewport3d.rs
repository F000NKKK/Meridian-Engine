use crate::Scene3d;

/// A 3D scene clipped to a pixel rectangle of the window.
#[derive(Debug, Clone)]
pub struct Viewport3d {
    pub scene: Scene3d,
    /// Left, top, width and height in physical pixels.
    pub rect: [u32; 4],
}
