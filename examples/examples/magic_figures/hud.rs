//! A small 2D overlay: a translucent panel with a pulsing bar clipped to
//! the panel, drawn over the 3D scene (the sprite pass, `Overlay2d`).

use meridian_sdk::{
    Camera2d, ClipRect, ClipSpan, GraphicsSession, Motor3, Overlay2d, Scene2d, Sprite,
    TextureHandle, Vec3,
};

const PANEL: [f32; 4] = [16.0, 16.0, 220.0, 36.0];

/// The overlay's one shared texture: a white pixel tinted per sprite.
pub(crate) struct Hud {
    white: TextureHandle,
}

impl Hud {
    pub(crate) fn new(base: &mut GraphicsSession) -> Self {
        let white = base.textures.upload_solid(&base.device, [255; 4]);
        Self { white }
    }

    /// The panel at the top-left of a window of `size` pixels; `time`
    /// drives the bar, which overshoots the panel to show the clip.
    pub(crate) fn overlay(&self, size: [u32; 2], time: f32) -> Overlay2d {
        let [x, y, w, h] = PANEL;
        let bar_width = w * (0.5 + 0.5 * (time * 1.5).sin()) * 1.4;
        let scene = Scene2d {
            camera: Camera2d::from_surface_size(pixels(size[0]), pixels(size[1])),
            sprites: vec![
                self.rect(
                    [x + w / 2.0, y + h / 2.0],
                    [w, h],
                    [0.02, 0.02, 0.05, 0.6],
                    0,
                ),
                self.rect(
                    [x + bar_width / 2.0, y + h / 2.0],
                    [bar_width, h - 12.0],
                    [1.0, 0.7, 0.2, 0.9],
                    1,
                ),
            ],
        };
        let clip = ClipRect {
            origin: [x, y],
            size: [w, h],
        };
        Overlay2d {
            scene,
            clips: vec![ClipSpan {
                layers: 1..=1,
                rect: clip,
            }],
        }
    }

    fn rect(&self, centre: [f32; 2], size: [f32; 2], tint: [f32; 4], layer: i32) -> Sprite {
        Sprite {
            texture: self.white,
            source_rect: None,
            size,
            tint,
            frame: Motor3::translation(Vec3::new(centre[0], centre[1], 0.0)),
            layer,
        }
    }
}

/// A pixel count as a float.
#[expect(
    clippy::cast_precision_loss,
    reason = "window sizes are far below 2^24"
)]
fn pixels(count: u32) -> f32 {
    count as f32
}
