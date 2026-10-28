//! Vertex layout description: attributes, formats, layouts.

mod attribute_desc;
mod format;
mod layout;
mod step;

pub use attribute_desc::VertexAttributeDesc;
pub use format::VertexFormat;
pub use layout::VertexLayout;
pub use step::VertexStep;
