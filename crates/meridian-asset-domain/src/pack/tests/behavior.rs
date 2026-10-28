//! Writing packs, reading assets back, and upgrading.

use std::cell::Cell;

use super::support::{id, path, sample, v1_index, with_index};
use crate::pack::layout::{HEADER_LEN, crc32};
use crate::pack::*;

#[test]
fn crc32_matches_the_standard_check_value() {
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    assert_eq!(crc32(b""), 0);
}

#[test]
fn a_pack_starts_with_its_magic_and_version() {
    let bytes = sample();
    assert_eq!(&bytes[..4], b"MPAK");
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), PACK_VERSION);
    assert_eq!(&bytes[6..8], &[0, 0]);
}

#[test]
fn assets_read_back_by_id_and_by_path() {
    let reader = PackReader::open(sample()).unwrap();
    assert_eq!(reader.entries().len(), 3);
    assert_eq!(reader.read_id(id(2)).unwrap(), b"");
    assert_eq!(reader.read_id(id(3)).unwrap(), vec![7_u8; 300]);
    let by_path = reader.read_path(&path("meshes/crate.obj")).unwrap();
    assert!(by_path.starts_with(b"v 0 0 0"));
    assert_eq!(
        reader.entry_by_id(id(1)).unwrap().path.as_str(),
        "meshes/crate.obj"
    );
    assert_eq!(reader.entry_by_path(&path("empty.bin")).unwrap().id, id(2));
    reader.verify().unwrap();
}

#[test]
fn an_empty_pack_opens() {
    let reader = PackReader::open(PackWriter::new().finish()).unwrap();
    assert_eq!(reader.entries(), []);
    reader.verify().unwrap();
}

#[test]
fn unknown_assets_are_typed_errors() {
    let reader = PackReader::open(sample()).unwrap();
    assert_eq!(reader.read_id(id(9)), Err(PackError::NoSuchId(id(9))));
    assert_eq!(
        reader.read_path(&path("nope")),
        Err(PackError::NoSuchPath(path("nope")))
    );
}

#[test]
fn the_writer_refuses_a_repeated_id_or_path_and_adds_nothing() {
    let mut writer = PackWriter::new();
    writer.add(id(1), path("a"), b"a").unwrap();
    assert_eq!(
        writer.add(id(1), path("b"), b"b"),
        Err(PackBuildError::DuplicateId(id(1)))
    );
    assert_eq!(
        writer.add(id(2), path("a"), b"b"),
        Err(PackBuildError::DuplicatePath(path("a")))
    );
    assert_eq!(writer.len(), 1);
    let reader = PackReader::open(writer.finish()).unwrap();
    assert_eq!(reader.entries().len(), 1);
}

/// A source that counts the bytes it hands out.
struct Counting<'a> {
    bytes: &'a [u8],
    served: Cell<u64>,
}

impl ReadAt for Counting<'_> {
    fn size(&self) -> Result<u64, ReadAtError> {
        self.bytes.size()
    }
    fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ReadAtError> {
        self.served.set(self.served.get() + buf.len() as u64);
        self.bytes.read_exact_at(offset, buf)
    }
}

#[test]
fn opening_and_reading_one_asset_does_not_read_the_whole_file() {
    let mut writer = PackWriter::new();
    writer.add(id(1), path("small"), b"tiny").unwrap();
    writer
        .add(id(2), path("big"), &vec![1_u8; 1_000_000])
        .unwrap();
    let bytes = writer.finish();
    let source = Counting {
        bytes: &bytes,
        served: Cell::new(0),
    };
    let reader = PackReader::open(&source).unwrap();
    assert!(
        source.served.get() < 1_000,
        "open read {} bytes",
        source.served.get()
    );
    assert_eq!(reader.read_id(id(1)).unwrap(), b"tiny");
    assert!(
        source.served.get() < 1_000,
        "one small asset read {} bytes",
        source.served.get()
    );
}

#[test]
fn upgrading_a_current_pack_gives_the_same_bytes() {
    let bytes = sample();
    assert_eq!(upgrade_pack(&bytes[..]).unwrap(), bytes);
}

#[test]
fn a_pack_with_a_version_1_index_reads_and_upgrades_losslessly() {
    let bytes = sample();
    let reader = PackReader::open(&bytes[..]).unwrap();
    let old = with_index(&bytes, &v1_index(reader.entries()));
    let older = PackReader::open(&old[..]).unwrap();
    assert_eq!(older.entries(), reader.entries());
    assert_eq!(older.read_id(id(3)).unwrap(), vec![7_u8; 300]);
    assert_ne!(old, bytes, "the old index is spelled differently");
    assert_eq!(
        upgrade_pack(&old[..]).unwrap(),
        bytes,
        "writers write the newest"
    );
}

#[test]
fn a_damaged_pack_does_not_upgrade() {
    let mut bytes = sample();
    bytes[HEADER_LEN + 2] ^= 1;
    assert!(matches!(
        upgrade_pack(&bytes[..]),
        Err(PackError::PayloadChecksum { .. })
    ));
}

#[test]
fn pack_paths_are_checked() {
    for good in ["a", "a/b.c", "meshes/crate.obj", "ünï/ok"] {
        assert!(PackPath::new(good).is_ok(), "{good}");
    }
    for bad in [
        "", "/a", "a/", "a//b", "./a", "a/../b", "a\\b", "a\nb", "..",
    ] {
        assert!(PackPath::new(bad).is_err(), "{bad:?}");
    }
    assert!(PackPath::new(&"a".repeat(1025)).is_err());
}
