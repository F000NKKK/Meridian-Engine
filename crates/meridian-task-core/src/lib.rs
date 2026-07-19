//! Job graph scheduler: worker threads, dependency tracking and parallel frame execution.
//!
//! Systems declare a job's dependencies when adding it to a [`JobGraph`];
//! [`Scheduler::run`] derives execution order from that graph rather than
//! the caller hand-sequencing calls, and runs independent branches (see
//! docs/threading-model.md's shape-of-a-frame example) across worker
//! threads in parallel. Current implementation is a single shared
//! ready-queue behind one mutex, not per-worker lock-free deques — see
//! "Implementation note" on [`Scheduler`] for why that's a deliberate
//! first step, not the final design.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

/// Opaque handle to a job within the [`JobGraph`] it was added to. Not
/// valid across different `JobGraph`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobId(usize);

type Action = Box<dyn FnOnce() + Send>;

struct JobEntry {
    name: &'static str,
    action: Option<Action>,
    dependencies: Vec<JobId>,
}

/// A dependency-ordered set of jobs for one frame. Build with
/// [`add_job`](Self::add_job), then hand to [`Scheduler::run`].
#[derive(Default)]
pub struct JobGraph {
    jobs: Vec<JobEntry>,
}

impl JobGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    /// The name a job was given when added — for logging/debugging which
    /// job is running, not used by the scheduler itself.
    pub fn job_name(&self, id: JobId) -> &'static str {
        self.jobs[id.0].name
    }

    /// Adds a job that becomes runnable only once every job in
    /// `dependencies` has completed. `dependencies` must be [`JobId`]s
    /// returned earlier from this same graph.
    pub fn add_job(
        &mut self,
        name: &'static str,
        dependencies: &[JobId],
        action: impl FnOnce() + Send + 'static,
    ) -> JobId {
        let id = JobId(self.jobs.len());
        self.jobs.push(JobEntry {
            name,
            action: Some(Box::new(action)),
            dependencies: dependencies.to_vec(),
        });
        id
    }
}

/// Per-run scheduling state, behind one lock: in-degree counts, the
/// reverse-edge (dependents) list, the queue of runnable-but-not-yet-taken
/// job indices, and how many jobs are still outstanding.
struct SchedulerState {
    indegree: Vec<usize>,
    dependents: Vec<Vec<usize>>,
    ready: VecDeque<usize>,
    remaining: usize,
}

/// Runs a [`JobGraph`] across worker threads, respecting declared
/// dependencies.
///
/// ## Implementation note
///
/// Every worker pulls from one shared `Mutex`-guarded ready-queue rather
/// than each having its own deque with the classic work-stealing protocol
/// (steal from a random peer when your own queue is empty). A single
/// shared queue is correct and easy to verify by test; per-worker
/// lock-free deques are a real throughput win at high job counts, but
/// implementing that safely is a separate, riskier piece of work — not
/// worth taking on before there's a real frame's worth of jobs to profile
/// against. Swapping the internals later doesn't change this type's API.
#[derive(Debug)]
pub struct Scheduler {
    pub worker_count: usize,
}

impl Scheduler {
    pub fn new(worker_count: usize) -> Self {
        Self {
            worker_count: worker_count.max(1),
        }
    }

    /// Runs every job in `graph` to completion. Blocks the calling thread
    /// until the whole graph has finished.
    ///
    /// # Panics
    ///
    /// Panics if `graph` contains a dependency cycle, or a [`JobId`] that
    /// doesn't belong to it — both are caller bugs, not runtime
    /// conditions to recover from, and running a cyclic graph would
    /// otherwise hang forever waiting for an in-degree that never reaches
    /// zero.
    pub fn run(&self, mut graph: JobGraph) {
        let n = graph.jobs.len();
        if n == 0 {
            return;
        }

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
        assert!(
            !has_cycle(&indegree, &dependents, n),
            "Scheduler::run: dependency cycle in JobGraph"
        );

        let actions: Mutex<Vec<Option<Action>>> =
            Mutex::new(graph.jobs.iter_mut().map(|j| j.action.take()).collect());

        let ready: VecDeque<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
        let state = Mutex::new(SchedulerState {
            indegree,
            dependents,
            ready,
            remaining: n,
        });
        let cvar = Condvar::new();

        std::thread::scope(|scope| {
            for _ in 0..self.worker_count {
                scope.spawn(|| worker_loop(&state, &cvar, &actions));
            }
        });
    }
}

