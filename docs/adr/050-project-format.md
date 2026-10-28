# ADR 050: the project format (`.mproj`)

## Status

Proposed. Builds on [ADR 016](016-data-formats-on-substrate-data.md) (every
Meridian-owned format is Substrate Data), [ADR 022](022-meridian-language.md)
(`.mel` code) and [data-formats.md](../data-formats.md). Open for the user's
decision: section "Solutions and subprojects".

## Context

Meridian has scenes, prefabs, saves, `.meta` sidecars and `.mpak` packs, but no
**project**: nothing says which scene starts, where the assets are, what the
render, physics and input settings are, or what code belongs to the game. Two
things stand in for it today:

- `docs/data-formats.md` lists `meridian.toml` as "project manifest" (TOML,
  schema owner `runtime-app`). It was never implemented, and it contradicts
  ADR 016 (no Meridian format outside Substrate Data).
- `examples/` is one Cargo package with a Rust program per example that stages
  the scene by hand (orbits, light rigs, camera, settings in code). Opening an
  example in an editor means opening a Rust crate.

The target is the Unity workflow: a project is a folder of **data files**; code
is optional and added per need. A person with no Rust toolchain can open a
project, edit a scene, press play. Strings are storage only (typed ids,
references and paths after one parse), as everywhere else.

## Decision

1. **A project is a directory with one `<name>.mproj` file at its root.** The
   directory is the project root; every path in the project is relative to it
   and typed (`ProjectPath`: no `..`, no absolute path, no drive letter).
   Text (people, IDE) or binary (`SDAT`, cooked), detected by magic like every
   other format; `format: 1` first, additive evolution (ADR 016 D5). The IDE
   edits it through `substrate_data::patch`, so comments survive.
2. **Content of `format: 1`.** Every key but `format` and `name` is optional
   with a documented default:

   ```
   {
     format: 1,
     name: "Magic Figures",
     engine: "0.1",                       // EngineVersion requirement, parsed once
     startup: "@asset:scenes/main.mscene", // a scene, typed AssetRef<Scene>
     assets: ["assets"],                  // asset roots (ProjectPath list), default ["assets"]
     settings: {                          // one section per subsystem, see 3
       render:  { bloom: true, shadows: "cascaded" },
       physics: { gravity: [0, -9.81, 0] },
       input:   { actions: "@asset:input/actions.minput" },
     },
     modules: [                           // optional code, see 5
       { kind: "mel",    root: "scripts" },
       { kind: "native", manifest: "code/Cargo.toml" },
     ],
   }
   ```

   `startup` and every asset reference use the existing typed references
   (`@asset:` path, `@asset_id:` id; a scene is an asset like a mesh). The
   reader checks the extension against the expected kind. Unknown key = error
   (a typo must not silently do nothing); an unknown **newer** `format` is
   `UnsupportedFormat`, like `.meta`.
3. **Settings are a registry, not a struct.** The project-domain crate cannot
   name `RenderSettings` or `PhysicsSettings` (that would make it a hub over
   rendering, physics, input). Each subsystem owns its section type (a plain
   struct with `Schema`, like a component) and the composition root
   (`meridian-sdk`) registers `name -> shape` in a `SettingsRegistry`, the same
   pattern as `ComponentRegistry` (ADR 003: a value the application builds, no
   global). A section missing from the file is the type's `Default`; a section
   nobody registered is **kept** as a raw value and written back unchanged
   (the `Keep` rule of unknown components), with a warning at its path: a newer
   engine, a module or another tool may own it. Keys of a **registered**
   section that its type does not know are errors, as for components. `describe()` of the registry
   gives the IDE its property grids, as for components.
4. **Layout is a convention, the file is the truth.**

   ```
   <project>/
     <name>.mproj
     assets/            scenes, prefabs, materials, meshes, textures, audio, each with .meta
     scripts/           .mel code and .meui markup (optional)
     code/              Cargo.toml of a native module (optional)
     build/             cooked output (.mpak, generated .g.rs)   -- git-ignored
     .meridian/         editor caches, thumbnails, per-user state -- git-ignored
   ```

   Nothing derived or per-user is stored in `.mproj`: thumbnails and other
   caches live under `.meridian/`; recent folders and window layout belong to
   the application, not the project. The project file stays reviewable and
   mergeable.
