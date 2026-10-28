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

/// The CRC-32 of `bytes`.
pub(in crate::pack) fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        let index = usize::from(crc.to_le_bytes()[0] ^ byte);
        crc = (crc >> 8) ^ TABLE[index];
    }
    !crc
}
