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

## Resources inside a `.mel` file

An element of a UI document may carry a `Resources` block, as in XAML:

```xml
<StackPanel width="{StaticResource Wide}">
  <StackPanel.Resources>
    <Number x:Key="Wide" Value="300" />
    <String x:Key="Title" Value="Settings" />
    <Color x:Key="Accent" Value="#FF8800" />
    <Bool x:Key="Compact" Value="false" />
  </StackPanel.Resources>
  <TextBlock text="{StaticResource Title}" />
  <TextBlock text="{StaticResource Strings.title}" />  <!-- a .mres entry -->
</StackPanel>
```

A resource is `<String|Number|Bool|Color x:Key="Key" Value="..." />` (same
value rules as `.mres`); the key is a plain identifier, unique in its
block. `{StaticResource Key}` is replaced at build time by the value of the
nearest block, from the element itself outwards, that defines the key; a
key written `Set.name` is the `.mres` entry, so `{StaticResource
Strings.title}` and `@res:Strings.title` are the same. An unknown key, a
bad block or a second block on one element is a build error at its line.

## Culture variants

`Strings.ru.mres` is the Russian variant of `Strings.mres`: the part after
the set name is a culture tag (letters and digits, `-` between parts, any
case; stored lowercase: `ru`, `en-gb`). A variant may define only entries
the default set has, with the same type (a number stays a number); what it
leaves out falls back to the default, and `en-gb` falls back to `en` and
then to the default. A variant without a default set, an unknown entry, a
type mismatch, a bad tag or two files for one culture is a build error at
its file. Variants generate no constants: `Strings::TITLE` is the default
value, and `@res:Strings.title` in a `.mel` compiled by the build script is
the default too.

The language switch is a run-time reload: `HotDocument::set_culture(Some(
Culture::parse("ru")))` makes the next `refresh` read `@res:` references
through `ResourceSets::for_culture` and patch the tree, with no file
change needed; `set_culture(None)` goes back to the default.

## Reloading

`HotDocument::open(mel, Some(resources_dir), root, debounce)` watches the
`.mel` and every `.mres` under the directory; the host calls
`refresh(tree, Instant::now())` each frame. A changed file is parsed
again with the new `ResourceSets` (resource values are resolved while
parsing, so a changed entry reaches the attributes that use it), and the
tree is patched in place or, when the shape changed, built again
(`Refreshed::Patched` / `Refreshed::Rebuilt`). A file that does not parse
is an error and leaves the tree as it was; the next change tries again.

## Not yet

Culture-aware lookup from code (generated constants are the default
value), placeholders (`{count}`), file/image/sound entries that bundle assets,
styles and templates (`{StaticResource}` of a whole `Style`), run-time
lookup (`{DynamicResource}`), and resources in tags built by
`build_scene`.
