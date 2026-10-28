//! Loading an `.mscene` document into a `World`: all or nothing, every
//! problem typed ([`SceneProblem`]) and placed ([`SceneIssue`]).

mod document_draft;
mod document_reader;
mod document_resolver;
mod document_source;
mod error;
mod finding;
mod pending_entity;
mod placement;
mod reading;
mod scene_issue;
mod scene_problem;

pub(crate) use document_source::DocumentSource;
pub use error::LoadError;
pub use scene_issue::SceneIssue;
pub use scene_problem::SceneProblem;
