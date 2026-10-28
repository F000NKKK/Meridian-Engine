//! Streaming a pack to a file.

use std::fs;

use meridian_asset_domain::{
    AssetId, PackPath, PackReader, PackWriteError, PackWriter, StreamingPackWriter, WriteAtError,
};

use super::{AddFileError, FileReadAt, FileWriteAt, add_file};
use crate::scratch::Scratch;

fn pack_path(text: &str) -> PackPath {
    PackPath::new(text).expect("test path")
}

#[test]
fn a_streamed_pack_file_equals_the_in_memory_pack() {
    let scratch = Scratch::new("stream-pack");
    // Bigger than two chunks, so the copy loop runs more than once.
    let big: Vec<u8> = (0..150_000_u32).map(|n| (n % 253) as u8).collect();
    fs::write(scratch.0.join("big.bin"), &big).expect("write");
    fs::write(scratch.0.join("empty.bin"), []).expect("write");
    fs::write(scratch.0.join("small.txt"), b"hello").expect("write");

    let out = scratch.0.join("out.mpak");
    let mut writer =
        StreamingPackWriter::new(FileWriteAt::create(&out).expect("create")).expect("header");
    for (bits, name) in [(1, "big.bin"), (2, "empty.bin"), (3, "small.txt")] {
        add_file(
            &mut writer,
            AssetId::from_bits(bits),
            pack_path(name),
            &scratch.0.join(name),
        )
        .expect("added");
    }
    writer.finish().expect("finished").sync().expect("synced");

    let mut expected = PackWriter::new();
    expected
        .add(AssetId::from_bits(1), pack_path("big.bin"), &big)
        .expect("add");
    expected
        .add(AssetId::from_bits(2), pack_path("empty.bin"), b"")
        .expect("add");
    expected
        .add(AssetId::from_bits(3), pack_path("small.txt"), b"hello")
        .expect("add");
    assert_eq!(fs::read(&out).expect("read"), expected.finish());

    let reader = PackReader::open(FileReadAt::open(&out).expect("open")).expect("pack");
    reader.verify().expect("intact");
    assert_eq!(
        reader.read_id(AssetId::from_bits(3)).expect("asset"),
        b"hello"
    );
}

#[test]
fn a_missing_source_file_is_a_read_error_and_a_duplicate_a_pack_error() {
    let scratch = Scratch::new("stream-errors");
    fs::write(scratch.0.join("a.bin"), b"a").expect("write");
    let out = scratch.0.join("out.mpak");
    let mut writer =
        StreamingPackWriter::new(FileWriteAt::create(&out).expect("create")).expect("header");

    let missing = add_file(
        &mut writer,
        AssetId::from_bits(1),
        pack_path("x"),
        &scratch.0.join("nope.bin"),
    );
    assert!(matches!(missing, Err(AddFileError::Read { .. })));

    let file = scratch.0.join("a.bin");
    add_file(&mut writer, AssetId::from_bits(1), pack_path("a"), &file).expect("added");
    let again = add_file(&mut writer, AssetId::from_bits(1), pack_path("b"), &file);
    assert!(matches!(
        again,
        Err(AddFileError::Pack(PackWriteError::Build(_)))
    ));
}

#[test]
fn an_unfinished_pack_file_is_not_a_pack() {
    let scratch = Scratch::new("stream-unfinished");
    let out = scratch.0.join("out.mpak");
    let mut writer =
        StreamingPackWriter::new(FileWriteAt::create(&out).expect("create")).expect("header");
    writer
        .add(AssetId::from_bits(1), pack_path("a"), b"data")
        .expect("added");
    drop(writer);

    let opened = PackReader::open(FileReadAt::open(&out).expect("open"));
    assert!(opened.is_err());
}

#[test]
fn a_pack_file_cannot_be_created_in_a_missing_directory() {
    let scratch = Scratch::new("stream-nodir");
    let error = FileWriteAt::create(scratch.0.join("no/such/dir/out.mpak")).expect_err("no dir");
    assert!(matches!(error, WriteAtError::Failed { .. }));
}
