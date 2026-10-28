# Data formats

Every format Meridian itself writes is [Substrate Data](https://github.com/F000NKKK/Substrate-Data)
(see [ADR 016](adr/016-data-formats-on-substrate-data.md)); its
[format spec](https://github.com/F000NKKK/Substrate-Data/blob/main/docs/format.md)
defines the model, the text syntax (JSON + comments, trailing commas,
unquoted keys), the `SDAT` binary form and frames. This document defines
what Meridian puts in them. Third-party asset formats (PNG, OBJ, MP3, …)
stay `asset-domain` decoders and are not covered here.

| File | Form on disk | Schema owner | What |
|---|---|---|---|
| `meridian.toml` | TOML | `runtime-app` | project manifest |
| `*.mscene` | text (editor) or binary (cooked) | `meridian-data-substrate` | a scene: entities and their components |
| `*.mprefab` | text or binary | `meridian-data-substrate` | a reusable entity tree, instanced by a scene |
| `*.meta` | text | `resource` adapter | an asset's stable id and import settings |
| `*.msave` | binary | envelope: `meridian-data-substrate`; `state`: the game | a save: the world's scene document + game state |
| `*.mpak` | binary | build tool | cooked content bundle |
| editor protocol | frames over a socket | `meridian-editor-domain` (host: `meridian-editor-app` + `meridian-editor-tcp`) | engine ↔ IDE messages |

Text and binary of one file kind share an extension: readers detect
`SDAT` magic and fall back to text.

## Typed references

| Written | Names | Resolved by |
|---|---|---|
| `@asset:meshes/crate.obj` | an asset, by path under the project's asset root | `resource` (path → `.meta` id → handle) |
| `@asset-id:3f2a9c…` | an asset, by its stable id (cooked files) | `resource` |
| `@entity:door_left` | an entity of the same document, by its `id` | `meridian-data-substrate` while loading |
| `@prefab:actors/hero` | a prefab, by path | `meridian-data-substrate` |

A reference is checked while the document is read (`substrate_data::Resolve`)
and every one is reported with its path, so a build can list what a scene
needs before loading anything.

## Math values

With the `data` feature, `numeric` and `gac` implement
`substrate_data::schema::NamedType` for these types (and `Schema` through
`named_schema!`), so each describes itself as `Shape::Named` and an editor
can show a gizmo instead of raw numbers. A motor's stored form is the
public `meridian_gac::MotorForm<S>`.

| Type | Named | Stored as | Text |
|---|---|---|---|
| `Scalar` (`f32`) | — | float | `1.5` |
| `Fixed` | `fixed` | int (raw Q16.16 bits) | `98304` |
| `Vec3` | `vec3` | `[x, y, z]` | `[0, 1.5, 0]` |
| `Bivector3` | `bivector3` | `[e23, e31, e12]` | `[0, 0, 0.5]` |
| `Rotor` | `rotor` | `[s, e23, e31, e12]` | `[1, 0, 0, 0]` |
| `Motor3` | `motor` | `{ rotor, translation }` or `{ rotor, dual }` | `{ rotor: [1, 0, 0, 0], translation: [0, 2, 0] }` |
| `Fixed*` flavors | `fixed_vec3`, `fixed_bivector3`, `fixed_rotor`, `fixed_motor` | the same, components as raw `Fixed` bits | `[0, 98304, 0]` |

Component order is `gac`'s own: `Bivector3::new(e23, e31, e12)` —
the Hodge duals of the x, y, z axes — and a rotor is
`cos(θ/2) − sin(θ/2)(n₁e23 + n₂e31 + n₃e12)`, so a rotation by θ about +Z
is `[cos θ/2, 0, 0, −sin θ/2]`. (`Multivector` stores `e13`; `e31` is
written as its negation, which is exact.) A rotor's other blades and a
motor's odd blades are zero by construction and are not stored.

A motor is written `{ rotor, translation }` — rotate, then move the
origin to `translation` — whenever rebuilding it from those two gives
the same bits; otherwise (recovering a translation through a rotation
rounds) it is written `{ rotor, dual }` with `dual` the motor's stored
`e0` part `[e01, e02, e03, e0123]`. Reading accepts either; a missing
`translation` is `[0, 0, 0]`; both at once is refused ("not a valid
motor", `Problem::Invalid` at the motor's path).

Every value round-trips bit-exactly through text, binary and frames —
floats included (`0.1`, `1e-7`, `-0.0`): text spells each float so it
reads back as the same `f64`, and the `f32` it came from is exact in it.
`Fixed` is never converted through a float. In every form a named type
is stored as its `of` (frames carry the component lists packed, a motor
as a `{ rotor, translation?, dual? }` record), so a tool can read a
frame by shape alone. The tests in `gac`'s `data` module pin all of
this, including the rotor order against the axis–angle formula.

## Scene (`.mscene`)

```jsonc
{
  format: 1,
  name: "Level 1",
  entities: [
    {
      id: "world",                        // stable within the document
      components: {
        transform: { motor: { rotor: [1, 0, 0, 0], translation: [0, 0, 0] } },
      },
    },
    {
      id: "hero",
      name: "Hero",
      prefab: "@prefab:actors/hero",      // optional: start from a prefab…
      parent: "@entity:world",
      components: {                       // …and override these components
        transform: { motor: { rotor: [1, 0, 0, 0], translation: [0, 1.5, 0] } },
        mesh_renderer: { mesh: "@asset:meshes/hero.obj", material: "@asset:materials/skin.mmat" },
        rigid_body: { motion: "dynamic", mass: 80 },
      },
    },
  ],
}
```

- `entities` is a list (order is the document's, kept by editors); each
  entity's `id` is an `EntityKey` (any text without whitespace or control
  characters), unique in the document. `@entity:` references are a kind
  whose ids are `EntityKey`s (`kind!(EntityKind = "entity": EntityKey)`),
  and `@prefab:` ones carry a `PrefabPath`. Runtime `Entity` handles are
  never written — they are generational and process-local.
- `components` is a map from a **registered component name** (a
  `ComponentName`: lower case letters, digits and `_`) to that
  component's own schema. One entry per component type, matching the ECS
  (one component of a type per entity). An unknown name is an error with
  a "did you mean" suggestion, unless the registry allows unknown
  components to pass through untouched (editors do, so a scene written by
  a newer game still opens).
- `prefab` instantiates the prefab, then applies the listed components as
  whole-component overrides. Field-level overrides are a later format bump.

### Binary form

The same document in the `SDAT` binary form (version 2), for cooked
content and saves. It is the document's value encoded canonically, so
lists of numbers are packed (a `Vec3` or a rotor is a list of 32-bit
floats), keys and ids are interned, and entities that share their keys
share one shape. Text and binary are one value: loading text and saving
binary, then loading that and saving text, gives the same document.

| 10 000 generated entities (name, parent, `transform`, `rigid_body`) | Bytes |
|---|---:|
| text, pretty (what `save_scene` writes) | 5 201 948 |
| text, compact | 2 263 108 |
| binary | 678 862 (13.1 % of pretty, 30.0 % of compact) |

(Measured by `tests/scene_size.rs`, `cargo test -p meridian-data-substrate
--test scene_size -- --nocapture`.)

`load_scene_bytes` detects the form: bytes that start with `SDAT` are
binary, anything else is UTF-8 text, never decided by the extension;
`load_scene` (text) and `load_scene_binary` take one form only. The
rules and the `SceneMap` are the text ones: all or nothing, every problem
typed and placed. A binary document has no lines, so a `SceneIssue` has
its `path` (`entities[3].components.transform`) and no line or column;
bytes that do not decode (truncated, an unknown tag, a map that sets a key
twice, a version this does not read, trailing bytes) are a single issue
with the byte `offset` (Substrate Data tracks offsets for decoding only,
not per value). Unknown components pass through a binary document as they
do through text when the registry keeps them.

## Prefab (`.mprefab`)

The same document without `name` (a prefab with `name` is an unknown
key), in either form: a list of entities whose ids are local to the
prefab. The first entity is the root and has no `parent`; every other
entity has one, so a prefab is a tree under its root (`PrefabEmpty`,
`PrefabRootHasParent`, `PrefabOrphan`).

A scene entity with `prefab: "@prefab:actors/hero"` is **the instance
root**: the prefab is read, and

- the scene entity takes the prefab root's place: its `name` if it has
  none of its own, and the root's components, under its own
  `components` — a component the entity lists **replaces the prefab's
  whole** (field-level overrides are a later format bump); `parent` is the
  entity's;
- every other prefab entity is spawned as `<instance id>/<prefab id>`
  (`hero/gun`), with `generated_by` set to the instance root in its
  `SceneEntity`; the `@entity:` references inside the prefab (`parent`, or
  in a component) are renamed with them, the prefab root's id meaning the
  instance (`@entity:body` in the prefab is `hero`);
- prefabs may instance prefabs, ids composing (`squad/leader/gun`), to
  `MAX_PREFAB_DEPTH` (16); a prefab that includes itself, directly or
  through others, is `PrefabCycle` with the chain.

Where the bytes come from is the caller's: the crate has no filesystem and
no asset pipeline. `PrefabSource` is the port (`fn prefab(&mut self,
&PrefabPath) -> Result<Vec<u8>, PrefabUnavailable>`; the bytes are an
`.mprefab` in either form), implemented by the application over its asset
layer; `PrefabLibrary` is an in-memory one. The caller's `Resolve` still
sees the `@prefab:` reference like any other. A prefab that is missing
is `PrefabUnavailable` at the entity's `prefab` field (with the closest
known path as the suggestion when the source has one — `PrefabLibrary`
does), one that does not load is `PrefabInvalid` with its own issues, and
a key an instance makes that the scene already has is `DuplicateEntity`.
As always, nothing is spawned if anything is wrong.

```rust
let scene = registry.load_scene_with_prefabs(&bytes, &mut world, &mut resolver, &mut source)?;
let hero = registry.instantiate_prefab(&PrefabInstance::new(key, path), &mut world, &mut resolver, &mut source)?; // from code
let tree = registry.load_prefab(&bytes, &mut world, &mut resolver, &mut source)?;  // as itself, local ids (an editor)
let text = registry.save_prefab(&world, &tree);                                    // or save_prefab_binary
```

`load_scene` and `load_scene_binary` (no source) leave a `prefab` field
as written. **Saving** a scene writes an instance as its root with the
prefab path and its *current* components as the overrides, and leaves the
generated `<id>/…` entities out: the prefab makes them again on load, so
edits to a generated entity are not saved (they need the field-level
override bump), and a `parent` that names a generated entity is not
expressible in a scene yet.

## Component registry

`meridian-data-substrate::ComponentRegistry` is built by the application (no
global): it maps a component name to its type's schema and the two
functions that move it between a document and a `World`.

```rust
let mut registry = ComponentRegistry::new();
registry.register_with::<Transform, TransformCodec>(ComponentName::new("transform")?)?; // ecs type: through a codec
registry.register::<MeshRenderer>(ComponentName::new("mesh_renderer")?)?;               // T: Component + Schema + Clone
registry.set_unknown_components(UnknownComponents::Keep);                               // editors; games keep `Reject`
let scene = registry.load_scene(&text, &mut world, &mut resolver)?; // SceneMap: EntityKey → Entity, or LoadError
let scene = registry.load_scene_bytes(&bytes, &mut world, &mut resolver)?; // either form, by magic
let text = registry.save_scene(&world, &scene);                     // text form
let bytes = registry.save_scene_binary(&world, &scene);             // SDAT form
let shapes = registry.describe();                                   // Vec<ComponentShape>, for the IDE
```

Registering a name or a type twice is a `RegisterError` (`NameTaken`,
`TypeTaken`). A failed load is a `LoadError`: every `SceneIssue` in file
order, each a typed `SceneProblem` (`Data { problem, .. }` for what
Substrate Data finds — syntax, types, missing fields, an `@entity:` key
the document hasn't — then `UnsupportedFormat`, `DuplicateEntity`,
`OwnParent`, `UnknownComponent { name, suggestion }`, `BadComponentName`)
with its path, line and column (binary: path, and the byte offset for bytes
that do not decode).

`Transform` is `ecs`'s, and `ecs` stays file-blind (ADR 016,
decision 3; no `data` feature on `ecs`), so it has no `Schema` impl:
`meridian-data-substrate::TransformForm`, a newtype implementing `Schema` and
converting to and from `Transform`, stores it as `{ motor }`, and
`TransformCodec` registers `Transform` through it. Any component whose
crate can't implement `Schema` uses the same `register_with` door. The
registry is keyed by name and by `TypeId` (`name_of::<T>()`); saving walks
the registered types per entity. Runtime `Entity { index, generation }`
never reaches a file: document keys are remapped to fresh entities on
load.

Loading is all-or-nothing: every problem is collected first, and a
document with any error spawns nothing. `@entity:` references (in
`parent` or inside components) are resolved against the document's own
keys; every other kind goes to the caller's `Resolve` (`NoResolver`
keeps them as written). `ecs` has no
hierarchy yet, so `parent` becomes `meridian-data-substrate::Parent(Entity)` — a
component that lives in `meridian-data-substrate` for now and moves to `ecs`
once it has a hierarchy — and is also kept in the returned `SceneMap`
(`parent_of`). Saving writes a `Parent` component back as the parent's
document key. Nothing propagates transforms along it yet.

