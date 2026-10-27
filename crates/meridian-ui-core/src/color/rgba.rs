//! The color value.

/// An RGBA color, each channel in `0.0..=1.0`, as authored (sRGB).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl Color {
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);

    pub const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// From 8-bit channels.
    pub fn from_rgba8(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        let channel = |value: u8| f32::from(value) / 255.0;
        Self::new(channel(red), channel(green), channel(blue), channel(alpha))
    }

    /// The channels as `[red, green, blue, alpha]`, the shape sprites tint with.
    pub const fn to_array(self) -> [f32; 4] {
        [self.red, self.green, self.blue, self.alpha]
    }

    /// True when nothing would be drawn.
    pub fn is_transparent(self) -> bool {
        self.alpha <= 0.0
    }
}
