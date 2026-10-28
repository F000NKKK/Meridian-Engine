//! [`PackError`] and [`PackBuildError`]: why a pack did not read or build.

use substrate_data::Diagnostic;
use thiserror::Error;

use super::{PACK_VERSION, PackBuildError, PackPath, ReadAtError};
use crate::AssetId;

/// Why a pack, or an asset in it, did not read. Every variant that points
/// into the file says where (a byte offset from the start of the file).
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum PackError {
    /// The source failed or was asked for bytes it does not have.
    #[error("reading {length} bytes at {offset}: {source}")]
    Read {
        /// Where the read started.
        offset: u64,
        /// How many bytes.
        length: u64,
        /// What the source said.
        source: ReadAtError,
    },
    /// Fewer bytes than the header.
    #[error("not a pack: {size} bytes, the header alone is {needed}")]
    TooShort {
        /// The size of the file.
        size: u64,
        /// The size of the header.
        needed: u64,
    },
    /// The file does not start with `MPAK`.
    #[error("not a pack: it does not start with `MPAK`")]
    BadMagic,
    /// A newer envelope version than this reads (or version 0, which no
    /// writer ever wrote).
    #[error("unsupported pack version {found} (this reads 1 to {PACK_VERSION})")]
    UnsupportedVersion {
        /// The version written.
        found: u16,
    },
    /// The header's own checksum does not match: it is damaged.
    #[error("the pack header is damaged (checksum {computed:08x}, stored {stored:08x})")]
    HeaderChecksum {
        /// What the header stores.
        stored: u32,
        /// What its bytes sum to.
        computed: u32,
    },
    /// Flags this version does not know are set.
    #[error("the pack header sets flags {flags:#06x} this version does not know")]
    UnknownFlags {
        /// The flags written.
        flags: u16,
    },
    /// The index is not where the header says, or is too large.
    #[error(
        "bad pack layout: index of {index_len} bytes at {index_offset}, \
         file of {file_len} bytes (the index must end the file, after the 32-byte header)"
    )]
    BadLayout {
        /// Where the index starts.
        index_offset: u64,
        /// How long it is.
        index_len: u64,
        /// The size of the file.
        file_len: u64,
    },
    /// The index is larger than any this reads.
    #[error("the pack index is {index_len} bytes, more than the {max} this reads")]
    IndexTooLarge {
        /// The index's size.
        index_len: u64,
        /// The most this reads.
        max: u64,
    },
    /// The index's checksum does not match: it is damaged.
    #[error("the pack index at {offset} is damaged (checksum {computed:08x}, stored {stored:08x})")]
    IndexChecksum {
        /// Where the index starts.
        offset: u64,
        /// What the header stores.
        stored: u32,
        /// What its bytes sum to.
        computed: u32,
    },
    /// The index is not the binary form, at the byte that shows it.
    #[error("the pack index: byte {offset}: {message}")]
    IndexMalformed {
        /// What is wrong.
        message: String,
        /// The byte, from the start of the file.
        offset: u64,
    },
    /// The index decodes but is not an index: fields missing, unknown or of
    /// the wrong type, each with its path (`entries[3].path`).
    #[error("the pack index: {}", joined(.0))]
    IndexInvalid(Vec<Diagnostic>),
    /// An entry's bytes are not inside the payload area.
    #[error(
        "entry {entry} (`{path}`): {len} bytes at {offset} are outside the payload \
         (bytes {payload_start} to {payload_end})"
    )]
    EntryOutOfBounds {
        /// The entry's position in the index.
        entry: usize,
        /// Its path.
        path: PackPath,
        /// Where its bytes start.
        offset: u64,
        /// How long they are.
        len: u64,
        /// Where the payload area starts.
        payload_start: u64,
        /// Where it ends (where the index starts).
        payload_end: u64,
    },
    /// Two entries have one id.
    #[error("entries {first} and {second} have the same id {id}")]
    DuplicateId {
        /// The id.
        id: AssetId,
        /// The first entry.
        first: usize,
        /// The second.
        second: usize,
    },
    /// Two entries have one path.
    #[error("entries {first} and {second} have the same path `{path}`")]
    DuplicatePath {
        /// The path.
        path: PackPath,
        /// The first entry.
        first: usize,
        /// The second.
        second: usize,
    },
    /// No entry has this id.
    #[error("no asset {0} in the pack")]
    NoSuchId(AssetId),
    /// No entry has this path.
    #[error("no asset `{0}` in the pack")]
    NoSuchPath(PackPath),
    /// An entry too big to hold in memory on this machine.
    #[error("`{path}` is {len} bytes, more than fits in memory here")]
    TooLargeForMemory {
        /// The entry's path.
        path: PackPath,
        /// Its size.
        len: u64,
    },
    /// Rebuilding the pack failed (not reachable for a pack that opened:
    /// its ids and paths are unique).
    #[error(transparent)]
    Build(#[from] PackBuildError),
    /// An asset's bytes do not match their checksum: they are damaged.
    #[error(
        "`{path}` ({len} bytes at {offset}) is damaged (checksum {computed:08x}, stored {stored:08x})"
    )]
    PayloadChecksum {
        /// The entry's path.
        path: PackPath,
        /// Where its bytes start.
        offset: u64,
        /// How many.
        len: u64,
        /// What the index stores.
        stored: u32,
        /// What the bytes sum to.
        computed: u32,
    },
}

fn joined(problems: &[Diagnostic]) -> String {
    let lines: Vec<String> = problems.iter().map(ToString::to_string).collect();
    lines.join("; ")
}