`describe()` returns a `ComponentShape { name, shape: ShapeDoc }` per
registered component, in registration order — a `Schema` itself, so
Meridian IDLE gets it as a frame and builds its inspector and `.meui`
completion from it.

## Asset metadata (`.meta`)

```jsonc
{ format: 1, id: "3f2a9c0e8b1d4a67", importer: "obj", settings: { scale: 1.0, generate_normals: true } }
```

The `id` is the asset's stable identity across renames (`resource`);
cooked content refers to assets by `@asset-id:`.

## Saves (`.msave`)

Binary only, always a map in this key order:

```text
{ format: 1, game: "<id>", created: <unix seconds>, scene: <scene document>, state: <game-owned value> }
```

- `format` is the envelope's own version (`SAVE_FORMAT`); a reader
  refuses any other with `SaveError::UnsupportedFormat` before it looks at
  anything else.
- `game` is the game's own identifier and `created` seconds since the Unix
  epoch, **passed in by the caller** (`SaveHeader::new(game, created)`):
  the data layer has no clock.
- `scene` is the registry's scene document of the world at save time —
  exactly what `save_scene` would write, as a value: a **full** scene, not
  a delta against the level file. A state-level delta is a later format
  bump (that is what `format` is for). Prefab instances are saved as their
  root (see above) and made again on load, so loading a save needs the
  same `PrefabSource`.
