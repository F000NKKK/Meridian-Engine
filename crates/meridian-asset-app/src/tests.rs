//! The loader against in-memory fakes.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use meridian_asset_domain::Decoder;

use super::*;

struct Files(HashMap<&'static str, Vec<u8>>);

impl ByteSource for Files {
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        self.0
            .get(path.as_str())
            .cloned()
            .ok_or_else(|| SourceError::NotFound(path.clone()))
    }
}

/// Runs a job right away on the calling thread.
struct Inline;

impl Spawn for Inline {
    fn spawn(&self, _priority: Priority, job: Box<dyn FnOnce() + Send>) {
        job();
    }
}

/// Runs each job on its own thread.
struct Threaded;

impl Spawn for Threaded {
    fn spawn(&self, _priority: Priority, job: Box<dyn FnOnce() + Send>) {
        thread::spawn(job);
    }
}

/// Never runs a job: the pool went away.
struct Gone;

impl Spawn for Gone {
    fn spawn(&self, _priority: Priority, _job: Box<dyn FnOnce() + Send>) {}
}

struct Sum;

impl Decoder<u32> for Sum {
    type Error = &'static str;
    fn decode(&self, bytes: &[u8]) -> Result<u32, Self::Error> {
        if bytes.is_empty() {
            return Err("empty");
        }
        Ok(bytes.iter().map(|&b| u32::from(b)).sum())
    }
}

struct Explodes;

impl Decoder<u32> for Explodes {
    type Error = &'static str;
    fn decode(&self, _bytes: &[u8]) -> Result<u32, Self::Error> {
        panic!("corrupt header");
    }
}

fn loader(spawner: Arc<dyn Spawn>) -> AssetLoader {
    let files = Files(HashMap::from([
        ("a.bin", vec![1, 2, 3]),
        ("empty.bin", vec![]),
    ]));
    AssetLoader::new(Arc::new(files), spawner)
}

#[test]
fn load_reads_then_decodes() {
    let pending = loader(Arc::new(Inline)).load(AssetPath::new("a.bin"), Sum);
    assert_eq!(pending.wait(), Ok(6));
}

#[test]
fn a_missing_asset_is_a_source_error() {
    let pending = loader(Arc::new(Inline)).load(AssetPath::new("nope.bin"), Sum);
    assert_eq!(
        pending.wait(),
        Err(LoadError::Source(SourceError::NotFound(AssetPath::new(
            "nope.bin"
        ))))
    );
}

#[test]
fn a_decoder_error_is_reported_not_swallowed() {
    let pending = loader(Arc::new(Inline)).load(AssetPath::new("empty.bin"), Sum);
    assert_eq!(pending.wait(), Err(LoadError::Decode("empty")));
}

#[test]
fn a_decoder_panic_becomes_an_error_and_wait_does_not_hang() {
    let pending = loader(Arc::new(Threaded)).load(AssetPath::new("a.bin"), Explodes);
    assert_eq!(pending.wait(), Err(LoadError::Panicked));
}

#[test]
fn a_job_that_never_runs_cancels_the_load() {
    let pending = loader(Arc::new(Gone)).load(AssetPath::new("a.bin"), Sum);
    assert_eq!(pending.wait(), Err(LoadError::Cancelled));
}

#[test]
fn take_polls_without_blocking_and_returns_the_result_once() {
    let pending = loader(Arc::new(Inline)).load(AssetPath::new("a.bin"), Sum);
    assert!(pending.is_ready());
    assert_eq!(pending.take(), Some(Ok(6)));
    assert_eq!(pending.take(), None);
}

#[test]
fn take_is_none_while_the_job_has_not_run() {
    struct Parked(std::sync::Mutex<Vec<Box<dyn FnOnce() + Send>>>);
    impl Spawn for Parked {
        fn spawn(&self, _priority: Priority, job: Box<dyn FnOnce() + Send>) {
            self.0.lock().unwrap().push(job);
        }
    }
    let spawner = Arc::new(Parked(std::sync::Mutex::new(Vec::new())));
    let pending = loader(spawner.clone()).load(AssetPath::new("a.bin"), Sum);
    assert!(!pending.is_ready());
    assert_eq!(pending.take(), None);

    let job = spawner.0.lock().unwrap().pop().unwrap();
    job();
    assert_eq!(pending.take(), Some(Ok(6)));
}

#[test]
fn many_loads_in_flight_all_resolve() {
    let finished = Arc::new(AtomicUsize::new(0));
    let loader = loader(Arc::new(Threaded));
    let pending: Vec<_> = (0..200)
        .map(|_| loader.load(AssetPath::new("a.bin"), Sum))
        .collect();
    for load in pending {
        assert_eq!(load.wait(), Ok(6));
        finished.fetch_add(1, Ordering::SeqCst);
    }
    assert_eq!(finished.load(Ordering::SeqCst), 200);
}

type HeldJob = (Priority, Box<dyn FnOnce() + Send>);

/// Holds every job so a test decides when (and whether) it runs.
#[derive(Default)]
struct Held(std::sync::Mutex<Vec<HeldJob>>);

impl Spawn for Held {
    fn spawn(&self, priority: Priority, job: Box<dyn FnOnce() + Send>) {
        self.0.lock().unwrap().push((priority, job));
    }
}

#[test]
fn load_with_hands_its_priority_to_the_spawner() {
    let spawner = Arc::new(Held::default());
    let loader = loader(spawner.clone());
    let _plain = loader.load(AssetPath::new("a.bin"), Sum);
    let _urgent = loader.load_with(Priority::High, AssetPath::new("a.bin"), Sum);

    let queued: Vec<Priority> = spawner.0.lock().unwrap().iter().map(|j| j.0).collect();
    assert_eq!(queued, [Priority::Normal, Priority::High]);
}

#[test]
fn a_cancelled_load_is_skipped_and_still_resolves() {
    let spawner = Arc::new(Held::default());
    let pending = loader(spawner.clone()).load(AssetPath::new("a.bin"), Sum);
    pending.cancel();

    let (_, job) = spawner.0.lock().unwrap().pop().unwrap();
    job();

    assert_eq!(pending.wait(), Err(LoadError::Cancelled));
}

#[test]
fn cancelling_after_the_job_ran_keeps_the_result() {
    let pending = loader(Arc::new(Inline)).load(AssetPath::new("a.bin"), Sum);
    pending.cancel();
    assert_eq!(pending.wait(), Ok(6));
}
