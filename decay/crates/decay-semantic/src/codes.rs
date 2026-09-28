//! Every semantic diagnostic by a stable name.
//!
//! A message may be reworded; its code may not. A code is what a tool keys
//! on — a documentation page, a test expecting a failure, an editor's quick
//! fix — so it outlives any wording and is never reused for something else.
//! Syntax diagnostics have their own, in [`decay_syntax::codes`].

/// What is wrong with a program that parses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Code {
    UnknownName,
    UnknownMember,
    NotCallable,
    NoMethods,
    PropertyCalled,
    FunctionNotCalled,
    ArgumentCount,
    TypeMismatch,
    CannotCompare,
    ConditionNotBool,
    NotIndexable,
    NotWalkable,
    InvalidOperand,
    Immutable,
    NotAPlace,
    Duplicate,
    DeclaredInSeveralFiles,
    NameTaken,
    MissingType,
    UnexpectedTypeArgument,
    FieldOrder,
    OutsideLoop,
    NoThis,
    EmptyDeclaration,
    NotExhaustive,
    UnreachableArm,
    MatchNeedsEnum,
    WrongEnum,
    UnknownEvent,
    HandlerSignature,
    StateFieldType,
    StateStartNotLiteral,
    MixedList,
    RangeOutsideFor,
    MissingFields,
    NotAStruct,
}

impl Code {
    /// Every code, its stable name, and what it means.
    pub const ALL: [(Self, &'static str, &'static str); 36] = [
        (
            Self::UnknownName,
            "unknown-name",
            "A name nothing in scope declares: not a local, a field, a script, an event, a state, an enum, a struct or anything the engine offers.",
        ),
        (
            Self::UnknownMember,
            "unknown-member",
            "A member, field or variant the value's type does not have.",
        ),
        (
            Self::NotCallable,
            "not-callable",
            "A call to something that is not a function: a value, an event, an enum, or a struct built without naming its fields.",
        ),
        (
            Self::NoMethods,
            "no-methods",
            "A call through `this` to the script's own function; call it by its bare name.",
        ),
        (
            Self::PropertyCalled,
            "property-called",
            "A property, such as `length` or a vector's `x`, written as a call.",
        ),
        (
            Self::FunctionNotCalled,
            "function-not-called",
            "An operation that takes arguments, used without calling it.",
        ),
        (
            Self::ArgumentCount,
            "argument-count",
            "A call, constructor or handler given the wrong number of arguments.",
        ),
        (
            Self::TypeMismatch,
            "type-mismatch",
            "A value of one type where another is needed.",
        ),
        (
            Self::CannotCompare,
            "cannot-compare",
            "`==` or `!=` between values of types that cannot be equal.",
        ),
        (
            Self::ConditionNotBool,
            "condition-not-bool",
            "An `if` or `while` condition that is not a `bool`; there is no truthiness.",
        ),
        (
            Self::NotIndexable,
            "not-indexable",
            "`[index]` on something that is not a list.",
        ),
        (
            Self::NotWalkable,
            "not-walkable",
            "`for` over something that is not a list or a range.",
        ),
        (
            Self::InvalidOperand,
            "invalid-operand",
            "An operator given a value it does not work on, such as `-` on text or `*` between two vectors.",
        ),
        (
            Self::Immutable,
            "immutable",
            "A change to something that may not change: a `let`, a state's `let`, a timer's property.",
        ),
        (
            Self::NotAPlace,
            "not-a-place",
            "An assignment or change to something that is not a variable, parameter or field of this script.",
        ),
        (
            Self::Duplicate,
            "duplicate",
            "A name declared, given or matched twice where it may appear once.",
        ),
        (
            Self::DeclaredInSeveralFiles,
            "declared-in-several-files",
            "A script, event, state field, enum, struct or shared function declared in more than one file of the project.",
        ),
        (
            Self::NameTaken,
            "name-taken",
            "A declaration taking a name something else already has.",
        ),
        (
            Self::MissingType,
            "missing-type",
            "A binding, field or parameter with no type and nothing to infer one from, or `List` without its element type.",
        ),
        (
            Self::UnexpectedTypeArgument,
            "unexpected-type-argument",
            "A type argument, `<T>`, on a type that takes none.",
        ),
        (
            Self::FieldOrder,
            "field-order",
            "A field's initializer reading itself, or a field declared below it.",
        ),
        (
            Self::OutsideLoop,
            "outside-loop",
            "`break` or `continue` outside a loop.",
        ),
        (
            Self::NoThis,
            "no-this",
            "`this` in a function outside any script.",
        ),
        (
            Self::EmptyDeclaration,
            "empty-declaration",
            "An enum with no variants or a struct with no fields.",
        ),
        (
            Self::NotExhaustive,
            "not-exhaustive",
            "A `match` that says nothing for some variant and has no `_` arm.",
        ),
        (
            Self::UnreachableArm,
            "unreachable-arm",
            "A `match` arm after `_`, which already takes everything.",
        ),
        (
            Self::MatchNeedsEnum,
            "match-needs-enum",
            "A `match` on a value that is not an enum's.",
        ),
        (
            Self::WrongEnum,
            "wrong-enum",
            "A `match` pattern naming another enum's variant.",
        ),
        (
            Self::UnknownEvent,
            "unknown-event",
            "A handler for an event nothing declares.",
        ),
        (
            Self::HandlerSignature,
            "handler-signature",
            "A handler whose parameters do not fit what its event carries.",
        ),
        (
            Self::StateFieldType,
            "state-field-type",
            "A state field of a type a state cannot hold.",
        ),
        (
            Self::StateStartNotLiteral,
            "state-start-not-literal",
            "A state field whose starting value is not a literal.",
        ),
        (
            Self::MixedList,
            "mixed-list",
            "A list literal whose elements are not all one type.",
        ),
        (
            Self::RangeOutsideFor,
            "range-outside-for",
            "A range, `a..b`, anywhere but what a `for` walks.",
        ),
        (
            Self::MissingFields,
            "missing-fields",
            "A struct built without naming every field.",
        ),
        (
            Self::NotAStruct,
            "not-a-struct",
            "Named fields, `Name(field: value)`, after a name that is not a struct.",
        ),
    ];

    /// Its stable name.
    #[must_use]
    pub fn id(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(code, _, _)| *code == self)
            .map_or("semantic", |(_, id, _)| id)
    }
}

#[cfg(test)]
mod tests {
    use super::Code;

    #[test]
    fn every_code_has_one_distinct_kebab_case_name() {
        let mut seen = std::collections::HashSet::new();
        for (code, id, summary) in Code::ALL {
            assert_eq!(code.id(), id);
            assert!(seen.insert(id), "`{id}` is used twice");
            assert!(
                id.split('-')
                    .all(|word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_lowercase())),
                "`{id}` is not kebab-case"
            );
            assert!(summary.ends_with('.'), "`{id}` needs a summary");
        }
        let codes: std::collections::HashSet<_> =
            Code::ALL.iter().map(|(code, _, _)| code).collect();
        assert_eq!(codes.len(), Code::ALL.len(), "a code is listed twice");
    }
}
