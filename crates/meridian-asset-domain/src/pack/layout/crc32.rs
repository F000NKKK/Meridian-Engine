//! CRC-32 (IEEE 802.3, the zlib/PNG polynomial) of a byte slice.

const POLYNOMIAL: u32 = 0xEDB8_8320;

const TABLE: [u32; 256] = table();

const fn table() -> [u32; 256] {
    let mut table = [0_u32; 256];
    let mut n = 0;
    while n < 256 {
        #[expect(clippy::cast_possible_truncation, reason = "n < 256")]
        let mut crc = n as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ POLYNOMIAL
            } else {
                crc >> 1
            };
            bit += 1;
        }
        table[n] = crc;
        n += 1;
    }
    table
}

/// A CRC-32 being computed over bytes that arrive in pieces. Feeding a
/// slice in any number of parts gives the same sum as [`crc32`] of the whole.
#[derive(Debug, Clone, Copy)]
pub(in crate::pack) struct Crc32(u32);

impl Crc32 {
    /// A sum over no bytes yet.
    pub(in crate::pack) fn new() -> Self {
        Crc32(u32::MAX)
    }

    /// Adds `bytes`.
    pub(in crate::pack) fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            let index = usize::from(self.0.to_le_bytes()[0] ^ byte);
            self.0 = (self.0 >> 8) ^ TABLE[index];
        }
    }

    /// The sum of everything added.
    pub(in crate::pack) fn finish(self) -> u32 {
        !self.0
    }
}

/// The CRC-32 of `bytes`.
pub(in crate::pack) fn crc32(bytes: &[u8]) -> u32 {
    let mut sum = Crc32::new();
    sum.update(bytes);
    sum.finish()
}
