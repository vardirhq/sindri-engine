# The Decay language reference

Decay is a small, statically-typed scripting language for gameplay in the Sindri
Next engine. This document describes the language **as it actually behaves
today**, verified against the implementation rather than the intent.

It is written to be read by people and by AI assistants. Both make the same
mistake with a young language: assuming it works like the mature one it
resembles. Decay looks like Rust. It is much smaller than Rust. Where something
does not exist, this document says so explicitly rather than leaving it out —
see [What does not exist](#what-does-not-exist) and
[Surprising behaviour](#surprising-behaviour), and treat those two sections as
the most important ones here.

**Status: unstable.** The syntax, type model, IR, and runtime are all expected
to change. Nothing here is a compatibility promise.

---

## Contents

- [A complete example](#a-complete-example)
- [Lexical structure](#lexical-structure)
- [Grammar](#grammar)
- [Types](#types)
- [Lists](#lists)
- [Vectors](#vectors)
- [Text](#text)
- [Containers](#containers)
- [Fields](#fields)
- [Functions](#functions)
- [Events](#events)
- [State](#state)
- [Shared functions](#shared-functions)
- [Constants](#constants)
- [Enums](#enums)
- [Structs](#structs)
- [Statements](#statements)
- [Expressions](#expressions)
- [Scope](#scope)
- [The host boundary](#the-host-boundary)
- [Execution limits](#execution-limits)
- [Surprising behaviour](#surprising-behaviour)
- [What does not exist](#what-does-not-exist)
- [Diagnostics](#diagnostics)

---

## A complete example

Every Decay feature that exists, in one file.

```rust
// Comments run to the end of the line. There are no block comments.

script PlayerController {
    // An authored property. `let` means the script never reassigns it;
    // `@export` means the host may set it before the script starts.
    @export
    let speed: f32 = 6.0;

    @export
    let label: String = "player";

    // Instance state. `var` may be reassigned. It survives between calls and
    // is not written back to the scene.
    var elapsed: f32 = 0.0;
    var airborne: bool = false;

    // Runs once, before the first `update`, after authored properties land.
    fn start() {
        elapsed = 0.0;
    }

    // Runs once a frame. `dt` is the frame's delta in seconds.
    fn update(dt: f32) {
        elapsed += dt;

        // A local binding. `let` is fixed, `var` may be reassigned.
        let offset: f32 = wave(elapsed) * speed;

        // Host paths. Decay does not know what a transform is; the host does.
        this.transform.position.x = offset;

        if offset > 0.0 {
            airborne = true;
        } else {
            airborne = false;
        }
    }

    // Functions may call each other by bare name, and may recurse.
    fn wave(seconds: f32) -> f32 {
        return sin(seconds);
    }
}
```

---

## Lexical structure

### Comments

`// to end of line`. That is the only comment form. **There are no `/* */` block
comments** and no doc comments.

### Identifiers

ASCII only: a letter or `_`, followed by letters, digits, or `_`. No Unicode
identifiers, and no raw identifiers.

### Keywords

```text
script  component  fn  let  var  if  else  while  for  in  break
continue  return  true  false  null  match
```

All seventeen are reserved. Six words are special only where no other name
could stand: `event`, `state`, `enum` and `struct` at the start of an item, `shared` right before
`fn`, and `on` followed by a name at the start of a member (see
[Events](#events), [State](#state) and [Shared functions](#shared-functions)). Elsewhere they are ordinary
identifiers, so `Bolt.on(hit)` and a local called `event` are unaffected.

### Number literals

```text
0        1        42        6.0       0.25
```

Digits, optionally followed by `.` and more digits. **There is no** exponent
notation (`1e6`), hex or binary (`0xff`), digit separator (`1_000`), leading dot
(`.5`), trailing dot (`1.`), or numeric suffix (`1.0f32`).

A negative number is the unary `-` operator applied to a literal, not part of
the literal.

Every number is the same type whether it is written `7` or `7.0`.

### String literals

Double quotes. Escapes: `\n`, `\r`, `\t`, `\"`, `\\`. Any other escape is a
diagnostic, and the character is taken literally. There are no raw strings, no
multi-line strings, no interpolation, and no single-quoted characters.

### Operators and punctuation

```text
+   -   *   /   %
+=  -=  *=  /=  %=  =
==  !=  <   <=  >   >=
&&  ||  !
->  @   .   ..  ,   ;   :
=>  |
(   )   {   }   [   ]
```

`[` and `]` write a list and index one; `..` joins a range's two ends, and
appears only in a `for`.

---

## Grammar

```ebnf
program      = { item } ;
item         = ( "script" | "component" ) IDENT "{" { member } "}"
             | "event" IDENT "(" [ params ] ")" ";"
             | "state" IDENT "{" { field } "}"
             | "enum" IDENT "{" [ IDENT { "," IDENT } [ "," ] ] "}"
             | "struct" IDENT "{" [ struct_field { "," struct_field } [ "," ] ]
               { function } "}"
             | [ "shared" ] function
             | [ "shared" ] "const" IDENT ":" type "=" expr ";" ;
struct_field = IDENT ":" type [ "=" expr ] ;
member       = { attribute } ( field | function | handler ) ;
attribute    = "@" IDENT ;

field        = ( "let" | "var" ) IDENT [ ":" type ] [ "=" expr ] ";" ;
function     = "fn" IDENT "(" [ params ] ")" [ "->" type ] block ;
handler      = "on" IDENT "(" [ params ] ")" block ;
params       = param { "," param } ;
param        = IDENT [ ":" type ] ;
type         = IDENT [ "<" type ">" ] ;

block        = "{" { stmt } "}" ;
stmt         = binding | return | if | while | for | match | break
             | continue | block | expr ";" ;
binding      = ( "let" | "var" ) IDENT [ ":" type ] [ "=" expr ] ";" ;
return       = "return" [ expr ] ";" ;
if           = "if" expr block [ "else" ( block | if ) ] ;
while        = "while" expr block ;
for          = "for" IDENT "in" ( expr | expr ".." expr ) block ;
match        = "match" expr "{" { arm } "}" ;
arm          = pattern { "|" pattern } "=>" block ;
pattern      = IDENT "." IDENT | "_" ;
break        = "break" ";" ;
continue     = "continue" ";" ;

expr         = assign ;
assign       = binary [ ( "=" | "+=" | "-=" | "*=" | "/=" | "%=" ) assign ] ;
binary       = unary { binop unary } ;         (* see the precedence table *)
unary        = [ "-" | "!" ] unary | postfix ;
postfix      = primary { "." IDENT | "[" expr "]" | "(" [ args ] ")" }
             | IDENT "(" IDENT ":" expr { "," IDENT ":" expr } [ "," ] ")" ;
args         = expr { "," expr } ;
primary      = IDENT | NUMBER | STRING | "true" | "false" | "null"
             | "(" expr ")" | "[" [ expr { "," expr } [ "," ] ] "]"
             | "match" expr "{" value_arm { "," value_arm } [ "," ] "}" ;
value_arm    = pattern { "|" pattern } "=>" expr ;
```

Note that `if` and `while` take a **block**, not a statement: `if x > 0.0 { }`,
never `if x > 0.0 doThing();`. Parentheses around the condition are allowed but
do nothing — they parse as an ordinary grouping expression — so write
`if x > 0.0 { }` rather than `if (x > 0.0) { }`.

`else if` is the one exception to `else` taking a block, and it is only a
spelling: the parser builds `else { if ... }`, so a chain is nested blocks and
nothing downstream sees a second kind of conditional.

Attributes are permitted only on fields. An attribute on a function or a
handler is a diagnostic.

---

## Types

| Written | Meaning |
| --- | --- |
| `f32` | The only numeric type |
| `bool` | `true` or `false` |
| `String` or `string` | Text; see [Text](#text) |
| `unit` or `void` | No value; the default return type |
| `Vec2`, `Vec3` | Two or three numbers that travel together; see [Vectors](#vectors) |
| `List<T>` (or `Array<T>`) | Several `T`, in order; see [Lists](#lists) |
| a declared `struct` or `enum` | See [Structs](#structs) and [Enums](#enums) |
| anything else | A **named host type**, opaque to Decay |

There is no `i32`, no `u32`, no `f64`, and no integer type of any kind. `7` and
`7.0` are the same value, and `7 / 2` is `3.5`.

> **`f32` holds an `f64`.** The type is spelled `f32` because engine transforms
> are `f32`, but every Decay value is stored as a 64-bit float and narrows only
> when it crosses into the engine. This is a known inconsistency and an open
> decision, recorded in `docs/decay-direction.md`.

A named type such as `Transform` is opaque to the language: Decay has no way to
declare one. The **host** may describe it, and whether it has decides what
happens after a dot:

- **Described** — members are checked. `this.transform.positon.x` is a compile
  error naming the type and the member, and the member's type is enforced like
  any other, so assigning a number to a `bool` field is caught.
- **Not described** — members are `Unknown`, and unknown is compatible with
  everything, so anything after the dot is accepted and a mistake surfaces at
  runtime.

The rule is **per type**, which is what makes describing a host gradual: a host
part-way through describing itself does not reject scripts working against the
parts it has not reached. Describing `Transform` while leaving `RigidBody`
undescribed means `this.transform.positon` is caught and
`this.rigidbody.anything.at.all` is not.

`this` follows the same rule. Once the host describes anything on `this`, a
member of `this` it did not describe is a compile error — so against Sindri,
which offers only `transform`, `this.sprite` is refused. What Sindri describes
is in `docs/scripting.md`.

`null` may be assigned to a named type, and to nothing else.

### Host references

A host may say that one named type is also another. Sindri says each script
type is also an `Entity`, so a `Bolt` goes wherever an `Entity` does and
compares with one; an `Entity` is not a `Bolt` until the host says so. The
language has no opinion about what either means.

A value typed as a script is not `this`, even inside that script: reaching
through one reaches what the host describes for that type, so calling one of
its functions is whatever the host makes of a call through a reference —
against Sindri, a message — while `this.helper()` is still refused.

#### Held references

A value of a named host type can be **held**, not only reached through. A host
may hand one back from a call, and a script can bind it, keep it in a field,
pass it, and compare it:

```decay
let target = World.find("Player");
if target != null && target != this.entity {
    target.transform.position.x = 0.0;
}
```

What a script cannot do is look inside one. There is no literal for a reference,
no arithmetic on one, and no conversion in either direction; the only references
a script holds are ones the host gave it. Whether a reference still names
anything is the host's question to answer — the language has no opinion on
whether the thing behind one is alive, and against Sindri that is
`World.exists`.

A path may start from a reference however it is held: a local, a field
(`this.target.transform`), or a value computed on the spot
(`World.find("Player").transform`). A computed one is evaluated once, before
any argument, and the rest of the path is walked from it.

This is what a reference is *for*: it is the difference between a script that
can only describe itself and one that can say something about another thing in
the world. `docs/scripting.md` records what Sindri makes of it.

### Lists

`List<T>` holds several values of one type, in order. It is the one type that
takes a type argument; `Array<T>` is its older spelling and means the same.

```rust
var ids = [3.0, 1.0, 4.0];               // a List<f32>
var held: List<Entity> = [];             // an empty one needs its type
let enemies = World.with_tag(this.enemy); // the host hands one back

ids.push(5.0);                           // at the end
ids.insert(0, 9.0);                      // at a position, moving the rest
let last = ids.pop();                    // off the end
let first = ids.remove_at(0);            // out of a position
ids[1] += 10.0;                          // an element, set in place
ids.clear();

let count: f32 = enemies.length;
if held.contains(enemies[0]) { }
let where = ids.index_of(4.0);           // or -1
for enemy in enemies { enemy.transform.position.y -= 1.0; }
for i in 0..count { }                     // 0, 1, ... count - 1
```

A literal's type is its first element's, and every other element must fit it.
An empty `[]` fits any list, so it is written where the type is: a field or
binding annotated `List<T>`. A field whose initializer is a list of literals
has that list's type without an annotation.

**A list is a value, like a vector.** Assigning one copies it: `var b = a;
b.push(1.0);` leaves `a` as it was. So a change — `push`, `pop`, `insert`,
`remove_at`, `clear`, `xs[i] = v` — is made to the list a variable, parameter
or field of this script holds, and that one must be a `var`. Changing a `let`
list, or one that is not in a variable at all (`World.with_tag("x").push(e)`),
is a diagnostic. A change is always a call — `xs.pop()`, not `xs.pop` — while
`length` is a property, as it is for text and vectors (`len`, its first
spelling, still works). The copy is made only
when a change needs it, so a list that is never shared is changed in place.

**Indices are the one numeric type.** There is no integer type, so a whole
number is a property of the *value*, checked where the value is: `items[1.5]`,
`items[-1.0]`, and `items[7.0]` on a list of three are three different runtime
errors, each naming what was wrong, and so is `pop` on an empty list.
`insert` takes any position up to the length. Introducing an integer type for
indexing alone would rewrite every signature in the language and at the host
boundary; `docs/decay-direction.md` records that trade.

A script may put at most 10,000 elements in one list: `push` or `insert` past
that fails with `ListTooLong`. A list the host hands back is not held to it.

A `for` walks the list it was given, as it was when the loop began: changing
the list, or the name it came from, inside the loop does not change what is
being walked. A `for` binding is immutable; to change an element, index it.

**A range** is written only as what a `for` walks: `for i in start..end`
walks the numbers `start`, `start + 1`, … while they are below `end`, so
`0..3` is `0, 1, 2` and `3..3` is nothing. Both ends are numbers and are
evaluated once. A range is never made into a list, so `0..1000000` costs two
numbers; walking it is bounded by the operation budget like any loop.

### Vectors

`Vec2` and `Vec3` are the language's own value types: a position, a direction,
a velocity. They are built by a script, not handed out by a host.

```rust
var velocity: Vec2 = Vec2(3.0, 4.0);
let up = Vec3(0.0, 1.0, 0.0);

velocity.x += 1.0;                 // components are x, y, and z for Vec3
let speed: f32 = velocity.length;  // 5.0 before the line above
let heading = velocity.normalized; // same direction, one unit long
```

| Written | Means |
| --- | --- |
| `Vec2(x, y)`, `Vec3(x, y, z)` | Builds one from numbers |
| `v.x`, `v.y`, `v.z` | One component, readable and assignable |
| `a + b`, `a - b` | Component by component; both the same size |
| `v * n`, `n * v`, `v / n` | Scaled by a number |
| `-v` | Pointing the other way |
| `a == b`, `a != b` | Equal when every component is |
| `v.length` | How long it is |
| `v.normalized` | One unit long; a zero vector stays zero |
| `a.dot(b)` | The dot product |
| `a.distance(b)` | How far apart two points are |
| `a.lerp(b, t)` | The point `t` of the way from `a` to `b` |

Compound assignment works on a whole vector as it does on a number:
`position += velocity * dt`.

**A vector is a value, copied like a number.** `var b = a; b.x = 9.0;` leaves
`a` alone. Changing a component is changing the variable or field that holds
the vector, so `a.x = 1.0` needs `a` to be a `var`, and the component of a
temporary — `(a + b).x = 1.0`, `f().x = 1.0` — cannot be assigned.

**What is not allowed is refused, not guessed at.** A `Vec2` and a `Vec3` do
not mix. A vector times a vector has no single obvious meaning, so it is an
error that points at `dot`; dividing a number by a vector, `%` on a vector,
and `<` between vectors are errors too. `length` and `normalized` are
properties and are not called; `dot`, `distance` and `lerp` are methods and
are.

A host may hand a vector out and take one back: a host member typed `Vec3` is
reached whole — `this.transform.position` — and `this.transform.position.x`
is still the one path the host answers, exactly as before vectors existed.

`Vec2` and `Vec3` are reserved type names; `Vec2(…)` and `Vec3(…)` are the
only calls the language answers itself rather than the host.

---

### Text

Text is joined with `+`, and asked questions with a dot:

```rust
let label = "Score " + score + " / " + target;   // "Score 3 / 5"
if name.starts_with("enemy_") { }
Game.set(stat + "_add", 0.0);
```

**`+` joins** when either side is text, and the other side may be anything
with one obvious spelling: text, a number, a `bool`, a vector or an enum's
variant. A whole number is written without a decimal point (`3`, not `3.0`);
any other as briefly as reads back as the same number (`0.1`, `2.5`). A
vector is `(1.5, -2)`. A variant is its own name, `Lobby`, without its
enum's. A reference, a timer, a collection or `null` has no spelling, and
joining one is a diagnostic. Nothing but `+` works on text: `-`, `*`, `/`,
`%` and the orderings `<` `>` are diagnostics, and `+=` joins onto a text
variable.

Positions and lengths count characters, not bytes. Its properties and methods
are all there is:

| Written | Gives |
| --- | --- |
| `s.length` | `f32`, how many characters |
| `s.uppercase`, `s.lowercase` | `String` |
| `s.trimmed` | `String`, without spaces at either end |
| `s.contains(part)`, `s.starts_with(part)`, `s.ends_with(part)` | `bool` |
| `s.find(part)` | `f32`, where `part` first starts, or `-1` |
| `s.slice(start, end)` | `String`, characters `start` up to, not including, `end` |
| `s.replace(old, new)` | `String`, every `old` replaced |

Like a vector's, one that takes no arguments is a property: `s.length()` is a
diagnostic. A `slice` position is a whole number like an index — a fraction is
refused when it runs — but one outside the text is held to it, so
`s.slice(0, 3)` of a two-character `s` is all of it and a backwards range is
empty. Replacing `""` changes nothing.

Text is a value, and no longer than 64 KiB: joining past that fails with
`TextTooLong`, because the operation budget counts a join as one step however
long it is, and `s = s + s` in a loop doubles.

A field whose initializer is a text literal is `String` without an annotation.

**A number is written a set way** by asking it:

| Written | Gives |
| --- | --- |
| `n.fixed(digits)` | `String` with exactly `digits` decimals: `12.5.fixed(2)` is `"12.50"`, `7.6.fixed(0)` is `"8"` |
| `n.padded(width)` | `String` of the nearest whole number, led with zeros to `width` digits: `7.0.padded(3)` is `"007"`, `-7.0.padded(3)` is `"-007"` |

Both round half away from zero, and neither ever writes `-0`. `digits` is a
whole number from 0 to 9 and `width` one from 0 to 20; anything else fails
with `DigitsOutOfRange` when it runs. A number that is not finite is spelled
as joining would spell it. These are the only members a number has, and they
are calls: `n.fixed` alone is a diagnostic.

```rust
let clock = minutes.fixed(0) + ":" + seconds.padded(2);   // "1:05"
```

### Timers

A `Timer` is a countdown, and a value like a number:

```rust
script Mine {
    var cooldown = Timer(0.0);          // already run out
    fn update(dt: f32) {
        if !cooldown.done { return; }
        cooldown = Timer(2.4);          // start again
    }
}
```

`Timer(seconds)` starts one; a negative duration starts one that has already
run out. It has four properties and nothing else: `done` (`bool`), `left`
(seconds to go, never below zero), `duration` (the seconds it started with)
and `progress` (from `0` when started to `1` when done). A timer is **read,
never written into** — `t.left = 1.0` is a diagnostic — and restarted by
assigning a new one. It takes part in no arithmetic.

**When time passes is the host's decision**; the language has no clock. The
runtime offers `ScriptInstance::advance_timers(seconds)`, which runs down
every timer the instance's fields hold. A timer in a local lives for one call
and is never run down. Against Sindri, a script's timers run down by each
frame before its `update`, and not on the frame it starts; see
`docs/scripting.md`.

A field whose initializer is `Timer(...)`, `Vec2(...)` or `Vec3(...)` has that
type without an annotation. Every other unannotated field is unknown, as
before.

---

## Shared functions

A function written outside any container belongs to its file: every script in
that file may call it by name. Marked `shared`, it belongs to the project
instead, and every script in every file may call it.

```rust
fn third(size: f32) -> f32 { return size / 3.0; }        // this file's
shared fn half(size: f32) -> f32 { return size * 0.5; }  // everyone's

script Enemy {
    let view_size: f32 = 11.0;
    fn update(dt: f32) { let edge = half(this.view_size) + third(1.0); }
}
```

- **No `this`.** A function outside a container belongs to no script, so it
  has no fields and no `this`; what it needs, it is passed. Using `this` in
  one is a diagnostic that says so.
- **Called by bare name**, checked like any call. A script's own function of
  the same name wins inside that script, and a file's own function wins over
  a shared one of the same name from another file.
- **One name, once.** Declaring one twice is a diagnostic, and so is a name a
  container, an event, a state or the host already has.
- `shared` is special only right before `fn`; elsewhere it is an ordinary
  name.

**Across files is the host's decision.** The language lowers a file's
top-level functions into its program, and `IrProgram::link` adds another
program's; a bare call tries the script's own functions, then the program's
top-level ones, then the host. A host that describes other files' `shared fn`
to the analyzer (`Environment::add_shared_function`) and links them makes
them callable from every file, which is what Sindri does for a project.

---

## Constants

A constant names a value worked out when the file compiles. Declared outside
any container, it is usable anywhere in its file; marked `shared`, anywhere in
the project.

```rust
enum Phase { Lobby, Play }

shared const VIEW_SIZE: f32 = 11.0;               // everyone's
const HALF: f32 = VIEW_SIZE / 2.0;                // this file's
const FIRST: Phase = Phase.Lobby;
const TITLE: String = "Orbital" + " Baked";
const WIDE: bool = HALF > 4.0 && !false;

script Enemy {
    var phase: Phase = FIRST;
    fn update(dt: f32) { if this.transform.position.x > HALF { phase = Phase.Play; } }
}
```

- **Always typed, always given a value.** A constant holds `f32`, `bool`,
  `String` or an enum; a vector, a list, a struct or an entity is a
  diagnostic.
- **Worked out when the file compiles.** Its value may use literals, the
  operators, an enum's variants and other constants, in any order in the file
  and across files for shared ones. It may not call anything, read a field,
  or name a local, so its value is the same every time the game runs. Text
  joins only to text in a constant (`"a" + "b"`); join a number to one where
  the constant is used. Dividing by zero, and constants that need each other,
  are diagnostics.
- **Never changes.** Assigning one is a diagnostic. Every use of a constant is
  its value written in place, so it costs what the literal would.
- **One name, once.** Declaring one twice in a file is a diagnostic, and so is
  a name a container, an event, a state, an enum, a struct, a function or the
  host already has. A local of the same name hides it, as it hides a field.
- `const` is special only at the start of an item or right after `shared`
  there.

**Across files is the host's decision**, as for shared functions. A host that
works out every file's `shared const` together (`fold_constants`) and
describes them to the analyzer (`Environment::add_constant`) makes them usable
in every file, which is what Sindri does for a project; a file's own
constant of the same name wins in that file.

---

## Containers

A file holds any number of `script` and `component` declarations. Names must be
unique across the file.

```rust
script Enemy { }
component Health { }
```

Both parse identically and hold the same members. The distinction is carried
through to the IR as `ContainerKind` for the host to act on; **the language
itself treats them the same**, and `sindri-decay` currently instantiates scripts
only.

---

## Fields

```rust
let speed: f32 = 6.0;      // fixed after initialization
var elapsed: f32 = 0.0;    // reassignable
@export let jump: f32 = 8.0;
```

A field needs a type, an initializer, or both. With neither, it is a diagnostic.

A field with no initializer starts as `null`, except a `Vec2` or `Vec3` field,
which starts at zero.

Field initializers run in **declaration order**, and may read fields declared
**above** them:

```rust
let base: f32 = 2.0;
let doubled: f32 = base * 2.0;   // fine
```

Reading a field declared *below*, or reading the field itself, is a compile
error naming both fields. It used to compile and fail at runtime with
`UnknownPath`, which reported a path rather than the field that could not have
had a value yet.

### `@export`

`@export` marks a field as authored by the host. It is the only attribute that
exists.

The host may set an exported field regardless of `let` or `var`: `let` means the
*script* does not reassign it, not that the author cannot author it. This is the
`[SerializeField]` distinction from other engines, and it is the capability that
justified Decay being statically typed — see `docs/scripting.md`.

A host that sets a field which is not `@export`, or does not exist, is refused.

---

## Functions

```rust
fn name(a: f32, b: bool) -> f32 { return 0.0; }
fn no_return(dt: f32) { }
```

An omitted return type is `unit`. An omitted parameter type is unknown, which is
compatible with everything — annotate parameters.

A function with no `return` returns unit. `return;` with no value also returns
unit.

Functions in the same container call each other **by bare name**:

```rust
fn helper() -> f32 { return 1.0; }
fn go() -> f32 { return helper(); }     // correct
```

> **`this.helper()` is not a method call.** There are no methods on a container.
> It used to become a *host path call* named `this.helper` and fail at runtime
> with `FunctionNotFound`, which looked like the engine's fault; it is now a
> compile error saying to write `helper(...)` instead. Always call sibling
> functions by bare name.

A **host type** may have methods, and those are checked: if the host describes
`RigidBody` with an `add_impulse` taking two numbers, then
`this.rigidbody.add_impulse(0.0, 1.0)` is checked for arity and argument types
like any other call.

Recursion works, bounded by the call-depth limit.

### `this`

`this` is bound in every function to the container's own type. Two uses work:

- `this.<field>` reads or writes one of the container's own fields. `speed` and
  `this.speed` are the same field.
- `this.<anything else>` is a **host path** — `this.transform.position.x` means
  nothing to Decay and everything to the host.

### Lifecycle

The language has no lifecycle of its own; the host decides which functions it
calls. `sindri-decay` calls `start()` once and `update(dt)` each frame, both
optional and both with exact signatures. See `docs/scripting.md`.

---

## Events

An event is something that happens, declared once at the top level of a file,
emitted by any script, and handled by any script:

```rust
event GoalScored(team: f32);

script Ball {
    fn update(dt: f32) { GoalScored.emit(1.0); }
}

script Match {
    var blue: f32 = 0.0;
    on GoalScored(team) { if team == 1.0 { blue += 1.0; } }
}
```

- **Declaring.** Every parameter of an event must be typed. Its name may not be
  one a container or the host already uses, and a file may declare it once.
- **Emitting.** `Name.emit(values)` is checked like a call against the
  declaration and returns unit. An event is not a function: `Name(values)` is a
  diagnostic that says to write `.emit`. A local of the same name shadows it.
- **Handling.** `on Name(params) { }` in a container. The handler must take as
  many values as the event carries; a parameter's type may be left unwritten
  and is then the event's, and a written one must match it. A handler returns
  nothing and cannot be called by name.

**When a handler runs is the host's decision**, as the lifecycle is. The
language lowers `Name.emit(...)` to a call on the host path `Name.emit` and a
handler to a function named `on Name`, which no script can write. Against
Sindri, an emit is queued and delivered after the pass to every running
script with a handler; see `docs/scripting.md`.

A host may declare events of its own — Sindri declares every event in the
project, so a file handles one declared in another — through the environment.
One the host reports as declared more than once cannot be used.

---

## State

A `state` declares values every script shares, reached by name:

```rust
state Tuning {
    let gravity: f32 = 30.0;
    var paused = false;
}

script Body {
    fn update(dt: f32) {
        if !Tuning.paused { this.transform.position.y -= Tuning.gravity * dt; }
    }
}
```

- **Fields only.** Each holds an `f32`, a `bool` or an enum, and its
  initializer is a literal — a number, a negated number, `true`, `false` or a
  variant such as `Phase.Lobby` — since it is set up before any script runs. A
  written type must match the literal.
- **`let` is fixed**: assigning to one is a diagnostic. `var` may be written
  from anywhere.
- **Declared once per field.** Several declarations may add fields to the same
  name; a field declared twice is a diagnostic. A state may not take a
  container's or an event's name. It may share a name with a namespace the
  host offers — Sindri's `Game` — as long as it redefines none of its
  members; its fields are then found first.
- A misspelt field is a diagnostic, as for any described type. A local of the
  same name shadows the state.

**Where the values live is the host's decision.** `Tuning.gravity` lowers to
the host path `Tuning.gravity`, as a read of any host value does. Sindri keeps
them on its board; see `docs/scripting.md`. A host describes a project's
state to a file through the environment, as it does events.

---

## Enums

An `enum` names a fixed set of values, so a script says what it means where it
would otherwise hold a number and a comment:

```rust
enum Phase { Lobby, Countdown, Play }

script Match {
    var phase = Phase.Lobby;
    fn update(dt: f32) {
        match this.phase {
            Phase.Lobby => { if ready() { this.phase = Phase.Countdown; } }
            Phase.Countdown | Phase.Play => { run(); }
        }
    }
}
```

- **A variant is always written with its enum's name**: `Phase.Lobby`, never
  `Lobby`. The enum's name is a type, so `let p: Phase = Phase.Play;` and
  `fn go(p: Phase)` work, and an unannotated field takes its type from a
  variant initializer.
- **Only `==` and `!=`.** A variant is no number: it takes part in no
  arithmetic and no ordering, and comparing variants of two different enums is
  a diagnostic.
- **Declared once.** A variant declared twice, an enum with no variants, and
  an enum that takes a name something else already has are diagnostics.
  Calling an enum like a function is a diagnostic that names a variant to
  write instead.
- A host may describe enums to a file through the environment, as it does
  events and state; Sindri describes every enum in the project to every file.

### `match`

`match` takes a value of an enum and runs the one arm its value names. Each arm
lists one or more variants separated by `|`, then `=>` and a block.

Written where a value goes, it gives one instead: each arm is `=>` and an
expression, and arms are separated by commas.

```rust
let label = match phase {
    Phase.Lobby => "Waiting",
    Phase.Countdown | Phase.Play => "Go",
    Phase.Over => "Again?",
};
let speed = base * match size { Size.Small => 1.0, _ => 0.5 };
```

- **Exhaustive.** A `match` that says nothing for some variant is a
  diagnostic, unless it ends with a `_` arm, which takes everything not named
  above it.
- A variant matched twice, a variant of another enum, and an arm after `_` are
  diagnostics.
- A `match` on anything but an enum is a diagnostic; compare numbers with
  `if`.
- **Every arm of a value gives the same type**, which is the match's. An arm
  that gives a value is an expression, not a block: to run statements, use
  the statement form. At the start of a statement, `match` is always the
  statement form.

---

## Structs

A `struct` names a value made of typed fields, so values that belong together
travel together:

```rust
struct Offer { index: f32, weight: f32, name: String }

script Chooser {
    var offers: List<Offer> = [];
    var best = Offer(index: -1.0, weight: 0.0, name: "none");

    fn add(index: f32) {
        offers.push(Offer(index: index, weight: 1.0, name: "module " + index));
    }
    fn update(dt: f32) {
        for offer in offers {
            if offer.weight > best.weight { best = offer; }
        }
        best.weight += 1.0;
    }
}
```

- **Built with every field named**, in any order: `Offer(name: "a",
  index: 1.0, weight: 2.0)`. A field left out, named twice or not in the
  struct is a diagnostic, and so is building one with positional arguments —
  except a field with a **default**, which may be left out and then holds it:

  ```rust
  struct Offer { name: String, weight: f32 = BASE * 1.5, rarity: Rarity = Rarity.Common }
  let o = Offer(name: "a");                // weight 3, rarity Common, with BASE 2
  ```

  A default is worked out when the file compiles, as a constant is: it holds
  `f32`, `bool`, `String` or an enum, and may use literals, operators,
  variants and constants, but not another field. Naming the field still
  gives it whatever it is given.
- **A struct is a value**, like a vector or a list: assigning one copies it,
  and `==` compares every field. A field is written through the variable,
  parameter or field of this script that holds the struct — `best.weight =
  2.0`, `this.hand.best.name = "Nova"`, `slot.at.x += 1.0` — and that one
  must be a `var`. A field of a list's element cannot be written in place:
  take it out, change it, and put it back (`var o = offers[i]; o.weight = 2.0;
  offers[i] = o;`).
- A list inside a struct is changed in place through it: `hand.cards.push(c)`.
- Its fields may be any type, including another struct, a list of one, an
  enum or an entity. It takes its type's name — `var c: Offer` — and a field
  initialized with one has that type without an annotation.
- **Declared once.** A struct with no fields, a field declared twice and a
  struct that takes a name something else already has are diagnostics. A host
  may describe structs to a file through the environment, as it does enums;
  Sindri describes every struct in the project to every file.
- `print` shows one as `Offer(index: 1, weight: 2, name: "a")`. Text does not
  join with a struct. A timer inside a struct is not run down by the host.

### Methods

Functions written in a struct, after its fields, are asked of one value:

```rust
struct Offer {
    index: f32,
    weight: f32,
    name: String,

    fn heavier(other: Offer) -> bool { return this.weight > other.weight; }
    fn doubled() -> Offer {
        return Offer(index: this.index, weight: this.weight * 2.0, name: this.name);
    }
}

if offer.heavier(best) { best = offer.doubled(); }
```

- **`this` is the value** the method was asked of, and its fields are read as
  `this.weight`; a bare `weight` is not in scope. A method may call the
  struct's other methods on `this`, and the engine, as any function can.
- **`this` does not change.** The method works on a copy, so writing
  `this.weight` is a diagnostic: return the changed value instead, as
  `doubled` does, and assign it where it is wanted.
- **Checked like any call**: `offer.heavier(1.0)` is a type mismatch, a
  method the struct does not have is named with the ones it does, and
  `offer.heavier` without its arguments is a diagnostic — a method is always
  called, even one that takes nothing, `offer.label()`.
- A method may not share a name with a field, and a struct's fields come
  before its methods. There are no methods on anything but a struct, no
  `impl` blocks, and no function that belongs to a struct without a value.
- Like the struct, its methods are every file's: Sindri describes each one to
  every file and links its code into every program that calls it.

---

## Statements

```rust
let x: f32 = 1.0;      // binding, fixed
var y: f32 = 2.0;      // binding, reassignable
y = 3.0;               // expression statement
return y;              // return
return;                // return unit
if y > 0.0 { } else if y < 0.0 { } else { }
while y > 0.0 { y -= 1.0; }
match phase { Phase.Play => { } _ => { } }
{ }                    // bare block, its own scope
```

A binding needs a type, an initializer, or both.

`if` and `while` require a `bool` condition — there is no truthiness, so
`if x` where `x` is a number is a diagnostic.

### Loops

`while` and `for`. There is no `loop`. `for` walks a list or a range — see
[Lists](#lists).

```rust
var i: f32 = 0.0;
while i < 10.0 {
    i += 1.0;

    if i == 5.0 {
        continue;      // back to the condition
    }
    if i == 8.0 {
        break;         // out of the loop
    }
}
```

`break` leaves the innermost enclosing loop; `continue` returns to its
condition. Both are refused at compile time outside a loop, and both close the
blocks they leave, so a binding declared inside one does not outlive the turn
that declared it.

A loop is bounded by the **operation budget** rather than trusted — see
[Execution limits](#execution-limits).

---

## Expressions

### Precedence

Loosest to tightest:

| Level | Operators | Associativity |
| --- | --- | --- |
| 1 | `=` `+=` `-=` `*=` `/=` `%=` | right |
| 2 | `\|\|` | left |
| 3 | `&&` | left |
| 4 | `==` `!=` | left |
| 5 | `<` `<=` `>` `>=` | left |
| 6 | `+` `-` | left |
| 7 | `*` `/` `%` | left |
| 8 | `-` `!` (unary prefix) | right |
| 9 | `.` `()` (postfix) | left |

### Operand rules

- `+ - * / %` require `f32` on both sides and produce `f32`, or vectors as
  [Vectors](#vectors) says. **`+` with text on either side joins**; see
  [Text](#text).
- `%` is a **remainder**, not a floored modulo: its sign follows the left
  operand, so `-7.0 % 3.0` is `-1.0` rather than `2.0`.
- `< <= > >=` require `f32` and produce `bool`.
- `== !=` require the two sides to be compatible types, and produce `bool`.
- `&& ||` require `bool` on both sides and produce `bool`, and **short-circuit**:
  the right side is evaluated only when the left does not already decide the
  answer. So `held != null && World.exists(held)` guards the call to its right,
  and `ready || expensive()` does not ask when it is already ready.
- unary `-` requires `f32`; unary `!` requires `bool`.

### Assignment

`=` assigns. `+= -= *= /= %=` read, apply, and assign, and require `f32` on both
sides.

Assignment targets are a name or a member path. Assigning to a `let` binding, to
a function, or to anything else is a diagnostic.

Assignment is an expression and evaluates to the assigned value, so `a = b = 1.0`
works.

### Division by zero

`1.0 / 0.0` is infinity and `1.0 % 0.0` is NaN, neither of them an error. There
is no integer division to trap.

---

## Scope

Function parameters and the function body share one scope, so a parameter and a
top-level binding of the same name collide.

Every `if` branch and every bare block opens a nested scope. A binding inside one
leaves with it, and shadows rather than replaces an outer binding of the same
name:

```rust
var x: f32 = 1.0;
if flag {
    var x: f32 = 2.0;   // a different x
}
// x is still 1.0
```

Assigning to a name the block did not declare writes through to the outer one,
as expected.

Container fields are visible in every function of that container.

---

## The host boundary

Decay has **no built-in functions and no standard library**. Not even `print` —
where one exists, the host registered it. The one exception is vectors: `Vec2`,
`Vec3`, and what a vector can do belong to the language, because a value type
a host could not build would be one a script could not either.

Everything a script can name beyond its own container comes from the host, which
registers globals before compilation. The compiler knows their names and
signatures and nothing else, and the runtime forwards every access as a path.

An unresolved name is a compile error. An unresolved *path* is a compile error
too, when the host described the type it goes through, and a runtime error when
it did not.

A host describes a type by listing its members, and describes what `this` offers
beyond the script's own fields. A container's own field always wins over a host
member of the same name, so the engine growing a name can never shadow state a
script already had.

For what the Sindri engine specifically provides — the entity's transform and
sprite, the keyboard, the frame's time, `print`, and six maths functions — see
`docs/scripting.md`. That document is the whole list; this one never grows a
Sindri-specific name, because the language does not have one.

---

## Execution limits

Decay calls may nest 64 deep by default, after which the script fails with
`CallDepthExceeded`.

Text is at most 64 KiB; making longer fails with `TextTooLong`. A list a script
builds is at most 10,000 elements; growing one past that fails with
`ListTooLong`.

One call may execute 1,000,000 instructions by default, after which it fails
with `OperationBudgetExceeded`. The budget is per outermost call, so a script is
not charged for what the previous frame did, and a script cannot buy itself more
by recursing.

The host may change either limit.

Both exist for the same reason: a script that does not stop must not take the
editor with it. The depth limit bounds recursion, which was the only way to run
forever until `while` arrived; the budget bounds everything else, and is what
makes a loop safe to offer at all. Neither is a panic — both are values the host
reports for one entity while every other script keeps running.

---

## Surprising behaviour

Things that are true and that most readers — human or model — will guess wrong.

1. **`this.method()` is not a method call in a script.** A container has no
   methods; it is a compile error telling you to call it by bare name. Host
   *types* and structs have methods, and those work — inside a struct's own
   method, `this` is the struct's value and `this.other()` calls another.
2. **There is no truthiness.** `if x` requires `x` to be `bool`.
3. **`"d" + 3.0` is `"d3"`, and `"d" + 1.0 + 2.0` is `"d12"`.** `+` groups
   left to right, so once text is on the left every `+` after it joins. Write
   `"d" + (1.0 + 2.0)` for `"d3"`.
4. **All numbers are floats.** `7 / 2` is `3.5`. There is no integer type and no
   integer division, and `%` is a remainder whose sign follows its left operand.
5. **Member types are checked only where the host described them.**
   `this.transfrom.position.x` is now a compile error against Sindri, because
   Sindri describes its transform — but a path into a type nobody described is
   still accepted and still fails at runtime.
6. **`let` fields are still settable by the host.** That is what `@export` means.
7. **A parameter shadows nothing** — parameters and body bindings share one
   scope, so reusing a parameter's name is a duplicate, not a shadow.
8. **A loop can be stopped by its budget.** A script that runs too long fails
   with `OperationBudgetExceeded` rather than hanging, and that failure is
   reported like any other.
9. **A range exists only in a `for`.** `for i in 0..3` walks `0, 1, 2`;
   `let r = 0..3;` is a parse error.
10. **A collection index is a float like every other number**, and a fractional
    one is refused rather than rounded.
11. **A vector is copied, not shared.** Assigning one and changing the copy
    leaves the original as it was.
12. **Two vectors do not multiply.** `a * b` is an error; write `a.dot(b)`.
13. **`v.length` is a property.** `v.length()` is an error, while `a.dot(b)`
    is a call.

## What does not exist

Do not write these. They are not unimplemented corners; they are absent from the
grammar, and every one of them is a parse error or a diagnostic.

**Control flow:** `loop`, `match` on anything but an enum, a block as a `match` arm's value, ternaries, labelled blocks, `break` or
`continue` with a label or a value. `for … in` walks a list or a range, and
nothing else.

**Data:** maps, dictionaries, sets, tuples, methods that change their struct,
defaults for fields of a type a constant cannot hold (a vector, a list, an
entity), enums with data
(`Some(x)`), a range as a value outside `for`, a step (`0..10 by 2`), negative
indices, slicing a list, `Option`, `Result`, `?`. `List<T>` exists; see
"Lists".

**Functions:** closures, lambdas, function values, default arguments, named
arguments, variadics, generics beyond `List<T>`, overloading, methods on
anything but a struct, `impl`, traits.

**Types:** integers, `f64`, unsigned types, characters, type aliases, casts,
inference beyond a binding's own initializer, nullable types, user-defined types
beyond `script`, `component`, `enum` and `struct`.

**Modules:** `import`, `use`, `mod`, `pub`, visibility of any kind, multiple
files. One file is one compilation unit and cannot refer to another by itself;
a host may describe other files' scripts, events, state and shared functions
to it, as Sindri does. There is no `import` line: in a Sindri project every
file already sees the others.

**Standard library:** `print`, `math.*`, format strings (`{:.2}`) — a number
is written with `fixed` and `padded` —, interpolation, parsing text into
numbers, conversion functions, list operations beyond the ones listed — no
`map`, `filter`, `sort` — time, randomness.

**Other:** operator overloading, macros, attributes other than `@export`, block
comments, doc comments, a `const` inside a container or a function, `static`,
exceptions, `try`, `panic`,
concurrency, `async`.

---

## Diagnostics

Compilation runs in two phases and reports both together, each diagnostic
carrying a byte span plus a 1-based line and column.

**Syntax** diagnostics come from the lexer and parser. The parser recovers and
continues, so one missing semicolon does not hide the rest of the file.

**Semantic** diagnostics come from name resolution and type checking: unknown
names, type mismatches, assignment to an immutable binding, duplicate members or
locals, a non-`bool` `if` condition, and wrong argument counts.

**A program with any diagnostic does not lower.** There is no partial
compilation and no warning level — everything reported is fatal.

Runtime failures are values, not panics: `UnknownPath`, `Immutable`,
`FunctionNotFound`, `Arity`, `InvalidBinary`, `NotACollection`,
`IndexNotANumber`, `IndexNotWhole`, `IndexOutOfRange`, `CallDepthExceeded`,
`OperationBudgetExceeded`, and others.
The host decides what to do with them; `sindri-decay` reports them per entity and
keeps running every other script.
