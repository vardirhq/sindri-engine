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
- [Vectors](#vectors)
- [Containers](#containers)
- [Fields](#fields)
- [Functions](#functions)
- [Events](#events)
- [State](#state)
- [Shared functions](#shared-functions)
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
continue  return  true  false  null
```

All sixteen are reserved. Three words are special only where no other name
could stand: `event` and `state` at the start of an item, and `on` followed by
a name at the start of a member (see [Events](#events) and [State](#state)). Elsewhere both are ordinary
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
->  @   .   ,   ;   :
(   )   {   }   [   ]
```

`[` and `]` are recognised by the lexer but appear nowhere in the grammar; there
is no indexing and there are no array literals.

---

## Grammar

```ebnf
program      = { item } ;
item         = ( "script" | "component" ) IDENT "{" { member } "}"
             | "event" IDENT "(" [ params ] ")" ";"
             | "state" IDENT "{" { field } "}"
             | function ;
member       = { attribute } ( field | function | handler ) ;
attribute    = "@" IDENT ;

field        = ( "let" | "var" ) IDENT [ ":" type ] [ "=" expr ] ";" ;
function     = "fn" IDENT "(" [ params ] ")" [ "->" type ] block ;
handler      = "on" IDENT "(" [ params ] ")" block ;
params       = param { "," param } ;
param        = IDENT [ ":" type ] ;
type         = IDENT [ "<" type ">" ] ;

block        = "{" { stmt } "}" ;
stmt         = binding | return | if | while | for | break | continue
             | block | expr ";" ;
binding      = ( "let" | "var" ) IDENT [ ":" type ] [ "=" expr ] ";" ;
return       = "return" [ expr ] ";" ;
if           = "if" expr block [ "else" ( block | if ) ] ;
while        = "while" expr block ;
for          = "for" IDENT "in" expr block ;
break        = "break" ";" ;
continue     = "continue" ";" ;

expr         = assign ;
assign       = binary [ ( "=" | "+=" | "-=" | "*=" | "/=" | "%=" ) assign ] ;
binary       = unary { binop unary } ;         (* see the precedence table *)
unary        = [ "-" | "!" ] unary | postfix ;
postfix      = primary { "." IDENT | "[" expr "]" | "(" [ args ] ")" } ;
args         = expr { "," expr } ;
primary      = IDENT | NUMBER | STRING | "true" | "false" | "null"
             | "(" expr ")" ;
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
| `String` or `string` | Text |
| `unit` or `void` | No value; the default return type |
| `Vec2`, `Vec3` | Two or three numbers that travel together; see [Vectors](#vectors) |
| `Array<T>` | Several `T`, in a fixed order |
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

### Collections

`Array<T>` is the one type that holds more than one value, and the one type
that takes a type argument. It is not user-definable and there is no literal
for one: **a collection only ever comes from the host.**

```rust
let enemies: Array<Entity> = World.with_tag(this.enemy);
let count: f32 = enemies.len;
let first: Entity = enemies[0.0];
for enemy in enemies {
    enemy.transform.position.y -= 1.0;
}
```

That is the whole of it. There is no way to build, grow, shrink, sort, or write
into one, and a `for` binding is immutable because an element is what the
collection holds at that position rather than a place to put something. The
absence is deliberate: a collection nothing can grow is a collection whose size
the host decided, which is what lets a host bound one.

`.len` is a property of the value, not a global `len(x)`. Decay has no modules,
so every global name added is one a script can no longer use for its own — and
`len` stays available as an ordinary name because of it.

**Indices are the one numeric type.** There is no integer type, so a whole
number is a property of the *value*, checked where the value is: `items[1.5]`,
`items[-1.0]`, and `items[7.0]` on a collection of three are three different
runtime errors, each naming what was wrong. Introducing an integer type for
indexing alone would rewrite every signature in the language and at the host
boundary; `docs/decay-direction.md` records that trade.

A `for` walks the collection it was given, not the name it came from:
reassigning that name inside the loop does not change what is being walked.

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

A function written outside any container is shared: every script may call
it by name.

```rust
fn half(size: f32) -> f32 { return size * 0.5; }

script Enemy {
    let view_size: f32 = 11.0;
    fn update(dt: f32) { let edge = half(this.view_size); }
}
```

- **No `this`.** A shared function belongs to no script, so it has no fields
  and no `this`; what it needs, it is passed. Using `this` in one is a
  diagnostic that says so.
- **Called by bare name**, checked like any call. A script's own function of
  the same name wins inside that script, as a local wins over a field.
- **One name, once.** Declaring it twice is a diagnostic, and so is a name a
  container, an event, a state or the host already has.

**Across files is the host's decision.** The language lowers a file's shared
functions into its program, and `IrProgram::link` adds another program's; a
bare call tries the script's own functions, then the shared ones, then the
host. A host that describes other files' shared functions to the analyzer
(`Environment::add_shared_function`) and links them makes them callable from
every file, which is what Sindri does for a project.

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

- **Fields only.** Each holds an `f32` or a `bool`, and its initializer is a
  literal — a number, a negated number, `true` or `false` — since it is set up
  before any script runs. A written type must match the literal.
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

## Statements

```rust
let x: f32 = 1.0;      // binding, fixed
var y: f32 = 2.0;      // binding, reassignable
y = 3.0;               // expression statement
return y;              // return
return;                // return unit
if y > 0.0 { } else if y < 0.0 { } else { }
while y > 0.0 { y -= 1.0; }
{ }                    // bare block, its own scope
```

A binding needs a type, an initializer, or both.

`if` and `while` require a `bool` condition — there is no truthiness, so
`if x` where `x` is a number is a diagnostic.

### Loops

`while` and `for`. There is no `loop`, and `for` walks a collection rather than
a range — see [Collections](#collections) — because a collection is the only
thing there is to iterate.

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

- `+ - * / %` require `f32` on both sides and produce `f32`. **`+` does not
  concatenate strings.**
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

1. **`this.method()` is not a method call.** A container has no methods; it is a
   compile error telling you to call it by bare name. Host *types* may have
   methods, and those work.
2. **There is no truthiness.** `if x` requires `x` to be `bool`.
3. **`+` does not join strings.** It is numeric addition only.
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
9. **`for` only walks a collection.** There are no ranges, so `for i in 0..3`
   is a parse error rather than a loop.
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

**Control flow:** `loop`, `match`, ternaries, labelled blocks, `break` or
`continue` with a label or a value. `for … in` exists and walks a collection;
there is nothing else to walk and no range to walk over.

**Data:** array literals, lists, maps, dictionaries, tuples, structs, enums,
ranges (`0..3`), `Option`, `Result`, `?`. `Array<T>` exists and is indexable,
but only the host makes one; see "Collections".

**Functions:** closures, lambdas, function values, default arguments, named
arguments, variadics, generics beyond `Array<T>`, overloading, methods, `impl`,
traits.

**Types:** integers, `f64`, unsigned types, characters, type aliases, casts,
inference beyond a binding's own initializer, nullable types, user-defined types
beyond `script` and `component`.

**Modules:** `import`, `use`, `mod`, `pub`, visibility of any kind, multiple
files. One file is one compilation unit and cannot refer to another by itself;
a host may describe other files' scripts, events, state and shared functions
to it, as Sindri does. There is no `import` line: in a Sindri project every
file already sees the others.

**Standard library:** `print`, `math.*`, string methods, formatting,
interpolation, conversion functions, collection *operations* — no `push`,
`map`, `filter`, `sort` — time, randomness.

**Other:** operator overloading, macros, attributes other than `@export`, block
comments, doc comments, `const`, `static`, exceptions, `try`, `panic`,
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
