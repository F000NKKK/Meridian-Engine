//! What decides whether two draws can share one instanced call.

use super::{DrawState, MaterialMaps};
use crate::MeshHandle;

/// Two draws with equal keys use the same mesh, pipeline and bind group, so
/// one instanced call can draw both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchKey {
    /// The mesh drawn.
    pub mesh: MeshHandle,
    /// The textures that resolved; none draws flat colour.
    pub maps: MaterialMaps,
    /// Metallic-roughness shaded rather than unlit.
    pub lit: bool,
    /// Blend and depth behaviour.
    pub state: DrawState,
    /// Whether the draw is part of the shadow-casting pass.
    pub casts_shadow: bool,
}
