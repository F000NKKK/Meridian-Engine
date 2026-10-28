//! Tests of stage dependencies and selection.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use meridian_compute_app::testing::SequentialDevice;
use meridian_compute_app::{ComputeContext, ComputeKernel, ComputeScheduler, DispatchSize};

use crate::testing::InlineExecutor;
use crate::*;

fn runtime() -> Runtime {
    Runtime::new(Arc::new(InlineExecutor))
}

/// A stage that depends on another must see its finished output — the
/// whole point of declaring the dependency, not an accident of order.
#[test]
fn dependent_stage_sees_upstream_stages_finished_work() {
    struct SetValue(Shared<f32>, f32);
    impl Stage for SetValue {
        fn run(&mut self) {
            *self.0.lock() = self.1;
        }
    }
    struct ReadValueInto(Shared<f32>, Arc<Mutex<f32>>);
    impl Stage for ReadValueInto {
        fn run(&mut self) {
            *self.1.lock().unwrap() = *self.0.lock();
        }
    }

    let value = Shared::new(0.0f32);
    let observed = Arc::new(Mutex::new(0.0f32));
    let mut runtime = runtime();
    let set = runtime.add_stage("set", &[], SetValue(value.clone(), 42.0));
    runtime.add_stage("read", &[set], ReadValueInto(value, observed.clone()));

    runtime.tick();

    assert_eq!(*observed.lock().unwrap(), 42.0);
}

/// A [`ComputeStage`] wrapping a trivial `ComputeKernel` must dispatch it
/// through `Runtime::tick`, the same as any other stage.
#[test]
fn compute_stage_dispatches_its_kernel_through_tick() {
    struct CountingKernel(Arc<AtomicUsize>);
    impl ComputeKernel for CountingKernel {
        fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
            context.parallel_for(size.x as usize, |_| {
                self.0.fetch_add(1, Ordering::SeqCst);
            });
        }
    }

    let dispatch_count = Arc::new(AtomicUsize::new(0));
    let mut runtime = runtime();
    runtime.add_stage(
        "compute",
        &[],
        ComputeStage::new(
            CountingKernel(dispatch_count.clone()),
            DispatchSize { x: 10, y: 1, z: 1 },
            ComputeScheduler::new(Arc::new(SequentialDevice)),
        ),
    );

    runtime.tick();

    assert_eq!(dispatch_count.load(Ordering::SeqCst), 10);
}

struct Counter(Arc<AtomicUsize>);
impl Stage for Counter {
    fn run(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// `tick_only` must run *only* the named stages — the physics-many-times,
/// render-once split its doc comment describes.
#[test]
fn tick_only_runs_exactly_the_selected_stages() {
    let physics_runs = Arc::new(AtomicUsize::new(0));
    let render_runs = Arc::new(AtomicUsize::new(0));
    let mut runtime = runtime();
    let physics_id = runtime.add_stage("physics", &[], Counter(physics_runs.clone()));
    let render_id = runtime.add_stage("render", &[physics_id], Counter(render_runs.clone()));

    // An accumulator catching up 3 physics steps, then one render.
    runtime.tick_only(&[physics_id]);
    runtime.tick_only(&[physics_id]);
    runtime.tick_only(&[physics_id]);
    runtime.tick_only(&[render_id]);

    assert_eq!(physics_runs.load(Ordering::SeqCst), 3);
    assert_eq!(render_runs.load(Ordering::SeqCst), 1);
}

/// `resize_all` reaches every stage, in registration order.
#[test]
fn resize_all_forwards_the_size_to_every_stage() {
    struct Sized(Arc<Mutex<Vec<(u32, u32)>>>);
    impl Stage for Sized {
        fn run(&mut self) {}
        fn resize(&mut self, width: u32, height: u32) {
            self.0.lock().unwrap().push((width, height));
        }
    }

    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = runtime();
    runtime.add_stage("a", &[], Sized(seen.clone()));
    runtime.add_stage("b", &[], Sized(seen.clone()));

    runtime.resize_all(640, 480);

    assert_eq!(*seen.lock().unwrap(), vec![(640, 480), (640, 480)]);
}

/// A dependency left out of `ids` is treated as already satisfied.
#[test]
fn tick_only_treats_unselected_dependencies_as_satisfied() {
    let runs = Arc::new(AtomicUsize::new(0));
    let mut runtime = runtime();
    let first = runtime.add_stage("first", &[], Counter(Arc::new(AtomicUsize::new(0))));
    let second = runtime.add_stage("second", &[first], Counter(runs.clone()));

    runtime.tick_only(&[second]);

    assert_eq!(runs.load(Ordering::SeqCst), 1);
}