5. **Code is a module, and Cargo is one kind of module.** `modules` lists code
   the project adds on top of the data:
   - `mel`: a folder of `.mel`/`.meui` files (ADR 022): compiled by the
     existing pipeline (Rust backend now, WASM later, loaded by the player).
   - `native`: a Cargo package for Rust code that needs the engine API
     directly. This is the only place Cargo appears; it is never the root of a
     project and a project without it needs no Rust toolchain.
   Which modules a project may use is decided by the host, not the file: the
   file lists them, the CLI refuses a module kind it cannot build or load.
6. **One runner, no per-project main.** `meridian run <project>` and
   `meridian build <project>` (the CLI, owned by Meridian-CLI; the IDE starts it
   as a process and reads its `--message-format json` stream; a thin composition
   root over `meridian-sdk`: `Layers`, `Importers::standard()`, the component and
   settings registries, `WorldFiles`) read the `.mproj`, mount the asset roots
   (or the packs under `build/`), register what the modules provide, load
   `startup` and run the frame loop. A data-only project runs on the prebuilt
   player with no compiler. A `native` module makes `meridian build` produce a
   player that links it (mechanism, static link or plugin library, to be
   decided with team/scripting; it does not change this format).
7. **Crates.** One pure domain crate (depends on `substrate-data` and
   `meridian-asset-domain` only; no file system, no `async`) owns `ProjectFile`, `ProjectPath`, `EngineVersion`,
   `ModuleSpec`, the reader/writer and `SettingsRegistry`; IDLE and the CLI
   both read through it. Its name and home are open: Meridian-CLI already has
   a `meridian-project-domain` (templates, project structure; project-app →
   project-domain ← project-fs), so either `ProjectFile` lives there or this
   crate gets another name, decided with the CLI's owner. File access and
   discovery (`<dir>/*.mproj`) live in `meridian-sdk` next to `WorldFiles`.
   Subsystem crates define their own settings sections and never depend on the
   project crate; only the composition root sees both. `runtime-app` stays
   frame orchestration (it does not own the manifest, which corrects the
   `meridian.toml` row).
8. **Examples become projects.** `examples/<name>/<name>.mproj` plus scene,
   material and asset files, run with `meridian run examples/<name>`. What data
   cannot express yet (procedural orbits and spins, a free-fly camera, a
   benchmark loop) is listed per example in the migration note and moves into
   either a new component or settings section (data) or a `mel` module (code),
   never back into a per-example Rust `main`. A few Rust API demonstrations stay
   as a `meridian-sdk` example package named as such, not as the way to make a
   game. `meridian.toml` disappears from the formats table.

## Solutions and subprojects

Question for the user: does Meridian need a `.msln`-style file grouping several
projects, and nested subprojects?

- **A. `.mproj` only now; reuse through packages later (recommended).** Unity
  has no solution file for content projects (its `.sln` is generated for C#
  code only), and Rust code already has Cargo workspaces. What people actually
  need is sharing content between projects: a reserved `packages` key in
  `format` N+1 (name + source: a path, a `.mpak`, later a registry) covers it
  without a second project file kind. Nothing is built for it now, the key is
  only reserved here so adding it is additive.
- **B. `.mproj` plus `.msln`.** A workspace file listing projects, for editing
  several at once (a game and its tools). Cost: a second format, a second
  discovery rule in the IDE, and rules for settings shared between projects.
- **C. Nested subprojects inside one `.mproj`.** Cost: a project root stops
  being one directory with one meaning; asset ids and `startup` need scoping.
  Not recommended.

A is chosen unless the user decides otherwise; B or C can be added later
without breaking `format: 1`.

## Consequences

- Opening a project is one file; the IDE can show name, startup scene and
  settings without scanning the tree, and "Open Example" is "open this
  `.mproj`".
- Settings are typed per subsystem and validated at load with paths, the same
  way as components. Adding a settings section is a type plus one registration;
  no format bump (an additive section).
- A project with no `modules` runs without a Rust toolchain. A project with
  `native` pays for Cargo only where it opts in.
- The `meridian.toml` row in data-formats.md is removed in the implementing PR.
- New crate and CLI surface to build (the project crate, the `sdk`
  discovery, `meridian run`/`build`); until they land the examples stay Cargo
  programs. The implementation order: the format and crate, `meridian run`
  over a data-only example, the examples one at a time, then modules.
- Open points: how a `native` module is linked (with team/scripting); whether
  `engine` is an exact version or a range (proposal: a semver requirement,
  checked by the CLI, ignored by the reader).
