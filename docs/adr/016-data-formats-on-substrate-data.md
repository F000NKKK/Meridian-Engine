# ADR 016: Data formats on Substrate Data

## Status

Accepted. Detailed in [data-formats.md](../data-formats.md).

## Context

Meridian has decoders for third-party asset formats (PNG, BMP, OBJ, audio
codecs in `asset-core`) and a scene *authoring* DSL
([ADR 015](015-extensible-scene-dsl.md)), but no format of its own for
anything it has to **write back**: a scene an editor saves, a prefab, a
save game, cooked content, project settings, or the messages between the
engine and an IDE. The planned Meridian IDLE needs three things the engine
cannot give today: scene files an inspector can edit without destroying
hand-written comments and layout, a description of every component type
(fields, types, docs) to build property grids from, and a compact form to
stream state over a socket.

Yog Mod Loader already solved exactly this with `yog-data` — one model,
JSON-compatible text with spans and in-place patching, a tagged binary
form, schema-packed frames and `#[derive(Schema)]` that also describes the
type — but tied to Minecraft. Its neutral core was extracted into
[Substrate Data](https://github.com/F000NKKK/Substrate-Data)
(`substrate-data`, MIT OR Apache-2.0), shared by Meridian, Yog and tools
on Substrate Platform.

Alternatives: `serde` + `serde_json`/`ron`/`bincode` — no spans per value,
no comment-preserving edits, no type description for editors, three
unrelated encodings; writing a Meridian-only format — duplicates
`substrate-data`, against "don't drag another crate's logic into your
own".

## Decision

1. **Every Meridian-owned format is Substrate Data.** Text for files
   people edit, binary (`SDAT`) for cooked/saved data, frames for
   engine ↔ editor messages. A file has one extension and either form,
   detected by magic bytes (`SDAT`) — the same rule as `asset-core`'s
   decoders, never by extension.
2. **Math types describe themselves** behind an optional `data` feature on
   `meridian-numeric-core` and `meridian-gac-core` (`impl Schema` with
   `Shape::Named` — `"vec3"`, `"rotor"`, `"motor"`, `"fixed"`), like a
   `serde` feature: no math crate depends on `substrate-data` unless asked.
   `Fixed` is stored as its raw `i32` so deterministic data round-trips
   bit-exactly.
3. **Scene serialization lives in a new crate `meridian-scene`** (above
   `ecs-core`, below `engine-core`): a `ComponentRegistry` (component name →
   shape, read into a `World`, write out of one), the `.mscene`/`.mprefab`
   document schemas, and `World` load/save. `ecs-core` stays generic and
   learns nothing about files (rule: no domain leakage into ECS); the
   registry is a plain value the application builds, not a global
   manager (ADR 003).
4. **References are typed strings** `@kind:id`: `@asset:` (an asset path
   relative to the project's asset root), `@entity:` (an entity's stable
   id within the same document), `@prefab:`. Asset references are resolved
   by `resource-core` identity, never by the loader (ADR 006).
5. **Formats evolve by appending.** Every document carries `format: N`;
   frames only append fields; binary tags only grow (Substrate Data's own
   rule). A migration is a function `Value -> Value` per bump. The named
   binary form an `io` frame carries rejects what it doesn't know, so the
   editor protocol adds a version handshake on top
   ([editor-protocol.md](../editor-protocol.md)).

## Consequences

- One parser, one error shape (`Diagnostic` with line, column and path)
  for every format; the IDE shows them all the same way.
- The IDE gets component descriptions from `ComponentRegistry::describe()`
  and edits files through `substrate_data::patch`, so hand-written comments
  survive inspector edits.
- `.mel` (ADR 015) stays the authoring markup for scenes written by hand;
  `.mscene` is what tools save. Settled by
  [ADR 017](017-mel-markup-and-code-behind.md): the two stay separate
  formats (`.mel` is not compiled into `.mscene`) but share one literal
  grammar — `.mel` attribute values follow the Substrate Data string rules
  (`@kind:id` is a reference, `@@` escapes, any other leading `@` is an
  error; resources are `@res:Set.name`).
- New external dependency `substrate-data` (git, pinned by revision until
  it is published to crates.io).
