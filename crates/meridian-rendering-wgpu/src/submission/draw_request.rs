//! One renderable, resolved and ready to bake.

use meridian_gac::Motor3;
use meridian_rendering_domain::MeshSource;
use meridian_rendering_domain::scene::Material;

use crate::Texture;

/// A renderable whose mesh, material and albedo texture are resolved.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DrawRequest<'a> {
    pub(crate) mesh: &'a MeshSource,
    pub(crate) material: &'a Material,
    pub(crate) frame: &'a Motor3,
    pub(crate) albedo: Option<&'a Texture>,
    pub(crate) casts_shadow: bool,
}
