//! A single-threaded executor for tests.

use crate::{Job, JobExecutor};

/// Runs the jobs one after another in list order, which is already
/// dependency order. The deterministic fake of [`JobExecutor`].
#[derive(Debug, Default, Clone, Copy)]
pub struct InlineExecutor;

impl JobExecutor for InlineExecutor {
    fn execute(&self, jobs: Vec<Job>) {
        for job in jobs {
            (job.action)();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::run_executor_contract;

    #[test]
    fn inline_executor_passes_the_contract() {
        run_executor_contract(&InlineExecutor);
    }
}
