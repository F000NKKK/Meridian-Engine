//! Tests of the event system and the frame scheduler.

use crate::*;
use meridian_task_core::JobGraph;

#[test]
fn event_system_round_trips_by_type() {
    let mut events = EventSystem::new();
    events.publish(1i32);
    events.publish(2i32);
    events.publish("hello");

    assert_eq!(events.drain::<i32>(), vec![1, 2]);
    assert_eq!(events.drain::<&str>(), vec!["hello"]);
}

#[test]
fn event_system_drain_empties_the_queue() {
    let mut events = EventSystem::new();
    events.publish(42i32);
    assert_eq!(events.drain::<i32>(), vec![42]);
    assert_eq!(events.drain::<i32>(), Vec::<i32>::new());
}

#[test]
fn event_system_drain_of_unpublished_type_is_empty() {
    let mut events = EventSystem::new();
    assert_eq!(events.drain::<f32>(), Vec::<f32>::new());
}

#[test]
fn frame_scheduler_runs_a_job_graph() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let ran = Arc::new(AtomicUsize::new(0));
    let mut graph = JobGraph::new();
    let ran2 = ran.clone();
    graph.add_job("job", &[], move || {
        ran2.fetch_add(1, Ordering::SeqCst);
    });

    FrameScheduler::new(2).run(graph);
    assert_eq!(ran.load(Ordering::SeqCst), 1);
}
