//! Registering the rendering components.

use super::{LightCodec, MeshReference};
use crate::{ComponentName, ComponentRegistry, RegisterError};

/// Registers the rendering components a scene file can hold: `light`
/// (`LightSource`) and `mesh_renderer` ([`MeshReference`]). The runtime
/// `MeshRenderer` holds opaque handles, so the file holds the typed `@asset:`
/// references and the sdk binds them to handles after the assets load.
///
/// # Errors
///
/// [`RegisterError`] when `light`, `mesh_renderer`, or their types are already registered.
pub fn register_rendering(registry: &mut ComponentRegistry) -> Result<(), RegisterError> {
    let name = ComponentName::new("light").expect("\"light\" is a valid component name");
    registry.register_with::<meridian_rendering_domain::LightSource, LightCodec>(name)?;
    let name = ComponentName::new("mesh_renderer").expect("\"mesh_renderer\" is a valid name");
    registry.register::<MeshReference>(name)?;
    Ok(())
}
