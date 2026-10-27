# Editor protocol

What a running Meridian game and its editor (Meridian IDLE) say to each
other. The types live in `meridian-editor-protocol`
([dependency-rules.md](dependency-rules.md): above `meridian-scene`, below
`engine-core`); the formats under them are
[Substrate Data](data-formats.md) ([ADR 016](adr/016-data-formats-on-substrate-data.md)).

The crate is **pure protocol**: no `async`, no sockets, no I/O beyond a
generic `Read`/`Write`. Who listens on which socket, and the code that
answers requests (it needs the `World`, the `ComponentRegistry` and the
frame loop), is the host's job in `engine-core` — not written yet.

## Wire

Each message is one Substrate Data frame on a byte stream
(`substrate_data::io::write_frame` / `read_frame`): a little-endian `u32`
length, then the message in Substrate Data's binary form (`SDAT`). Enums are
tagged by variant name (`{ type: "get_component", entity: …, component: … }`)
and structs by field name, so the bytes are self-describing, and a tool with
no code for the message can read it by shape.

`write_message` / `read_message` wrap those two calls and return a typed
`FrameError` instead of an `io::Error` to unwrap:

| `FrameError` | When |
|---|---|
| `Io` | the stream failed |
| `Truncated` | the stream ended inside a frame |
| `TooLong` | a length over `substrate_data::io::MAX_FRAME` (a broken or hostile peer) |
| `Malformed(diagnostics)` | the frame isn't a message of that type: bytes that don't decode, a missing field, or a variant or field this build doesn't know — each a `Diagnostic` with its path |

`read_message` returns `None` where the stream ends cleanly between frames.

## Messages

Editor to host, always a `RequestEnvelope { id: RequestId, request }`:

| `Request` | Fields | Answer |
|---|---|---|
| `Hello` | `version` | `Welcome`, or `Failed(VersionMismatch)` |
| `Describe` | | `Described` |
| `ListEntities` | | `Entities` |
| `GetComponent` | `entity`, `component` | `Component` |
| `SetComponent` | `entity`, `component`, `value` | `Done` |
| `SpawnEntity` | | `Spawned` |
| `DespawnEntity` | `entity` | `Done` |
| `Play`, `Pause`, `Step` | | `Done` |
| `Subscribe`, `Unsubscribe` | `topic` | `Done` |

Host to editor, a `HostMessage`: either `Response { id, response }`, the
answer to the request with that id (so answers may interleave with
events), or `Event { event }`.

| `Response` | Carries |
|---|---|
| `Welcome` | the negotiated `version`: the lower of the host's and the editor's |
| `Described` | `components: Vec<ComponentShape>` — `ComponentRegistry::describe()` **as it is**, not re-modelled; the IDE builds inspectors and completion from it |
| `Entities` | `entities: Vec<EntityRef>` |
| `Component` | `value`: the component as a Substrate Data value |
| `Spawned` | the new `entity` |
| `Done` | nothing: the request succeeded |
| `Failed` | `error: RequestError` |

| `RequestError` | Meaning |
|---|---|
| `VersionMismatch { host, client }` | `Hello` from an editor newer than the host |
| `NotGreeted` | a request other than `Hello` came first |
| `NoSuchEntity { entity }` | |
| `NoSuchComponent { component }` | no such name in the host's registry |
| `ComponentMissing { entity, component }` | the entity exists but lacks it |
| `InvalidValue { diagnostics }` | a `value` that isn't of the component's shape |
| `Unsupported` | a request this host doesn't know (a newer editor's, or past the negotiated version) |

Errors are data, not text: the editor decides how to show them.

| `Event` | Topic (`Subscribe`) |
|---|---|
| `EntityAdded`, `EntityRemoved` | `Entities` |
| `ComponentChanged { entity, component, value }` | `Components` |
| `Log { level, message }` | `Log` |
| `PlayStateChanged { state }` | `PlayState` |

### Typed ids

- `EntityRef { index, generation }` — an `ecs-core::Entity`, valid for one
  run of the host; re-list after reconnecting.
