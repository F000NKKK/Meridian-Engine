//! A CPU-side compute buffer.

/// A CPU-backend compute buffer: owned bytes. A GPU backend has its own
/// device-memory handles; the two are different representations of different
/// backends and are not unified here.
#[derive(Debug, Clone, Default)]
pub struct ComputeBuffer {
    data: Vec<u8>,
}

impl ComputeBuffer {
    /// A buffer of `byte_len` zero bytes.
    #[must_use]
    pub fn zeroed(byte_len: usize) -> Self {
        Self {
            data: vec![0u8; byte_len],
        }
    }

    /// A buffer holding `data`.
    #[must_use]
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// The buffer's length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the buffer holds no bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The bytes, read-only.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// The bytes, writable.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeroed_has_requested_length_and_zero_bytes() {
        let buffer = ComputeBuffer::zeroed(16);
        assert_eq!(buffer.len(), 16);
        assert!(buffer.as_slice().iter().all(|&byte| byte == 0));
    }

    #[test]
    fn mutable_slice_writes_are_visible() {
        let mut buffer = ComputeBuffer::zeroed(4);
        buffer.as_mut_slice()[2] = 42;
        assert_eq!(buffer.as_slice(), &[0, 0, 42, 0]);
    }

    #[test]
    fn from_bytes_keeps_the_bytes_and_empty_is_empty() {
        assert_eq!(ComputeBuffer::from_bytes(vec![1, 2]).as_slice(), &[1, 2]);
        assert!(ComputeBuffer::default().is_empty());
    }
}
