# `.mres` resource sets

A `.mres` file is a set of named values a game keeps out of its markup and
code: texts, numbers, flags and colors, each with an optional comment. It
is the Meridian counterpart of Yog's `.yres` and of a .NET `.resx`: a build
step turns it into typed constants, and `.mel` attributes refer to entries
as `@res:Set.name`. Decided in
[ADR 017](adr/017-mel-markup-and-code-behind.md); built by
`meridian-mel-build`.

## Where they go

Anywhere under the directory passed to `compile_dir` (for the examples,
`examples/assets/scenes/`). The **set** is named after the file:
`Strings.mres` is the set `Strings`. A set name is also a Rust type name
(letters, digits and `_`, starting with a letter) and is unique across the
directory, whatever subfolder the file is in.

## Format

One Substrate-Data text document (JSON plus comments, trailing commas and
unquoted keys; the grammar is `team/data`'s, see
[data-formats.md](data-formats.md)): an object from names to entries, in
the order they should appear in the generated docs.

```
{
  // A bare string is a text, a bare number or flag is itself.
  title: "Meridian",
  rows: 20,
  show_fps: true,
  // The object form adds a type and a comment.
  accent: { type: "color", value: "#FF8800", comment: "Highlight color" },
  at_sign: "@@home",   // the text "@home"
}
```

| Field | Meaning |
|---|---|
| `value` | Required in the object form. |
| `type` | `string` (default), `number`, `bool`, `color`. |
| `comment` | Shown in the generated docs; never shipped. |

A text starting with `@` is written `@@`, as everywhere in Substrate Data;
`@file:...` or a lone `@` in a text is an error. A color is `#RRGGBB`
(opaque) or `#AARRGGBB`.

A **name** is a dotted identifier (`title`, `screen.title`). It becomes the
constant `SCREEN_TITLE` (snake case, upper-cased, dots as `_`); two names
that give the same constant are an error.

## Generated code

`compile_dir` writes `OUT_DIR/mel/<relative path>.g.rs` for each file; the
code-behind pulls it in with `meridian_sdk::mel_include!("Strings")`
(`"ui/Strings"` for a file in `ui/`):

```rust
pub struct Strings;
impl Strings {
    /// `title` of `Strings`: `Meridian`.
    pub const TITLE: &'static str = "Meridian";
    pub const ROWS: i64 = 20;            // number: i64, or f64 with a fraction
    pub const SHOW_FPS: bool = true;
    pub const ACCENT: u32 = 0xFFFF8800;  // 0xAARRGGBB
}
```

## In `.mel`

A UI attribute whose value is `@res:Set.name` takes the resource's value
when the document is compiled; an unknown set or name is a build error at
the attribute's line. A text starting with `@` stays escaped, so it reads
as the same text. Other references (`@mesh:hero.mesh`) pass through for the
program to resolve; any other leading `@` is an error.

## Not yet

Culture variants (`Strings.ru_ru.mres`) and run-time language switching,
placeholders (`{count}`), file/image/sound entries that bundle assets, a
`Resources`/`x:Key` block inside a `.mel` file with `{StaticResource Key}`,
and `@res:` in tags built by `build_scene`.
