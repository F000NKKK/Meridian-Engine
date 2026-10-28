//! What loading a scene document can say: [`LoadError`] with every
//! problem typed ([`SceneProblem`]) and placed ([`SceneIssue`]).

mod error;
mod scene_issue;
mod scene_problem;

pub use error::LoadError;
pub use scene_issue::SceneIssue;
pub use scene_problem::SceneProblem;
