# `meridian-gpu-driver` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `lib`

**Async on genuine I/O, not on everything** — same policy as every
other driver crate in this workspace (see
[ADR 009](../adr/009-async-io-via-tokio.md)): [`Device::new`]/
[`Device::new_windowed`] (an OS/driver handshake) and
[`Device::read_buffer`] (waiting on in-flight GPU work) are real
`async fn`s; allocation/recording calls stay synchronous.
