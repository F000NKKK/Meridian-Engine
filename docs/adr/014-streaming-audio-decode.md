# ADR 014: Streaming audio decode, strategy-configured, Iterator-shaped

## Status

Location update: streaming decode lives in `meridian-asset-symphonia`
([ADR 024](024-wave-5-ports-and-adapters.md)); the decision below is
unchanged.

Accepted — implements the "streaming decode is a real future concern"
deferral recorded in [ADR 013](013-compressed-audio-codecs.md)'s
consequences.

## Context

ADR 013's decoders are eager: the whole track decodes into one
`AudioData`. Decoded PCM is ~10× the compressed size — the repo's own
91-second demo track is 1.5 MB compressed and ~17.5 MB decoded; real
background music measured in minutes is hundreds of megabytes. A music
player that holds three tracks eagerly pays for all of them at once.

## Decision

**Incremental *decode*, not incremental I/O.** `asset-core` keeps its
bytes-in/data-out contract: the compressed bytes still arrive complete
and in memory; what streams is the decode itself —
`StreamingAudioDecoder` holds the open symphonia demux/decode pipeline
and yields bounded interleaved-`i16` blocks on demand, so resident
memory is compressed-size + one block instead of the full PCM. File/
network I/O streaming is a different, orthogonal concern (it would put
I/O into a crate that deliberately has none); if it ever lands, it
enters through symphonia's own `MediaSource` seam (which already
accepts any `Read + Seek`), not through a redesign of this API.

**One thin loading interface, resolved by configuration.**
`open_audio(bytes, &DecodeStrategy) -> AudioAsset` is the front door
over both paths; callers that don't care about the split never touch
per-format decoders. `DecodeStrategy` is the whole policy surface:

- `mode: Auto | ForceEager | ForceStreaming` — force either path
  unconditionally, or let `Auto` decide;
- `auto_threshold_bytes` — `Auto` streams once the *estimated decoded
  size* (from container metadata: declared frame count × channels × 2)
  exceeds it; a track whose length the container doesn't declare always
  streams, because an unbounded decode can't be safely eager;
- `block_frames` — the streaming buffer size (frames per
  `next_block`).

The probe happens once: `Auto`/`ForceEager` drain the already-open
pipeline (`decode_all`) rather than probing twice.

**`Iterator` is the streaming abstraction, deliberately.** Rust's
standard pull-streaming shape for CPU-bound work is `Iterator`
(`futures::Stream` is async and buys nothing for decoding in-memory
bytes), so `StreamingAudioDecoder` implements
`Iterator<Item = Result<Vec<i16>, DecodeError>>`. No engine-specific
`AssetStream` trait is invented with a single implementor: a future
video pipeline gets its own sibling module (`audio/`'s naming —
`audio_streaming` — leaves the `video/` namespace free), and a shared
cross-asset trait is extracted only when that second concrete case
exists to shape it. Texture/mip streaming is explicitly *not* that
case — it's GPU residency policy, which ADRs 002/006 place outside
`asset-core` entirely.

**Looping is `rewind()`, and the bytes are shared.** The compressed
input lives in an `Arc<[u8]>`; rewinding rebuilds the decode pipeline
over the same shared bytes with no copy — the loop-playback primitive
`music_sphere` uses.

## Alternatives considered

- **Always stream** — rejected: for short SFX (the common case) a
  single `AudioData` is simpler for every caller and the memory
  argument vanishes; the strategy default (`Auto`, 16 MiB) gives small
  assets the simple path automatically.
- **A bespoke engine `AssetStream`/`BlockSource` trait now** —
  rejected as speculative generality (one implementor); see above.
- **Async streaming (`futures::Stream`)** — rejected: decode of
  in-memory bytes is bounded CPU work, exactly what ADR 009 says must
  *not* be async.

## Consequences

- `AudioAsset` is a two-armed enum; playback code that supports both
  paths matches once at load time (see `music_sphere`'s `Track`).
- Verified equivalence: streamed blocks reassemble bit-exactly to the
  eager decode for both WAV and MP3 (test-enforced).
- `Auto` mode's unknown-length rule means raw-frame-sync MP3s without
  metadata stream even when tiny — safe, mildly conservative, and
  overridable with `ForceEager`.