- `state` is the game's: `save_game::<T: Schema>` writes any type through
  its own schema, straight, so a `substrate_data::Stream` in it (voxel
  chunks, a recorded input log) stays a stream in the file and is read one
  item at a time.

```rust
let header = SaveHeader::new("my-game", created_unix_seconds);
let bytes = registry.save_game(&header, &world, &scene, &progress);
let loaded: LoadedSave<Progress> = registry.load_game(&bytes, &mut world, &mut resolver, &mut source)?;
let header = SaveHeader::read(&bytes)?;                         // header only
let (header, scene) = registry.load_game_scene(&bytes, &mut world, &mut resolver, &mut source)?; // no state
```

**What is read lazily.** The fields are written header first, then the
scene, then `state`. `SaveHeader::read` and `load_game_scene` walk the
file step by step and stop after the fields they were asked for: a save
browser reads a hundred headers, and an editor with no schema for a game's
state still opens its scene, without decoding the state (a test damages
the end of a save and both still succeed; `load_game` then reports the
damage as `SaveError::State`/`Malformed`). `load_game` reads the whole file
and decodes only `state` with the game's schema, before it spawns the
scene, so it is all or nothing like every load. The price: those two
calls do not validate the bytes after the last field they read.

**Evolution.** The `state` is the *named binary form*, not a frame
(Substrate Data's [evolution rules](https://github.com/F000NKKK/Substrate-Data/blob/main/docs/evolution.md)).
Frames are positional: a reader that is older or newer in any way but an
appended field reads another version's bytes as its own and may get
garbage without an error, and they cannot be inspected without the
schema. The named form is keyed and strict, so:

- a **newer game's** save read by an older game fails loudly —
  `SaveError::State` with `UnknownField` at `state.<field>` — instead of
  silently dropping what the older game cannot keep;
- an **older game's** save read by a newer game works when the newer
  game only appended fields with `#[data(default)]` or `Option` (and
  appended enum variants, never removed or reordered ones);
- anything else (a retyped, moved or removed field) is a bump of the
  game's own: put a `version` as the first field of `state` and migrate on
  read, as `evolution.md` says ("Telling versions apart"). The engine
  versions only the envelope.

Malformed bytes are a typed `SaveError` (`Malformed { offset }`,
`NotAnEnvelope`, `MissingField`, `BadField`, `UnsupportedFormat`, `Scene`
with the scene's own issues, `State` with the game's problems and their
paths), never a panic.

## How a World is saved and restored

There are two copies of a world and they are never mixed.

| | On disk | In memory |
|---|---|---|
| What | the **scene document**: `registry.save_scene*` (a `.mscene`), `save_game` (a `.msave` embeds it) | `World::snapshot(&Cloners)` / `restore` |
| For | levels, saves, editors, cooked content | play/stop in the editor, rollback, tests |
| Entities | named by stable document keys (`EntityKey`); `Entity { index, generation }` is **never written** | the very same `Entity` values, generations kept; one spawned after the snapshot is dead after the restore |
| Components | the registered ones only, through their schemas (unknown ones kept as written if the registry says so) | every `Clone` type registered in `Cloners`; no format involved |
| Loading | spawns fresh entities and remaps keys (`SceneMap`) | puts the old world back |

A snapshot is a memory copy and has no version, so it must not be written
to a file, and a scene document must not be used to roll a running world
back (every entity would be a new one, and handles held elsewhere would
dangle). To persist a running game: save the registry's scene document of
the world (`.msave`); to resume, load it into a fresh world.

## Loading and saving from an application (`meridian-sdk`)

`meridian-data-substrate` has no filesystem; the composition root wires it
to the asset layer (`meridian_sdk::persistence`, no new crate and no new
port: `PrefabSource` stays the data crate's port).

- `AssetPrefabSource` implements `PrefabSource` over an `AssetLoader`
  (`asset-app`): the prefab `actors/hero` is the asset
  `actors/hero.mprefab` (a path already ending in `.mprefab` is used as
  written). The read runs on the loader's background threads
  ([ADR 027](adr/027-background-asset-loading.md)) and the caller waits,
  so there is no `async` and no `tokio`. A missing file is
  `PrefabUnavailable` (`no such prefab`) at the entity that asked; other
  failures carry the source's text.
- `WorldFiles` (registry + directory + loader, built by the application)
  is the entry point: `load_scene(path, &mut world, &mut resolver)`,
  `spawn_prefab`, `save_scene(path, SceneView, SceneForm::{Text, Binary})`,
  `load_game::<T>`, `save_game`, `read_save_header`. Errors are one
  `WorldFilesError` (`Read`, `Interrupted`, `Scene`, `Save`,
  `PathRejected`, `Write`) that wraps the data crate's typed errors.
- Call them from a loading thread, never from a frame, and not from a job
  of the loader's own pool (it waits on that pool). Loads are all or
  nothing, as always.
- Writes go under the same root (a path that is absolute or leaves it is
  `PathRejected`), through a temporary sibling renamed into place, so a
  crash leaves the old save or the new one. `ByteSource` is read-only; a
  `ByteSink` port is a later step when a non-directory target is needed.
- The persisted form is the scene document (`save_scene`/`save_game`);
  `World::snapshot` stays in-memory only and is not used here.

## Editor protocol

Messages are Substrate Data frames over a byte stream
(`substrate_data::io::write_frame`/`read_frame`): a little-endian `u32`
length, then the message in the binary form. Requests, responses and
events are tagged enums deriving `Schema`, in `meridian-editor-domain`;
they are only ever appended to, and the two peers agree on a
`ProtocolVersion` in the first message, because the binary form rejects
what it doesn't know (an older reader does not skip a newer peer's field).
Specified in [editor-domain.md](editor-domain.md).

## Status

| Piece | State |
|---|---|
| `substrate-data` crates | done (separate repository; 0.3.x on crates.io) |
| `data` feature on `numeric`/`gac` | done |
| `meridian-data-substrate` (registry, `.mscene` text and binary load/save, `describe()`) | done (hierarchy as `meridian-data-substrate::Parent` until `ecs` has one) |
| `.mprefab` (load, instancing with whole-component overrides, nesting, cycle and depth checks, `PrefabSource` port, text and binary save) | done; field-level overrides, patching an instance's edits back, and a scene `parent` naming a generated entity are later |
| `.msave` envelope (`format`/`game`/`created`/`scene`/`state`, lazy header and scene) | done; scene is full, a delta is a later `format` |
| `meridian-sdk` `WorldFiles` + `AssetPrefabSource` (scene, prefab and save by path over the asset layer) | done; not yet driven by `runtime-app`'s frame loop |
| patching an existing `.mscene` file on save | not implemented |
| `.meta`, `.mpak` | designed, not implemented |
| `meridian-editor-domain` (messages, framing, golden-pinned wire) | done |
| `meridian-editor-app` (session, ports), `meridian-editor-tcp` (loopback TCP) | done; not yet wired into `runtime-app`'s frame loop |
