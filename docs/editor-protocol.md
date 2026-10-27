# Editor protocol

What a running Meridian game and its editor (Meridian IDLE) say to each
other. The types live in `meridian-editor-domain`
([dependency-rules.md](dependency-rules.md): above `meridian-data-substrate`, below
`runtime-app`); the formats under them are
[Substrate Data](data-formats.md) ([ADR 016](adr/016-data-formats-on-substrate-data.md)).

The crate is **pure protocol**: no `async`, no sockets, no I/O beyond a
generic `Read`/`Write`. The game's side is [the host](#host): the code that
answers requests (`meridian-editor-app`) and the socket it is served on
(`meridian-editor-tcp`).

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

- `EntityRef { index, generation }` — an `ecs::Entity`, valid for one
  run of the host; re-list after reconnecting.
- `ComponentRef` — an alias of `meridian-data-substrate::ComponentName`, the name
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

IDLE depends on `meridian-editor-domain` (a Rust crate: its own process
speaks the same types) and:

1. connects and sends `Hello`;
2. sends `Describe` and builds its inspector, completion and hover docs from
   the `ComponentShape` list (`Shape::describe()` of each component);
3. sends `ListEntities`, then `Subscribe` to `Entities`, `Components`, `Log`
   and `PlayState` to stay current;
4. edits through `SetComponent` and `SpawnEntity` / `DespawnEntity`, and
   drives the simulation with `Play`, `Pause` and `Step`.

Messages are matched by `RequestId`; events arrive any time between.

## Host

The game's side is two crates, split on purpose: what answers is pure and
synchronous, what listens does I/O.

| Crate | Does | Depends on `tokio` |
|---|---|---|
| `meridian-editor-app` | `Session`: the state machine of one editor connection | no |
| `meridian-editor-tcp` | `EditorListener`, `serve_stream`: frames over loopback TCP | yes |

Both sit at the same tier as the protocol (above `scene`, below
`runtime-app`) with no edge beyond `ecs`, `scene` and the protocol
([dependency-rules.md](dependency-rules.md)); both are *infra* in
`craftbook.toml`, because an app-layer crate may not depend on the protocol
crate, which is infra (ADR 019).

### Lifecycle

1. The host binds `127.0.0.1:<port>` (`EditorListener::bind`; port `0` picks
   a free one, read it back with `address()`) and accepts **one** editor
   (`serve_one`). There is no authentication: the socket is loopback only,
   and whoever can connect can edit the game.
2. Each frame is classified (`Incoming::classify`): a request goes to the
   `RequestHandler`; a frame with a readable `id` but a request this build
   does not know is answered `Failed { Unsupported }` with that id (the
   pattern under "Requests the host cannot read") and the connection goes
   on; a frame with no readable `id` ends it with `ServeError::BrokenPeer`.
3. `Session::handle(envelope, &mut Game)` returns the `HostMessage`s to
   send: the `Response`, then the `Event`s the request caused that the
   editor subscribed to. `Game` borrows the `World`, the `ComponentRegistry`
   and a `SimulationControl` for that one request, so the host decides when
   requests run and the session owns no game state.
4. The editor hanging up between frames ends `serve_one` with `Ok`. A
   stream cut inside a frame (**including inside the 4-byte length**, which
   `substrate_data::io::read_frame_bytes` reads as a clean end), a length
   over `MAX_FRAME` or bytes that are not a message end it with
   `ServeError::Frame(Truncated | TooLong | Malformed)`.

`Session` rules: before a successful `Hello` every request is
`Failed(NotGreeted)`; `Hello` negotiates with `ProtocolVersion::negotiate`
(a newer editor gets `VersionMismatch` and stays ungreeted; a second `Hello`
negotiates again). `Describe` is `ComponentRegistry::describe()` as it is.
`ListEntities` is ordered by index. `GetComponent` / `SetComponent` check, in
order, the entity (`NoSuchEntity`), then the component name
(`NoSuchComponent`); `GetComponent` then reports `ComponentMissing`, and
`SetComponent` reads `value` with the component's own schema through
`ComponentRegistry::set_component` (`InvalidValue` carries every
`Diagnostic` with its path, and the world is untouched). A successful set
announces the value *as stored*, not as sent.

Events a request causes: `SpawnEntity` and `DespawnEntity` announce
`EntityAdded` / `EntityRemoved` (topic `Entities`), `SetComponent` announces
`ComponentChanged` (`Components`), and a request that changes the play
state announces `PlayStateChanged` (`PlayState`) only when it did change.
`Session::announce(event)` turns an event the host raises on its own into a
message when its topic is subscribed.

### Play, pause, step

Stepping stays with `runtime-app`: the host implements the
`SimulationControl` port (`state`, `play`, `pause`, `step`) and the session
only maps the requests onto it. `Step` pauses, steps once, and stays
paused. `ManualSimulation` is the port's double: it counts steps, for tests
and for a headless host.

### What is implemented, and what is not

Implemented: every `Request` of protocol version 1; subscriptions by
`EventTopic`; the loopback TCP adapter; the unknown-request pattern.

Not yet:

- **Wiring into `runtime-app`.** `runtime-app` neither depends on the host
  nor implements `SimulationControl`. A game today builds the pieces itself:
  a `RequestHandler` that owns (or reaches) its `World`, runs
  `Session::handle`, and an `EditorListener::serve_one` on a tokio task. The
  `RequestHandler` is synchronous, so a handler that needs the frame loop's
  `World` must forward requests to it (a channel, answered at a frame
  boundary); that bridge comes with the wiring.
- **Events the host raises on its own** (`Log`, `PlayStateChanged` from the
  game, `ComponentChanged` for edits the game made itself): `announce`
  exists, but the adapter writes only what the handler returns for a
  request, so they are not pushed while the editor is idle. Detecting world
  changes not caused by a request needs change tracking in `ecs`.
- A Unix socket or named pipe (`serve_stream` takes any async stream, so it
  is a few lines), a second concurrent editor, authentication.
- Gating by the negotiated version: all of version 1 is served as soon as
  the editor is greeted; this starts to matter with version 2.
- Patching the text `.mscene` an entity came from with
  `substrate_data::patch::patch` (see "Setting a component"): the host edits
  the live `World` only.

## Not yet

Entity selection and hierarchy messages, scene load/save requests, a
log-history request, authentication. Each is appended when it is built.
