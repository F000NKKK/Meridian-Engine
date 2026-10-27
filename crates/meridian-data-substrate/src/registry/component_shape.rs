//! [`ComponentShape`]: a registered component's description.

use substrate_data::{Schema, ShapeDoc};

use crate::ComponentName;

/// A registered component and the description of its shape — what an IDE
/// builds its inspector and completion from. A [`Schema`] itself, so the
/// list reaches the IDE as a frame.
#[derive(Debug, Clone, PartialEq, Schema)]
pub struct ComponentShape {
    /// The component's name in documents.
    pub name: ComponentName,
    /// Its shape (`Shape::describe()`).
    pub shape: ShapeDoc,
}
