//! Truncating and flipping bytes of a pack: nothing panics, nothing
//! allocates what the file does not hold, and every damage is found.

use super::support::{id, sample};
use crate::pack::*;

/// Opens and verifies; `Ok` only when every byte read back is intact.
fn check(bytes: &[u8]) -> Result<(), PackError> {
    PackReader::open(bytes)?.verify()
}

#[test]
fn every_truncation_is_an_error() {
    let bytes = sample();
    for cut in 0..bytes.len() {
        assert!(check(&bytes[..cut]).is_err(), "cut at {cut} went unnoticed");
    }
    assert!(check(&bytes).is_ok());
}

#[test]
fn every_flipped_byte_is_found_by_open_or_verify() {
    let bytes = sample();
    for at in 0..bytes.len() {
        for flip in [0x01_u8, 0x10, 0x80, 0xff] {
            let mut damaged = bytes.clone();
            damaged[at] ^= flip;
            assert!(
                check(&damaged).is_err(),
                "byte {at} ^ {flip:#x} went unnoticed"
            );
        }
    }
}

#[test]
fn appended_bytes_are_a_bad_layout() {
    let mut bytes = sample();
    bytes.push(0);
    assert!(matches!(
        PackReader::open(bytes).unwrap_err(),
        PackError::BadLayout { .. }
    ));
}

#[test]
fn a_damaged_asset_does_not_hide_the_others() {
    let mut bytes = sample();
    let start = PackReader::open(&bytes[..])
        .unwrap()
        .entry_by_id(id(3))
        .unwrap()
        .offset;
    let hit = usize::try_from(start).unwrap() + 10;
    bytes[hit] ^= 0xff;
    let damaged = PackReader::open(&bytes[..]).unwrap();
    assert!(damaged.read_id(id(1)).is_ok());
    let error = damaged.read_id(id(3)).unwrap_err();
    assert!(
        matches!(&error, PackError::PayloadChecksum { offset, len: 300, .. } if *offset == start),
        "{error}"
    );
}

#[test]
fn random_garbage_with_a_valid_looking_header_never_panics() {
    // A simple xorshift stream: reproducible, no dependency.
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut below = |n: usize| usize::try_from(next() % n as u64).unwrap_or(0);
    let good = sample();
    for _ in 0..2000 {
        let mut bytes = good.clone();
        for _ in 0..=below(6) {
            let at = below(bytes.len());
            bytes[at] = u8::try_from(below(256)).unwrap_or(0);
        }
        if below(3) == 0 {
            let cut = below(bytes.len());
            bytes.truncate(cut);
        }
        drop(check(&bytes));
    }
}
