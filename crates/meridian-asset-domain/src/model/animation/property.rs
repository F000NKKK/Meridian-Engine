//! What a channel animates.

/// A joint property a channel moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelProperty {
    /// Translation, three components per value.
    Translation,
    /// Rotation as a quaternion `[x, y, z, w]`, four components per value.
    Rotation,
    /// Scale, three components per value.
    Scale,
}

impl ModelProperty {
    /// How many numbers one value of the property takes.
    #[must_use]
    pub fn components(self) -> usize {
        match self {
            ModelProperty::Translation | ModelProperty::Scale => 3,
            ModelProperty::Rotation => 4,
        }
    }
}
