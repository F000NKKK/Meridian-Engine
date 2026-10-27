//! The shape of a job graph: in-degrees and reverse edges, checked once.

use std::collections::VecDeque;

use crate::JobGraph;

/// How many unfinished dependencies each job has, and which jobs wait on
/// each job. Built by [`Topology::of`], which rejects malformed graphs.
pub(crate) struct Topology {
    pub(crate) indegree: Vec<usize>,
    pub(crate) dependents: Vec<Vec<usize>>,
}

impl Topology {
    /// # Panics
    ///
    /// If `graph` has a dependency cycle, or a [`JobId`](crate::JobId) that
    /// doesn't belong to it — both are caller bugs, and running a cyclic
    /// graph would otherwise hang forever waiting for an in-degree that
    /// never reaches zero.
    pub(crate) fn of(graph: &JobGraph) -> Self {
        let n = graph.jobs.len();
        let mut indegree = vec![0usize; n];
        let mut dependents: Vec<Vec<usize>> = (0..n).map(|_| Vec::new()).collect();
        for (i, job) in graph.jobs.iter().enumerate() {
            for dep in &job.dependencies {
                assert!(
                    dep.0 < n,
                    "Scheduler::run: JobId({}) doesn't belong to this JobGraph",
                    dep.0
                );
                indegree[i] += 1;
                dependents[dep.0].push(i);
            }
        }
        let topology = Self {
            indegree,
            dependents,
        };
        assert!(
            !topology.has_cycle(),
            "Scheduler::run: dependency cycle in JobGraph"
        );
        topology
    }

    /// Kahn's algorithm dry run: if fewer than `n` nodes are ever reachable
    /// from a zero in-degree, some subset forms a cycle.
    fn has_cycle(&self) -> bool {
        let n = self.indegree.len();
        let mut indegree = self.indegree.clone();
        let mut queue: VecDeque<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
        let mut visited = 0usize;
        while let Some(i) = queue.pop_front() {
            visited += 1;
            for &d in &self.dependents[i] {
                indegree[d] -= 1;
                if indegree[d] == 0 {
                    queue.push_back(d);
                }
            }
        }
        visited != n
    }
}
