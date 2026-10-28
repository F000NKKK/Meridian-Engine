# MEL grammar

The syntax of the Meridian Engine Language (`.mel`), C#-like with Rust's
ownership, `Result` and data-carrying enums ([ADR 022](adr/022-meridian-language.md)).
It is derived from Yog Script (`yog-mod-loader/docs/design/ys.md`) and is a
draft: golden tests will fix it, and this page changes in the same commit as
the grammar (Substrate-Data's rule for `docs/format.md`). Architecture:
[mel-architecture.md](mel-architecture.md).

Each row is marked **P1** (first parser, golden-tested first), **P2**
(ownership, enums, `Result`) or **P3** (macros, components, tests).

## 1. Lexical

- Source is UTF-8. `//` line comments, `/* */` block comments (nest), `///`
  doc comments (attached to the next item).
- Identifiers: types and members PascalCase, locals and parameters camelCase
  (a convention the linter checks, not the parser).
- Literals: integers (`42`, `0xFF`, `1_000`, suffix `i64`, `u8`), `Fixed`
  (`1.5fx`), floats (`1.5f32`, `1.5f64`), `true`/`false`, `char` (`'a'`),
  strings. A string is as spelled; interpolation `"x = {x}"`, `{{` for a
  brace. Unit literals `20t`, `5s`, `3b` are library-declared suffixes (P3).
- Keywords (reserved): `let mut const class struct record enum interface
  extend component system macro fn new move in ref out where if else switch
  foreach for while return break continue try is as when using module
  public readonly fixed test tune panic default null`.
  Contextual: `has every saved synced`.
- A statement ends with `;`. Blocks use `{ }`.

## 2. Types

| Form | Example | Notes |
|---|---|---|
| Primitive | `i32 i64 u8 u32 f32 f64 bool char string` | `Fixed` is the deterministic number (ADR 008) |
| Named, generic | `List<i32>`, `Dictionary<K, V>`, `Shared<T>` | collections own their items |
| Nullable | `Player?` | only a nullable type can be `null` |
| Result | `Result<T, E>` | `E` defaults to `Error` |
| Tuple | `(i32, string)` | P2 |
| Function | `Func<A, R>`, `Action<A>` | |
| Array/list literal | `[1, 2, 3]` | collection expression |

Value types (`struct`, `record struct`, enums, numbers) are inline and may be
`Copy`; heap types (`class`, `record`, collections) are owned by one owner.

## 3. Declarations

```
file        := using* module? item*
using       := "using" path ";"
module      := "module" path ";" ("fixed")?
item        := attribute* visibility? ("class" | "struct" | "record" | "enum"
               | "interface" | "extend" | function | const | test) 
```

```mel
module Game.Hud;                       // add `fixed` for the deterministic path

const string Title = "Hud";

record Pos(i32 X, i32 Y, i32 Z);       // P1: record = readonly, compared by value
struct Counter(i32 Value);             // P1: value type

enum Mode {                            // P2: a case may carry data
    Idle,
    Charging(i32 Rate),
    Full,
}

class HudViewModel {                   // P1
    public string Title = "Hello";
    public i32 Count { get; private set; }     // property
    public Handled OnClick(in UiEvent e) {     // handler named by a .meui
        Count += 1;
        return Handled.Yes;
    }
}

i32 Area(i32 w, i32 h) => w * h;       // P1: types first
```

- Primary constructors: `class Timer(Ticks every) { ... }`, parameters in
  scope in the body; `new Timer(5s) { Left = 1t }`, `new(...)` when the type
  is known.
- `class` inherits one class and implements interfaces; `struct` and `record`
  do not inherit; interfaces may have default bodies. P2.
- Generics with constraints: `T Max<T>(T a, T b) where T : IComparable<T>`.
- `extend Player { ... }` adds members to any type. P2.

## 4. Ownership (P2)

Rust's rules, C#'s spelling. Every heap value has one owner; the checker
tracks owners, moves and borrows non-lexically.

| Parameter | Call | Rust |
|---|---|---|
| `T x` | `F(x)` (copy) or `F(move x)` | `T` |
| `in T x` | `F(x)` or `F(in x)` | `&T` |
| `ref T x` | `F(ref x)`, `x` a `let mut` | `&mut T` |
| `out T x` | `F(out let x)` | assigned by the callee |
| `move T x` | `F(move x)` | `T`, ownership taken |

- `let x = ...` is immutable, `let mut x = ...` mutable. `move x` ends `x`;
  using it afterwards is an error (`MEL04xx`). Copies are implicit only for
  `Copy` values.
- Any number of `in` borrows or one `ref` at a time; a borrow never outlives
  its owner. Lifetimes are inferred and elided at signatures; written
  `'a` only when ambiguous: `in 'a Item First<'a>(in 'a List<Item> items)`.
- A scope end drops the value (`IDisposable.Dispose()` runs). No GC;
  shared ownership is `Shared<T>` / `Weak<T>`.
- A `readonly` method borrows `this` read-only; a `move` method consumes it.

## 5. Expressions

Precedence, lowest to highest (each operator is a concept with a binding
power, so this table is data):

`=` `+=` ... (right) · `??` · `||` · `&&` · `==` `!=` · `<` `>` `<=` `>=`
`is` · `|` · `^` · `&` · `<<` `>>` · `+` `-` · `*` `/` `%` · unary `! - ~
move in ref try` · postfix `.` `?.` `()` `[]` `!` (non-null) · primary.

- `a ?? b`, `a?.b` for nullable. `x as T`, `x is T`, `x is Mode.Charging(let r)`.
- Lambdas: `x => x * 2`, `(a, b) => a + b`, trailing block `list.Select { it * 2 }`
  with implicit `it`. A `move` lambda takes ownership of its captures.
- `new T(...)`, collection `[1, 2]`, string interpolation.
- `try expr` passes an `Err` up (Rust's `?`), converting with `From`. P2.
- `switch` expression, exhaustive:

```mel
let label = mode switch {
    Mode.Idle => "idle",
    Mode.Charging(let rate) when rate > 3 => "fast",
    Mode.Charging(_) => "slow",
    Mode.Full => "full",
};
```

Patterns: constants, `_`, `let name`, enum cases with fields, records
(`Pos(0, let y, _)`), properties (`{ Health: < 5 }`), lists
(`[let first, .., let last]`), relational and range (`> 3`, `1..10`),
`and`/`or`/`not`, guards with `when`. A `switch` over an enum covers every
case or has `_`; a missing case is an error. P2.

## 6. Statements

`let`/`let mut`, assignment, `if`/`else`, `while`, `for`, `foreach (let x in
xs)` (`foreach (let mut x in xs)` lends each item mutably), `return`, `break`,
`continue`, expression statements, blocks. P1. `switch` as a statement,
`using` scopes, `panic!(...)` P2.

## 7. Failures (P2)

Expected failures are `Result<T, E>` (`Ok(v)` / `Err(e)`, `try`, `Map`,
`MapErr`, `AndThen`, `UnwrapOr`); bugs panic (`panic!`, `Unwrap()` of an `Err`
or `null`, index out of range, checked overflow). There are no exceptions. A
panic unwinds to the handler boundary; the host disables that handler and
reports the `.mel` line.

## 8. Attributes, macros, tests (P3)

- Attributes are C#'s: `[Obsolete("...")]` on declarations; classes deriving
  `Attribute`; `[Derive(Clone, Equals)]`.
- Macros are Rust's, called with `!`: declarative `macro Name { ($x:expr) =>
  {...}; }` and procedural macros written in MEL and run by the compile-time
  evaluator. Expansion errors point at the call.
- `test "name" { ... }` runs in the compile-time evaluator (or the headless
  engine); `tune i32 Rate = 4;` is a live value an editor can change.

## 9. Components and systems (P3, last)

```mel
component Charge(Energy capacity) {
    public saved Energy Energy in 0..capacity = 0;
}
system Solar(Charge charge, Block block) every 1t { ... }
```

A `system` is a generated query over `ecs` archetypes, not a scan. Whether
this stays in the language or becomes a library plus macros is open
(mel-architecture.md).

## 10. Pairing with `.meui`

`x:Class="Game.Hud.HudViewModel"` in a `.meui` names a `.mel` class; its
public members are the binding targets of `{x:Bind}` and event handlers
(`click="{x:Bind OnClick}"`), type-checked at compile time. Both compile to
Rust, and errors point at the `.mel` or `.meui` line.

## Diagnostic codes

`MEL0001-0099` lexical, `0100-0199` syntax, `0200-0299` names, `0300-0399`
types, `0400-0499` ownership and borrows, `0500-0599` patterns and enums,
`0600-0699` macros, `0700-0799` backend ("not supported by this backend").
