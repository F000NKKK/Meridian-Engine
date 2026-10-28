//! Registering the rendering components.

use crate::{ComponentName, ComponentRegistry, RegisterError};

use super::LightCodec;

/// Registers the rendering components a scene file can hold: `light`
/// (`LightSource`). `MeshRenderer` holds opaque handles, so it has no file
/// form here: a scene names meshes and materials, and the renderer binds them.
///
/// # Errors
///
/// [`RegisterError`] when `light`, or `LightSource`, is already registered.
pub fn register_rendering(registry: &mut ComponentRegistry) -> Result<(), RegisterError> {
    let name = ComponentName::new("light").expect("\"light\" is a valid component name");
    registry.register_with::<meridian_rendering_domain::LightSource, LightCodec>(name)?;
    Ok(())
}
