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
| `*.mprefab` | text or binary | `meridian-data-substrate` | a reusable entity tree |
| `*.meta` | text | `resource` adapter | an asset's stable id and import settings |
| `*.msave` | binary | the game | a save: scene delta + game state |
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

## Prefab (`.mprefab`)

The same document without `name`: a list of entities whose ids are local
to the prefab; the first entity is the root. Instancing renames ids as
`<instance id>/<prefab id>`.

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
let text = registry.save_scene(&world, &scene);                     // text form
let shapes = registry.describe();                                   // Vec<ComponentShape>, for the IDE
```

Registering a name or a type twice is a `RegisterError` (`NameTaken`,
`TypeTaken`). A failed load is a `LoadError`: every `SceneIssue` in file
order, each a typed `SceneProblem` (`Data { problem, .. }` for what
Substrate Data finds — syntax, types, missing fields, an `@entity:` key
the document hasn't — then `UnsupportedFormat`, `DuplicateEntity`,
`OwnParent`, `UnknownComponent { name, suggestion }`, `BadComponentName`)
with its path, line and column.

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
document key. Nothing propagates transforms along it yet; instancing
`prefab` is also a later step.

`describe()` returns a `ComponentShape { name, shape: ShapeDoc }` per
registered component, in registration order — a `Schema` itself, so
Meridian IDLE gets it as a frame and builds its inspector and `.mel`
completion from it.

## Asset metadata (`.meta`)

```jsonc
{ format: 1, id: "3f2a9c0e8b1d4a67", importer: "obj", settings: { scale: 1.0, generate_normals: true } }
```

The `id` is the asset's stable identity across renames (`resource`);
cooked content refers to assets by `@asset-id:`.

## Saves (`.msave`)

Binary only: `{ format, game, created, scene: <scene delta>, state: <game-owned value> }`.
Large state (voxel areas, recorded inputs) uses `substrate_data::Stream`
so it can be read lazily. The game owns the `state` schema; the engine
only owns the envelope.

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
| `substrate-data` crates | done (separate repository) |
| `data` feature on `numeric`/`gac` | done |
| `meridian-data-substrate` (registry, `.mscene` load/save, `describe()`) | done (text form; hierarchy as `meridian-data-substrate::Parent` until `ecs` has one) |
| `.mscene` binary form, patching an existing file on save | not implemented |
| `.mprefab`, `.meta`, `.msave`, `.mpak` | designed, not implemented |
| `meridian-editor-domain` (messages, framing, golden-pinned wire) | done |
| `meridian-editor-app` (session, ports), `meridian-editor-tcp` (loopback TCP) | done; not yet wired into `runtime-app`'s frame loop |
