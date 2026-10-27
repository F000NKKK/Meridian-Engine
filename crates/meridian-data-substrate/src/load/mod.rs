//! Loading an `.mscene` document into a `World`: all or nothing, every
//! problem typed ([`SceneProblem`]) and placed ([`SceneIssue`]).

mod document_draft;
mod document_reader;
mod document_resolver;
mod error;
mod pending_entity;
mod scene_issue;
mod scene_problem;

pub(crate) use document_reader::DocumentReader;
pub use error::LoadError;
pub use scene_issue::SceneIssue;
pub use scene_problem::SceneProblem;
