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
| `*.meta` | text or binary | `meridian-asset-domain` | an asset's stable id, importer, import settings and dependencies |
| `*.msave` | binary | envelope: `meridian-data-substrate`; `state`: the game | a save: the world's scene document + game state |
| `*.mpak` | binary (`MPAK` envelope; index is `SDAT`) | `meridian-asset-domain` (read: `PackSource` in `asset-app`, `FileReadAt` in `asset-fs`; streamed write: `FileWriteAt`, `add_file` in `asset-fs`) | cooked content pack: payload blobs + index, random access by id or path |
| editor protocol | frames over a socket | `meridian-editor-domain` (host: `meridian-editor-app` + `meridian-editor-tcp`) | engine ↔ IDE messages |

Text and binary of one file kind share an extension: readers detect
`SDAT` magic and fall back to text.

## Typed references

| Written | Names | Resolved by |
|---|---|---|
| `@asset:meshes/crate.obj` | an asset, by path under the project's asset root | `resource` (path → `.meta` id → handle) |
| `@asset_id:3f2a9c0e8b1d4a67` | an asset, by its stable id (cooked files, `.meta` `dependencies`) | `resource` |
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
- `prefab` instantiates the prefab, then applies the listed `components`
  as whole-component replacements and the entity's `overrides` field by
  field (see [Prefab](#prefab-mprefab)).

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
  whole**; `parent` is the entity's; its `overrides` then change single
  fields (below);
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
as written.

### Overrides

An entity that says `prefab` may also have `overrides`: a map from the key
of an entity of the instance to what changes in it. The target is the
instance itself (`hero`) or an entity its prefab made, by the key it has
once instanced (`hero/gun`; in a nested instance `squad/leader/gun`). It is
a key in the document the override is written in, so inside a prefab it is
that prefab's local id (`leader/gun`) and is renamed with the instance. Each
target holds any of three keys:

```text
overrides: {
  "hero/gun": {
    fields:     { rigid_body: { mass: 2 } },        // merged into the component
    components: { follow: { target: "@entity:hero" } },   // the component, whole
    remove:     ["mesh_renderer"],                  // taken away
  },
}
```

- `fields` is a map from component name to a **patch**, merged into the
  component's document value: maps merge key by key, recursively; a list,
  a scalar, `null`, or a map over a value that is not a map **replaces**
  what is there (`null` is a value, not a deletion). A component the
  target does not have gets the patch as its whole value. The merged value
  is read by the component's own schema, so a field it doesn't know or a
  value of the wrong type is a `Data` problem at that field.
- `components` replaces the component whole (the same as the entity's own
  `components`, which stays and means the same for the instance root) or
  adds it.
- `remove` takes components away; naming one the target lacks is
  `OverrideRemovesMissing`.
- Order: the entity's `components`, then per target in the order written
  `components`, `fields`, `remove`. `@entity:` names inside an override
  mean what they mean elsewhere in the document, and may name generated
  entities (`@entity:hero/gun`).
- Problems, each at its path (`entities[0].overrides["h/gunn"]`):
  `BadOverrideTarget` (not an entity key), `OverrideOutsideInstance`
  (neither the instance nor below it), `NoSuchGeneratedEntity` (the prefab
  makes no such entity; with a suggestion), `OverridesWithoutPrefab`
  (`overrides` on an entity without `prefab`), `OverridesNeedPrefabs`
  (loaded without a `PrefabSource`, which can't know what they override),
  `UnknownComponent`, `BadComponentName`, and unknown keys in a target.
  All are reported together; nothing is spawned.

The key is additive: `format` stays 1, every document that read before
reads the same, and an older reader refuses a document that has
`overrides` with an unknown-key problem rather than ignoring it.

### Entities below an instance

`parent`, and any `@entity:` reference in a component, may name an entity
a prefab makes: `parent: "@entity:hero/gun"` puts a scene entity under the
gun. The name is checked after the prefabs are instanced:
`NoSuchGeneratedEntity` if it isn't made (also in a document read without
a source, where nothing is), and `ParentInsideInstance` for an instance
parented to one of its own entities. Only a name below an entity that
says `prefab` is taken for such an entity; any other unknown name is the
usual unresolved reference.

### Saving instances

`save_scene`, `save_scene_binary` and `save_game` write a prefab instance as
its root with the prefab path and its *current* components, and leave the
generated `<id>/…` entities out; they need no prefabs but cannot keep an
edit to a generated entity. The `_with_prefabs` forms do:

```rust
let text  = registry.save_scene_with_prefabs(SceneView::new(&world, &scene), &mut source)?;
let bytes = registry.save_scene_binary_with_prefabs(SceneView::new(&world, &scene), &mut source)?;
let save  = registry.save_game_with_prefabs(&header, SceneView::new(&world, &scene), &state, &mut source)?;
```

Each instance's prefab is read again (the same `PrefabSource`) and every
entity of the instance — the root and the generated ones, nested
instances included — is compared with what the prefab makes. What the
instance writes is `overrides` only, keyed by the entity's key:

- a component whose document value differs: the **changed fields only**
  (`fields`, recursive for maps), a changed list or scalar whole;
- a component the prefab entity lacks, and a component that can't be
  patched (a map lost a key, so a patch cannot say so), as `components`;
- a component the prefab entity has and the world lacks, as `remove`;
- the `name` only when it differs from the prefab root's; no
  `components` on the instance, and no `overrides` when nothing changed.

Loading what is written with the same prefabs gives the same world
(round-trip tested in both forms, nested instances too). `SceneSaveError`
names the instance and prefab when a prefab can't be read. What is not
saved: a generated entity despawned (the prefab makes it again), a
generated entity's `name`, `parent` or `prefab`, and entities added to
the world under an instance. `save_prefab` writes a prefab as before.

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

One sidecar per source asset, next to it: `meshes/crate.obj` has
`meshes/crate.obj.meta` (`AssetPath::sidecar`). Text when an editor or a
person writes it, `SDAT` binary when cooked; readers detect the magic.

```jsonc
{
  format: 1,
  id: "3f2a9c0e8b1d4a67",            // the asset's stable identity
  importer: "obj",                   // which importer reads the source file
  settings: { scale: 1.0, generate_normals: true },   // the importer's own, optional
  dependencies: ["@asset_id:0a1b2c3d4e5f6071"],        // assets this one needs, optional
}
```

- `id` is an `AssetId`: 64 bits as 16 lower case hex digits. It is the
  asset's identity across renames and moves, and what cooked content
  (`.mpak`, `dependencies`) names it by (`@asset_id:`). It is a name on
  disk, not a runtime handle: no generation, no lifetime. Mapping an id to
  a `resource` handle is the `ResourceIdTable` (below), filled by the code
  that loads the asset, not by the loader or this file (ADR 006). The data layer has no random source, so
  whoever creates a `.meta` chooses the value.
- `importer` is an `ImporterName` (lower case letters, digits, `_`, `-`,
  starting with a letter). Which names exist is the application's
  `ImporterRegistry` (below).
- `settings` is a map this crate does not interpret: the importer reads it
  with its own schema. Keys keep file order. Absent means empty.
- `dependencies` are `@asset_id:` references, each at most once and never the
  asset itself (`DuplicateDependency`, `SelfDependency`). Being references
  they are checked and listed by `substrate_data::Resolve` like any other,
  so a build can walk the graph before loading anything.
- Unknown keys are an error, so a file written by a newer version is refused
  rather than half read.

**Versioning.** `format` is the sidecar's own version (`META_FORMAT`, 1).
Writers always write the newest. A reader accepts every version up to the
newest it knows: an older one is migrated as a value to the newest before it
is read (none exists yet); a newer one is `MetaError::UnsupportedFormat`,
decided before the other fields are looked at, so it is not reported as a
list of unknown keys. A missing or non-positive `format` is `BadFormat`.

**Errors.** `MetaError`: `NotUtf8 { valid_up_to }`, `Malformed { offset }`
(binary), `Invalid(Vec<Diagnostic>)` for syntax and field problems with
their path, line and column (path only in binary), and the checked rules
above. A `.meta` is read whole or not at all.

```rust
let meta = AssetMeta::read(&bytes)?;            // text or binary
let text = meta.to_text();                      // or meta.to_bytes()
```

### From a `.meta` to content: importers, catalog, identity

Four pieces, each in the layer that owns it, none global:

- **`ImporterRegistry<T>`** (`meridian-asset-app`): the table from an
  `importer` name to an `Importer<T>`, one registry per content type
  (`ImageAsset`, `MeshAsset`, ...), so a name cannot return the wrong type.
  `Importer::import(bytes, &AssetMeta)` is pure: it reads the asset's
  `settings` from the meta and nothing from disk. A plain `Decoder` is an
  importer through `FromDecoder` (`register_decoder`); an importer with
  settings implements `Importer` itself. `register` twice under one name is
  `RegisterError::NameTaken` (the first stays); a name the registry does not
  have is `ImportError::UnknownImporter`; the importer's own rejection is
  `ImportError::Failed { importer, source }`. The domain's `Decoder` stays the
  decoding port; the registry is application wiring, built by the
  composition root (`meridian_sdk::content::Importers::standard()` registers
  `image`/`bmp`/`png`/`jpeg`, `obj`, `gltf`, `wgsl`, `audio`/`wav`).
- **`AssetLoader::load_imported(path, registry)`**: one background job reads
  `<path>.meta`, picks the importer it names, reads the asset and imports it.
  A missing sidecar is `LoadError::Source(NotFound(<path>.meta))`, a damaged
  one `LoadError::Decode(ImportError::InvalidMeta)`. `read_meta(source, path)`
  is the blocking read of one sidecar.
- **`AssetCatalog`** (`meridian-asset-app`): `AssetCatalog::scan(&dyn
  AssetListing)` walks a source, reads every `*.meta` and keeps
  `AssetId` to `CatalogEntry { id, path, importer, dependencies }` and path to
  id. `AssetListing` (a `ByteSource` that can name its paths) is a separate
  port because not every source can list: `FsSource` walks the directory
  (no symbolic links followed), `PackSource` lists its entries. The scan
  is all or nothing: a bad sidecar (`CatalogError::Meta`, naming the file),
  two assets with one id (`DuplicateId`), a dependency no `.meta` declares
  (`MissingDependency`) or a loop (`DependencyCycle`) is an error, so a
  build can walk the graph before loading anything. A `.meta` whose source
  file is absent is not checked. It is blocking: run it off the frame thread.
- **`ResourceIdTable<K, T>`** (`meridian-resource`): the map from a stable key
  (here the `AssetId`) to the `ResourceId<T>` of the resource loaded for it,
  and back (`key_of`). Two facts only: which keys exist (`declare`, filled
  from the catalog) and which are loaded as what (`bind` / `unbind`). It
  loads nothing, holds no content and decides no lifetime: the loading code
  binds after it inserted the content in its pool, the code that releases
  unbinds, and a reloaded resource has a new generation so the old id is not
  found. `bind` refuses an unknown key, a key already bound and a resource
  that is another key's (`BindError`).

```rust
let catalog = AssetCatalog::scan(&source)?;                  // .meta -> ids
let mut table = ResourceIdTable::<AssetId, ImageAsset>::declaring(catalog.iter().map(|e| e.id));
let image = loader.load_imported(entry.path.clone(), images.clone()).wait()?;
table.bind(entry.id, ResourceId::new(pool.insert(image)))?;  // identity, after loading
```

## Asset pack (`.mpak`)

A cooked content pack: the payloads of many assets and an index, readable
by asset id or path without reading the whole file.

```text
offset  size  field
0       4     magic            "MPAK"
4       2     version          envelope version, u16 (1)
6       2     flags            u16, must be 0 in version 1
8       8     index_offset     u64, where the index starts
16      8     index_len        u64; index_offset + index_len == file length
24      4     index_crc32      CRC-32 (IEEE) of the index bytes
28      4     header_crc32     CRC-32 of bytes 0..28
32      ...   payloads         each asset's bytes, back to back, no padding
...     ...   index            Substrate Data binary (version 2), to the end of the file
```

All numbers are little-endian. The index is one map:

```jsonc
{ entries: [ { id: "3f2a9c0e8b1d4a67", path: "meshes/crate.obj",
               offset: 32, len: 1204, crc32: 2387465021 }, … ] }
```

`offset` and `len` locate the asset's bytes from the start of the file and
must lie inside the payload area (`32 .. index_offset`); `crc32` is the
CRC-32 of those bytes. `id` is the asset's `AssetId`; `path` a `PackPath`
(`/`-separated, relative, no empty, `.` or `..` segment, no `\` or control
characters, at most 1024 bytes), the same shape a directory asset source
takes. Ids and paths are unique in a pack. Every byte of a pack is covered by
a checksum (header, index, each payload), so a flipped byte is found by
`open` or by reading the asset it belongs to.

**Reading.** `PackReader::open(source)` reads the 32-byte header and the
index only, checks both, and checks every entry; an asset's bytes are read
by range when asked for (`read_id`, `read_path`) and verified against
their `crc32` before they are returned. `PackReader::verify` reads them all
(what a build or installer runs once). The source is the `ReadAt` port
(`size`, `read_exact_at(offset, buf)`) that `asset-domain` owns: `[u8]` and
`Vec<u8>` implement it, `meridian_asset_fs::FileReadAt` implements it for a
file, and a memory map or range requests would be other adapters. No read
or allocation exceeds what the file holds: the index length must end the
file exactly, is capped at `MAX_INDEX_LEN` (256 MiB), and an entry's size
was checked against the payload area before its buffer is made.

**Writing.** `PackWriter::add(id, path, bytes)` then `finish()`: header,
payloads in the order added, index. It builds in memory.
`StreamingPackWriter` writes the same bytes for packs larger than memory,
through the `WriteAt` port (`write_at(offset, bytes)`, the write twin of
`ReadAt`; `Vec<u8>` implements it, `meridian_asset_fs::FileWriteAt` does for a
file). The header comes first in the file but holds the index's place and
CRC-32, known only at the end, so the writer reserves 32 zero bytes (not a
valid header), streams the payloads after them with each asset's CRC-32
computed as its chunks pass, writes the index after the last payload and
then the header over the reserved bytes. That is why the sink is positioned
rather than forward-only, and why a file abandoned or failed halfway never
opens as a pack (`BadMagic`). An asset is written whole (`add`) or in
chunks (`begin`, `write`, `end`); memory held is the entry list and one
chunk. For the same assets in the same order the bytes are identical to
`PackWriter`'s (tested for any chunking). Errors are `PackWriteError`:
`Build` (duplicate id or path, refused at `begin` before anything is
written), `Sink(WriteAtError)`, `AssetOpen`, `NoAssetOpen`, `TooLarge`; after
any error the writer is dropped and the output discarded.
`meridian_asset_fs::add_file(writer, id, path, file)` copies a file in 64 KiB
chunks.

**Build tool.** There is no command-line crate in the workspace yet, so no
`mpak` command ships: a build tool is a loop over `AssetCatalog::scan` of the
source directory (ids and paths from the `.meta` files) that calls `add_file`
for each asset and its `.meta`, then `finish()` and `FileWriteAt::sync`. The
pieces are re-exported by `meridian_sdk::content`; a CLI crate (layer
interface, depending on `asset-fs`) is a later addition, not part of this
change.

**Versioning, and the lossless upgrade rule.** Readers accept every
envelope version up to `PACK_VERSION` (1) and every Substrate Data binary
version they read (the index of a pack may be SDAT version 1 or 2); writers
always write the newest of both. A newer `version` is
`UnsupportedVersion` (version 0 never existed and is refused too), nonzero
`flags` are `UnknownFlags` (checked after the header checksum, so a damaged
header is not mistaken for a newer one), and the index document is strict,
so a newer index with extra keys is refused, not half read; a change to the
index or layout bumps `version`. `upgrade_pack(source)` rewrites any readable
pack as the newest: entries, their order, ids, paths and every payload
byte are kept, so the value read is the value written; it verifies each
payload on the way, and upgrading a current pack returns identical bytes.
Binary tags and layout fields only grow, as in Substrate Data.

**Errors.** `PackError`, every variant with the offset, entry or path it
concerns: `Read`, `TooShort`, `BadMagic`, `UnsupportedVersion`,
`HeaderChecksum`, `UnknownFlags`, `BadLayout`, `IndexTooLarge`,
`IndexChecksum`, `IndexMalformed { offset }` (from the start of the file),
`IndexInvalid` (with the index's own paths, `entries[3].path`),
`EntryOutOfBounds`, `DuplicateId`, `DuplicatePath`, `NoSuchId`,
`NoSuchPath`, `TooLargeForMemory`, `PayloadChecksum`. Tests truncate a pack
at every length and flip every byte: nothing panics and every damage is an
error.

**Through the asset layer.** `meridian_asset_app::PackSource` is a
`ByteSource` over a pack: the asset path is the path in the pack, a missing
one is `NotFound`, a bad path `Rejected`, a damaged asset `Unreadable` with
the pack's words. So an `AssetLoader` — and `AssetPrefabSource`, `WorldFiles`
— works on a pack as on a directory. A pack carries its `.meta`
sidecars as entries like any other file, so `AssetCatalog::scan` and
`load_imported` work on it unchanged.

**Choosing at startup.** The composition root decides where assets come
from once, with a typed value (`meridian_sdk::content`), never by looking at
a file name:

```rust
let layers = Layers::pack("base.mpak")                       // lowest priority
    .over(Layer::Pack("patch-1.mpak".into()))                 // replaces assets it holds
    .over(Layer::Directory("dev-assets".into()));             // loose files win
let content = Mounted::mount(layers)?;                        // headers + indexes only
let loader = content.default_loader();                        // AssetLoader, ADR 027
let files = WorldFiles::new(save_dir, loader, registry);      // unchanged
```

`Layer` is `Directory` or `Pack`; `Layers` is one or more, lowest priority
first, never empty (`Layers::directory(p)` and `Layers::pack(p)` are the
single-layer cases). `Mounted::mount` opens them all or none (`OpenError`:
`NotADirectory`, `File`, `Pack`, each naming the path). Reads go through
`OverlaySource` in `asset-app`: the top layer that has the path answers, and
only *not found* falls through, so a damaged asset in a patch is reported
(`Unreadable`), not replaced by the older copy under it. The listing the
catalog scans is the union of the layers. `WorldFiles` and `AssetLoader`
take the same `ByteSource` either way; saves are not a layer: `WorldFiles`
still writes to the directory it is given.

```rust
let source = PackSource::open(FileReadAt::open("content.mpak")?)?;   // header + index only
let loader = AssetLoader::new(Arc::new(source), pool);
```

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
  same `PrefabSource`; `save_game_with_prefabs` writes them as the changes
  from their prefab, edits to generated entities included.
- `state` is the game's: `save_game::<T: Schema>` writes any type through
  its own schema, straight, so a `substrate_data::Stream` in it (voxel
  chunks, a recorded input log) stays a stream in the file and is read one
  item at a time. Since Substrate Data 0.4 loading does not walk the
  stream: `Stream::iter()` yields a `Result` per item, so a damaged item
  surfaces (with its index and offset) when the game iterates, or earlier
  through `Stream::validate()`.

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
  `load_game::<T>`, `save_game`, `read_save_header`. Saving writes prefab
  instances as overrides ([Saving instances](#saving-instances)), reading
  each instance's prefab from the directory again. Errors are one
  `WorldFilesError` (`Read`, `Interrupted`, `Scene`, `Save`, `Instance`,
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
| `.mprefab` (load, instancing with whole-component and field-level overrides, overrides on generated entities by path, `parent`/references naming generated entities, nesting, cycle and depth checks, `PrefabSource` port, text and binary save; instances saved as the changes from their prefab) | done; removing or reparenting a generated entity is not saved |
| `.msave` envelope (`format`/`game`/`created`/`scene`/`state`, lazy header and scene) | done; scene is full, a delta is a later `format` |
| `meridian-sdk` `WorldFiles` + `AssetPrefabSource` (scene, prefab and save by path over the asset layer) | done; not yet driven by `runtime-app`'s frame loop |
| patching an existing `.mscene` file on save | not implemented |
| `.meta` (`AssetMeta`: id, importer, settings, dependencies; text and binary; version rule) | done in `asset-domain`; read by `ImporterRegistry`/`load_imported`, `AssetCatalog` (`asset-app`) and `ResourceIdTable` (`resource`); the sdk registers the shipped importers |
| `.mpak` (`PackWriter`, `PackReader`, `upgrade_pack`; `PackSource`, `FileReadAt`) | done; the `sdk` picks a directory, packs or a stack of them at startup (`content::Layers`, `Mounted`); `StreamingPackWriter` (+ `WriteAt`, `FileWriteAt`, `add_file`) writes a pack without holding it; no build-tool CLI crate exists yet (recipe above) |
| `meridian-editor-domain` (messages, framing, golden-pinned wire) | done |
| `meridian-editor-app` (session, ports), `meridian-editor-tcp` (loopback TCP) | done; not yet wired into `runtime-app`'s frame loop |
