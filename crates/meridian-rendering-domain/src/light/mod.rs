//! The light component and its extraction.

mod extract;
mod kind;
mod source;

pub use extract::extract_lights;
pub use kind::LightKind;
pub use source::LightSource;
