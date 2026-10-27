//! Fake and contract suite for [`JobExecutor`](crate::JobExecutor) (Craftbook 8.4).

mod executor_contract;
mod inline_executor;

pub use executor_contract::run_executor_contract;
pub use inline_executor::InlineExecutor;
