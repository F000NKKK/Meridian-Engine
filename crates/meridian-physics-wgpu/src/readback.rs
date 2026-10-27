//! Reading words back out of GPU readback buffers.

/// The four bytes of the little-endian word at `at` in a GPU readback.
/// Readback buffers are sized by the dispatch that wrote them, so `at + 4`
/// is in range for every index the caller computes from that size.
pub(crate) fn le_word(bytes: &[u8], at: usize) -> [u8; 4] {
    [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]
}
