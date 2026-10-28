//! The streaming writer writes the same bytes as the in-memory one, and
//! fails without ever leaving something that reads as a pack.

use super::support::{id, path, sample};
use crate::pack::layout::{Crc32, crc32};
use crate::pack::*;

/// The assets of [`sample`], plus a larger one, as (id, path, bytes).
fn assets() -> Vec<(u64, &'static str, Vec<u8>)> {
    vec![
        (
            1,
            "meshes/crate.obj",
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n".to_vec(),
        ),
        (2, "empty.bin", Vec::new()),
        (3, "textures/grass.png", vec![7_u8; 300]),
        (
            4,
            "big.bin",
            (0..70_000_u32).map(|n| (n % 251) as u8).collect(),
        ),
    ]
}

fn in_memory(assets: &[(u64, &str, Vec<u8>)]) -> Vec<u8> {
    let mut writer = PackWriter::new();
    for (bits, name, bytes) in assets {
        writer.add(id(*bits), path(name), bytes).unwrap();
    }
    writer.finish()
}

fn streamed(assets: &[(u64, &str, Vec<u8>)], chunk: usize) -> Vec<u8> {
    let mut writer = StreamingPackWriter::new(Vec::new()).unwrap();
    for (bits, name, bytes) in assets {
        writer.begin(id(*bits), path(name)).unwrap();
        for part in bytes.chunks(chunk) {
            writer.write(part).unwrap();
        }
        writer.end().unwrap();
    }
    writer.finish().unwrap()
}

#[test]
fn the_bytes_equal_the_in_memory_writer_whatever_the_chunking() {
    let assets = assets();
    let expected = in_memory(&assets);
    for chunk in [1, 7, 256, 4096, 100_000] {
        assert_eq!(streamed(&assets, chunk), expected, "chunk {chunk}");
    }
}

#[test]
fn add_is_begin_write_end() {
    let assets = assets();
    let mut writer = StreamingPackWriter::new(Vec::new()).unwrap();
    for (bits, name, bytes) in &assets {
        writer.add(id(*bits), path(name), bytes).unwrap();
    }
    assert_eq!(writer.len(), 4);
    assert_eq!(writer.finish().unwrap(), in_memory(&assets));
}

#[test]
fn the_sample_pack_and_the_empty_pack_match() {
    let mut writer = StreamingPackWriter::new(Vec::new()).unwrap();
    assert!(writer.is_empty());
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
    assert_eq!(writer.finish().unwrap(), sample());

    let empty = StreamingPackWriter::new(Vec::new())
        .unwrap()
        .finish()
        .unwrap();
    assert_eq!(empty, PackWriter::new().finish());
}

#[test]
fn a_streamed_pack_reads_back_and_verifies() {
    let assets = assets();
    let bytes = streamed(&assets, 1000);
    let reader = PackReader::open(&bytes[..]).unwrap();
    reader.verify().unwrap();
    assert_eq!(reader.read_id(id(3)).unwrap(), vec![7_u8; 300]);
}

#[test]
fn a_duplicate_is_refused_before_anything_is_written() {
    let mut writer = StreamingPackWriter::new(Vec::new()).unwrap();
    writer.add(id(1), path("a"), b"first").unwrap();
    assert!(matches!(
        writer.begin(id(1), path("b")),
        Err(PackWriteError::Build(PackBuildError::DuplicateId(_)))
    ));
    assert!(matches!(
        writer.begin(id(2), path("a")),
        Err(PackWriteError::Build(PackBuildError::DuplicatePath(_)))
    ));
    writer.add(id(2), path("b"), b"second").unwrap();
    let expected = {
        let mut w = PackWriter::new();
        w.add(id(1), path("a"), b"first").unwrap();
        w.add(id(2), path("b"), b"second").unwrap();
        w.finish()
    };
    assert_eq!(writer.finish().unwrap(), expected);
}

#[test]
fn misuse_is_a_typed_error() {
    let mut writer = StreamingPackWriter::new(Vec::new()).unwrap();
    assert_eq!(writer.write(b"x"), Err(PackWriteError::NoAssetOpen));
    assert_eq!(writer.end(), Err(PackWriteError::NoAssetOpen));
    writer.begin(id(1), path("a")).unwrap();
    assert_eq!(
        writer.begin(id(2), path("b")),
        Err(PackWriteError::AssetOpen)
    );
    assert!(matches!(writer.finish(), Err(PackWriteError::AssetOpen)));
}

/// Takes `limit` writes, then fails every one; keeps what it took.
struct Failing {
    bytes: Vec<u8>,
    limit: usize,
}

impl WriteAt for Failing {
    fn write_at(&mut self, offset: u64, bytes: &[u8]) -> Result<(), WriteAtError> {
        if self.limit == 0 {
            return Err(WriteAtError::Failed {
                reason: "disk full".into(),
            });
        }
        self.limit -= 1;
        self.bytes.write_at(offset, bytes)
    }
}

fn write_three(sink: &mut Failing) -> Result<(), PackWriteError> {
    let mut writer = StreamingPackWriter::new(sink)?;
    for (bits, name, bytes) in assets().iter().take(3) {
        writer.add(id(*bits), path(name), bytes)?;
    }
    writer.finish().map(drop)
}

#[test]
fn a_sink_that_fails_anywhere_is_an_error_and_leaves_no_valid_pack() {
    // Reserved header, three payloads, the index and the header: 6 writes.
    for limit in 0..6 {
        let mut sink = Failing {
            bytes: Vec::new(),
            limit,
        };
        let error = write_three(&mut sink).unwrap_err();
        assert!(matches!(error, PackWriteError::Sink(_)), "limit {limit}");
        assert!(
            PackReader::open(&sink.bytes[..]).is_err(),
            "limit {limit} left a readable pack"
        );
    }
    let mut sink = Failing {
        bytes: Vec::new(),
        limit: 6,
    };
    write_three(&mut sink).unwrap();
    PackReader::open(&sink.bytes[..]).unwrap().verify().unwrap();
}

#[test]
fn memory_sinks_refuse_a_gap() {
    let mut sink = vec![0_u8; 4];
    assert!(matches!(
        sink.write_at(9, b"x"),
        Err(WriteAtError::Gap { .. })
    ));
    sink.write_at(2, b"abcd").unwrap();
    assert_eq!(sink, [0, 0, b'a', b'b', b'c', b'd']);
    assert!(matches!(
        sink.write_at(u64::MAX, b"x"),
        Err(WriteAtError::Gap { .. })
    ));
}

#[test]
fn an_incremental_checksum_equals_the_whole() {
    let data: Vec<u8> = (0..1000_u32)
        .map(|n| u8::try_from(n * 7 % 256).unwrap())
        .collect();
    for split in [0, 1, 499, 999, 1000] {
        let mut sum = Crc32::new();
        sum.update(&data[..split]);
        sum.update(&data[split..]);
        assert_eq!(sum.finish(), crc32(&data), "split {split}");
    }
}

#[test]
fn a_streamed_pack_truncated_or_flipped_anywhere_is_an_error() {
    let pack = streamed(&assets()[..3], 64);
    let check = |bytes: &[u8]| PackReader::open(bytes)?.verify();
    for cut in 0..pack.len() {
        assert!(check(&pack[..cut]).is_err(), "cut at {cut}");
    }
    for at in 0..pack.len() {
        let mut damaged = pack.clone();
        damaged[at] ^= 0x55;
        assert!(check(&damaged).is_err(), "byte {at}");
    }
}