fn worker_loop(
    state: &Mutex<SchedulerState>,
    cvar: &Condvar,
    actions: &Mutex<Vec<Option<Action>>>,
) {
    loop {
        let job_index = {
            let mut guard = state.lock().unwrap();
            loop {
                if let Some(i) = guard.ready.pop_front() {
                    break Some(i);
                }
                if guard.remaining == 0 {
                    break None;
                }
                guard = cvar.wait(guard).unwrap();
            }
        };

        let Some(i) = job_index else {
            break;
        };

        let action = actions.lock().unwrap()[i].take();
        if let Some(action) = action {
            action();
        }

        let mut guard = state.lock().unwrap();
        let newly_ready: Vec<usize> = std::mem::take(&mut guard.dependents[i])
            .into_iter()
            .filter(|&d| {
                guard.indegree[d] -= 1;
                guard.indegree[d] == 0
            })
            .collect();
        guard.ready.extend(newly_ready);
        guard.remaining -= 1;
        drop(guard);
        cvar.notify_all();
    }
}

/// Kahn's algorithm dry run: if fewer than `n` nodes are ever reachable
/// from a zero in-degree, some subset forms a cycle.
fn has_cycle(indegree: &[usize], dependents: &[Vec<usize>], n: usize) -> bool {
    let mut indegree = indegree.to_vec();
    let mut queue: VecDeque<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
    let mut visited = 0usize;
    while let Some(i) = queue.pop_front() {
        visited += 1;
        for &d in &dependents[i] {
            indegree[d] -= 1;
            if indegree[d] == 0 {
                queue.push_back(d);
            }
        }
    }
    visited != n
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex as StdMutex};

    #[test]
    fn empty_graph_runs_without_blocking() {
        Scheduler::new(2).run(JobGraph::new());
    }

    #[test]
    fn single_job_runs() {
        let ran = Arc::new(AtomicUsize::new(0));
        let mut graph = JobGraph::new();
        let ran2 = ran.clone();
        graph.add_job("only", &[], move || {
            ran2.fetch_add(1, Ordering::SeqCst);
        });
        Scheduler::new(1).run(graph);
        assert_eq!(ran.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn dependent_job_runs_after_its_dependency() {
        let order = Arc::new(StdMutex::new(Vec::new()));
        let mut graph = JobGraph::new();

        let order_a = order.clone();
        let a = graph.add_job("a", &[], move || order_a.lock().unwrap().push("a"));

        let order_b = order.clone();
        graph.add_job("b", &[a], move || order_b.lock().unwrap().push("b"));

        Scheduler::new(4).run(graph);
        assert_eq!(*order.lock().unwrap(), vec!["a", "b"]);
    }

    #[test]
    fn independent_jobs_all_run_regardless_of_worker_count() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut graph = JobGraph::new();
        for _ in 0..20 {
            let c = count.clone();
            graph.add_job("independent", &[], move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }
        Scheduler::new(4).run(graph);
        assert_eq!(count.load(Ordering::SeqCst), 20);
    }

    #[test]
    fn diamond_dependency_runs_in_valid_order() {
        // a -> b, a -> c, (b, c) -> d — d must see both b's and c's effects.
        let order = Arc::new(StdMutex::new(Vec::new()));
        let mut graph = JobGraph::new();

        let oa = order.clone();
        let a = graph.add_job("a", &[], move || oa.lock().unwrap().push("a"));

        let ob = order.clone();
        let b = graph.add_job("b", &[a], move || ob.lock().unwrap().push("b"));

        let oc = order.clone();
        let c = graph.add_job("c", &[a], move || oc.lock().unwrap().push("c"));

        let od = order.clone();
        graph.add_job("d", &[b, c], move || od.lock().unwrap().push("d"));

        Scheduler::new(4).run(graph);

        let order = order.lock().unwrap();
        assert_eq!(order.first(), Some(&"a"));
        assert_eq!(order.last(), Some(&"d"));
        assert_eq!(order.len(), 4);
    }

    #[test]
    #[should_panic(expected = "dependency cycle")]
    fn cyclic_graph_panics_instead_of_hanging() {
        // Build a 2-cycle by hand: add_job only accepts already-issued
        // JobIds, so a JobGraph built purely through the public API can
        // never contain a cycle. To exercise the panic path we reach past
        // the API and forge a cycle directly on the private field — this
        // is the one test in the suite allowed to do that, specifically
        // to prove the guard that protects against a malformed graph
        // (e.g. one deserialized from bad data in the future) works.
        let mut graph = JobGraph::new();
        let a = graph.add_job("a", &[], || {});
        let b = graph.add_job("b", &[a], || {});
        graph.jobs[a.0].dependencies.push(b);

        Scheduler::new(2).run(graph);
    }
}
