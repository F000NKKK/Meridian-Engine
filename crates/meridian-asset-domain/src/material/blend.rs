//! [`MaterialBlend`]: how a material's colour combines with what is behind it.

use substrate_data::Schema;

/// How a surface is composited. Written as lower case text (`opaque`,
/// `alpha_blend`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Schema)]
pub enum MaterialBlend {
    /// Replaces what is behind it.
    #[default]
    #[data(rename = "opaque")]
    Opaque,
    /// Mixes by the colour's alpha.
    #[data(rename = "alpha_blend")]
    AlphaBlend,
}
