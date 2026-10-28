//! Packs and damaged packs for the tests.

#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "the version 1 writer below packs small test numbers"
)]

use crate::AssetId;
use crate::pack::layout::crc32;
use crate::pack::{PackEntry, PackPath, PackWriter};

pub(super) fn id(bits: u64) -> AssetId {
    AssetId::from_bits(bits)
}

pub(super) fn path(text: &str) -> PackPath {
    PackPath::new(text).unwrap()
}

/// Three assets: a mesh, an empty file and a texture.
pub(super) fn sample() -> Vec<u8> {
    let mut writer = PackWriter::new();
    writer
        .add(
            id(1),
            path("meshes/crate.obj"),
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )
        .unwrap();
    writer.add(id(2), path("empty.bin"), b"").unwrap();
    writer
        .add(id(3), path("textures/grass.png"), &[7_u8; 300])
        .unwrap();
    writer.finish()
}

pub(super) fn fix_header_crc(bytes: &mut [u8]) {
    let crc = crc32(&bytes[..28]);
    bytes[28..32].copy_from_slice(&crc.to_le_bytes());
}

/// `bytes` with its index replaced by `index` (header fixed up).
pub(super) fn with_index(bytes: &[u8], index: &[u8]) -> Vec<u8> {
    let start = usize::try_from(u64::from_le_bytes(bytes[8..16].try_into().unwrap())).unwrap();
    let mut out = bytes[..start].to_vec();
    out.extend_from_slice(index);
    out[16..24].copy_from_slice(&(index.len() as u64).to_le_bytes());
    out[24..28].copy_from_slice(&crc32(index).to_le_bytes());
    fix_header_crc(&mut out);
    out
}

/// The index as Substrate Data version 1 wrote it: every value tagged, no
/// shapes, no inline ints.
pub(super) fn v1_index(entries: &[PackEntry]) -> Vec<u8> {
    let mut v1 = V1::default();
    v1.out.extend_from_slice(b"SDAT\x01");
    v1.map(1);
    v1.string("entries");
    v1.list(entries.len());
    for e in entries {
        v1.map(5);
        v1.string("id");
        v1.str(&e.id.to_string());
        v1.string("path");
        v1.str(e.path.as_str());
        v1.string("offset");
        v1.int(e.offset as i64);
        v1.string("len");
        v1.int(e.len as i64);
        v1.string("crc32");
        v1.int(i64::from(e.crc32));
    }
    v1.out
}

#[derive(Default)]
struct V1 {
    out: Vec<u8>,
    table: Vec<String>,
}

impl V1 {
    fn varint(&mut self, mut n: u64) {
        while n > 0x7f {
            self.out.push((n & 0x7f) as u8 | 0x80);
            n >>= 7;
        }
        self.out.push(n as u8);
    }
    fn string(&mut self, s: &str) {
        if let Some(i) = self.table.iter().position(|t| t == s) {
            self.varint((i as u64) << 1);
        } else {
            self.varint(((s.len() as u64) << 1) | 1);
            self.out.extend_from_slice(s.as_bytes());
            self.table.push(s.to_owned());
        }
    }
    fn str(&mut self, s: &str) {
        self.out.push(5);
        self.string(s);
    }
    fn int(&mut self, n: i64) {
        self.out.push(3);
        self.varint(((n << 1) ^ (n >> 63)) as u64);
    }
    fn list(&mut self, len: usize) {
        self.out.push(6);
        self.varint(len as u64);
    }
    fn map(&mut self, len: usize) {
        self.out.push(7);
        self.varint(len as u64);
    }
}
