//! Packs that must not open, and why.

use super::support::{fix_header_crc, sample, with_index};
use crate::pack::index::IndexDocument;
use crate::pack::layout::HEADER_LEN;
use crate::pack::*;

#[test]
fn a_newer_version_is_unsupported_and_version_zero_too() {
    for version in [PACK_VERSION + 1, 0, u16::MAX] {
        let mut bytes = sample();
        bytes[4..6].copy_from_slice(&version.to_le_bytes());
        assert_eq!(
            PackReader::open(bytes).unwrap_err(),
            PackError::UnsupportedVersion { found: version }
        );
    }
}

#[test]
fn unknown_flags_are_refused_even_with_a_right_checksum() {
    let mut bytes = sample();
    bytes[6] = 1;
    fix_header_crc(&mut bytes);
    assert_eq!(
        PackReader::open(bytes).unwrap_err(),
        PackError::UnknownFlags { flags: 1 }
    );
}

#[test]
fn files_that_are_not_packs_say_so() {
    assert!(matches!(
        PackReader::open(Vec::new()),
        Err(PackError::TooShort { size: 0, .. })
    ));
    assert!(matches!(
        PackReader::open(vec![0_u8; 31]),
        Err(PackError::TooShort { .. })
    ));
    assert_eq!(
        PackReader::open(vec![0_u8; 64]).unwrap_err(),
        PackError::BadMagic
    );
}

#[test]
fn an_index_that_lies_about_its_place_is_a_bad_layout_without_a_huge_allocation() {
    let mut bytes = sample();
    bytes[16..24].copy_from_slice(&(u64::MAX - 5).to_le_bytes());
    fix_header_crc(&mut bytes);
    assert!(matches!(
        PackReader::open(bytes).unwrap_err(),
        PackError::BadLayout { .. }
    ));
}

#[test]
fn an_index_larger_than_the_limit_is_refused_before_it_is_read() {
    let mut bytes = sample();
    // A header claiming the index starts at 32 and ends the file, with the
    // file made to look longer than the limit by the source, not the data.
    bytes[8..16].copy_from_slice(&(HEADER_LEN as u64).to_le_bytes());
    bytes[16..24].copy_from_slice(&(MAX_INDEX_LEN + 1).to_le_bytes());
    fix_header_crc(&mut bytes);
    assert!(matches!(
        PackReader::open(Huge(bytes)).unwrap_err(),
        PackError::IndexTooLarge { .. }
    ));
}

#[derive(Debug)]
struct Huge(Vec<u8>);

impl ReadAt for Huge {
    fn size(&self) -> Result<u64, ReadAtError> {
        Ok(MAX_INDEX_LEN + 1 + HEADER_LEN as u64)
    }
    fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> Result<(), ReadAtError> {
        self.0.read_exact_at(offset, buf)
    }
}

#[test]
fn entries_outside_the_payload_or_repeated_are_refused() {
    let entries = PackReader::open(sample()).unwrap().entries().to_vec();

    let mut outside = entries.clone();
    outside[0].len = u64::MAX;
    assert!(matches!(
        open_with_entries(&outside),
        PackError::EntryOutOfBounds { entry: 0, .. }
    ));

    let mut into_header = entries.clone();
    into_header[1].offset = 4;
    assert!(matches!(
        open_with_entries(&into_header),
        PackError::EntryOutOfBounds { entry: 1, .. }
    ));

    let mut same_id = entries.clone();
    same_id[2].id = same_id[0].id;
    assert!(matches!(
        open_with_entries(&same_id),
        PackError::DuplicateId {
            first: 0,
            second: 2,
            ..
        }
    ));

    let mut same_path = entries;
    same_path[1].path = same_path[0].path.clone();
    assert!(matches!(
        open_with_entries(&same_path),
        PackError::DuplicatePath {
            first: 0,
            second: 1,
            ..
        }
    ));
}

#[test]
fn an_index_that_is_not_an_index_has_paths() {
    let junk = substrate_data::binary::encode(&substrate_data::Value::Int(3));
    let error = PackReader::open(with_index(&sample(), &junk)).unwrap_err();
    assert!(matches!(error, PackError::IndexInvalid(_)), "{error}");

    let missing =
        substrate_data::binary::encode(&substrate_data::Value::Map(substrate_data::Map::default()));
    let PackError::IndexInvalid(problems) =
        PackReader::open(with_index(&sample(), &missing)).unwrap_err()
    else {
        panic!("expected IndexInvalid");
    };
    assert!(
        problems[0].to_string().contains("entries"),
        "{}",
        problems[0]
    );

    let garbage = PackReader::open(with_index(&sample(), b"SDAT\x02\xee\xee")).unwrap_err();
    assert!(
        matches!(garbage, PackError::IndexMalformed { offset, .. } if offset >= 32),
        "{garbage}"
    );

    // A list that claims 2^60 items and has none is malformed, not allocated.
    let mut huge = b"SDAT\x02\x06".to_vec();
    huge.extend_from_slice(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x10]);
    let claim = PackReader::open(with_index(&sample(), &huge)).unwrap_err();
    assert!(matches!(claim, PackError::IndexMalformed { .. }), "{claim}");
}

#[test]
fn a_foreign_entry_is_refused_not_read() {
    let reader = PackReader::open(sample()).unwrap();
    let mut foreign = reader.entries()[0].clone();
    foreign.offset = 1 << 40;
    assert!(matches!(
        reader.read_entry(&foreign),
        Err(PackError::EntryOutOfBounds { .. })
    ));
}

fn open_with_entries(entries: &[PackEntry]) -> PackError {
    let index = substrate_data::to_bytes(&IndexDocument::of(entries));
    PackReader::open(with_index(&sample(), &index)).unwrap_err()
}
