//! The behaviour every [`ComputeDevice`](crate::ComputeDevice) must have.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

use crate::ComputeDevice;

/// Asserts the device contract against fresh devices from `make`: at least
/// one thread; zero-filled buffers; no call for `count == 0`; and for any
/// count, the ranges handed to `work` are disjoint, in bounds, and cover every
/// index exactly once.
///
/// # Panics
///
/// Panics when `D` breaks the contract.
pub fn run_device_contract<D: ComputeDevice>(make: impl Fn() -> D) {
    let device = make();
    assert!(device.support().threads >= 1, "at least one thread");
    let buffer = device.allocate_buffer(16);
    assert_eq!(buffer.len(), 16);
    assert!(buffer.as_slice().iter().all(|&byte| byte == 0));

    let calls = AtomicUsize::new(0);
    device.dispatch_ranges(0, &|_| {
        calls.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(calls.load(Ordering::SeqCst), 0, "no work for count 0");

    for count in [1, 7, 4096] {
        assert_covers_exactly_once(&device, count);
    }
}

fn assert_covers_exactly_once(device: &impl ComputeDevice, count: usize) {
    let seen = Mutex::new(vec![0u8; count]);
    device.dispatch_ranges(count, &|range| {
        assert!(range.end <= count, "range {range:?} is out of bounds");
        let mut seen = seen.lock().unwrap_or_else(PoisonError::into_inner);
        for index in range {
            seen[index] += 1;
        }
    });
    let seen = seen.lock().unwrap_or_else(PoisonError::into_inner);
    assert!(
        seen.iter().all(|&visits| visits == 1),
        "every index of {count} is visited exactly once"
    );
}