- `ComponentRef` — an alias of `meridian-scene::ComponentName`, the name
  `.mscene` documents and `Describe` use: one id per component everywhere.
- `RequestId` — chosen by the editor, echoed in the response.
- `ProtocolVersion` — see below.

### Setting a component

`SetComponent.value` replaces the component; it is the component's *new value* (a Substrate Data
value, read with the component's own schema; a value that doesn't read is
`InvalidValue` with every problem and its path). Where the entity comes from
a text `.mscene`, the host builds the document's new value and writes it
with `substrate_data::patch::patch`, which rewrites only what changed, so a
level designer's comments, key order, number spelling and layout survive an
inspector edit.

## Append-only

Peers are built at different times, so the messages only grow:

- variants are added **at the end** of `Request`, `Response`, `RequestError`,
  `Event`, `HostMessage`, `EventTopic`, `LogLevel`, `PlayState`; never
  reordered, renamed or removed;
- fields are added **at the end** of a struct or variant, with
  `#[data(default)]` so an older sender's message still reads;
- a field's type and meaning never change.

Two tests enforce it. `golden` pins one encoded frame per variant byte for
byte (renaming a variant or field, or changing a field's type, changes the
bytes). `variant_order` pins every enum's variant order (the order is what
a packed frame or `frame::read_shaped` by index depends on, and the binary
form does not show it). Appending a variant means adding its row to both;
**no existing row may change**.

## Versioning

`ProtocolVersion::CURRENT` is bumped whenever messages are added. The
binary form refuses what it doesn't know (an unknown variant or field is
`FrameError::Malformed`, never skipped), so the two peers agree on what to
send:

1. The editor's first message is `Hello { version }`.
2. A host accepts a client whose version is **not newer than its own**
   and negotiates `host.negotiate(client)`, the lower of the two, else
   `Failed(VersionMismatch)`. It answers `Welcome { version }` with the
   negotiated version.
3. From then on each peer sends only what the negotiated version
   knows: a newer host does not send variants or fields appended after the
   editor's version.

A request before `Hello` is `Failed(NotGreeted)`.

**What bumps the version.** Besides adding messages, any change to the
written shape of what the messages embed from Substrate Data bumps
`ProtocolVersion`: `Diagnostic` (in `InvalidValue`), `ComponentShape` and its
`ShapeDoc` (in `Described`), and the `Value` model (in `Component`,
`ComponentChanged`, `SetComponent`). Moving to packed frames would be a bump
too.

### Requests the host cannot read

`read_message::<RequestEnvelope>` fails a whole frame it doesn't understand
(`Malformed`), and then the host has no `id` to answer. A host that wants to
answer instead of dropping the connection (an old host, or a client that
skipped `Hello`) does this:

1. `read_message::<Value>` the frame (any frame is a `Value`);
2. take its `id` field and read it as `RequestId`;
3. read the whole value as `RequestEnvelope` (`substrate_data::read_value`);
4. on failure answer `HostMessage::Response { id, response: Failed {
   error: Unsupported } }` with that id.

A frame with no readable `id` is a broken peer: close the connection. The
`malformed` tests run this pattern against an unknown-variant request.

## How Meridian IDLE consumes it

IDLE depends on `meridian-editor-protocol` (a Rust crate: its own process
speaks the same types) and:

1. connects and sends `Hello`;
2. sends `Describe` and builds its inspector, completion and hover docs from
   the `ComponentShape` list (`Shape::describe()` of each component);
3. sends `ListEntities`, then `Subscribe` to `Entities`, `Components`, `Log`
   and `PlayState` to stay current;
4. edits through `SetComponent` and `SpawnEntity` / `DespawnEntity`, and
   drives the simulation with `Play`, `Pause` and `Step`.

Messages are matched by `RequestId`; events arrive any time between.

## Not yet

The host (`engine-core`), a transport, authentication, entity selection and
hierarchy messages, scene load/save requests, and a log-history request.
Each is appended when it is built.
