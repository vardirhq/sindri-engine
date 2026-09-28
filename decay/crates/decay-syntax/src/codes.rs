//! Every syntax diagnostic by a stable name.
//!
//! A message may be reworded; its code may not. A code is what a tool keys
//! on — a documentation page, a test expecting a failure, an editor's quick
//! fix — so it outlives any wording and is never reused for something else.

/// What went wrong reading the source, before any meaning is given to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxCode {
    UnexpectedCharacter,
    UnterminatedString,
    UnknownEscape,
    InvalidNumber,
    ExpectedToken,
    ExpectedExpression,
    ExpectedItem,
    ExpectedMember,
    AttributeNotAllowed,
    VariantNeedsEnumName,
    StateFieldsOnly,
    HandlerReturnsValue,
    ConstructNeedsStructName,
    FieldAfterMethod,
}

impl SyntaxCode {
    /// Every code, its stable name, and what it means.
    pub const ALL: [(Self, &'static str, &'static str); 14] = [
        (
            Self::UnexpectedCharacter,
            "unexpected-character",
            "A character that starts no token Decay has.",
        ),
        (
            Self::UnterminatedString,
            "unterminated-string",
            "Text opened with `\"` and never closed on its line.",
        ),
        (
            Self::UnknownEscape,
            "unknown-escape",
            "A `\\` in text followed by something other than n, r, t, \" or \\.",
        ),
        (
            Self::InvalidNumber,
            "invalid-number",
            "Digits that do not make a number.",
        ),
        (
            Self::ExpectedToken,
            "expected-token",
            "A particular word or symbol was needed here, such as `;`, `{`, `)` or a name.",
        ),
        (
            Self::ExpectedExpression,
            "expected-expression",
            "A value was needed here and none was written.",
        ),
        (
            Self::ExpectedItem,
            "expected-item",
            "Something at the top of a file that is not a script, component, enum, struct, event, state or function.",
        ),
        (
            Self::ExpectedMember,
            "expected-member",
            "Something inside a script that is not a field, a function or an `on` handler.",
        ),
        (
            Self::AttributeNotAllowed,
            "attribute-not-allowed",
            "An attribute such as `@export` somewhere only a script's field may carry one.",
        ),
        (
            Self::VariantNeedsEnumName,
            "variant-needs-enum-name",
            "A `match` pattern that names a variant without its enum: write `Phase.Lobby`.",
        ),
        (
            Self::StateFieldsOnly,
            "state-fields-only",
            "Something inside a `state` that is not a field.",
        ),
        (
            Self::HandlerReturnsValue,
            "handler-returns-value",
            "An `on` handler written with a return type; nothing waits for one.",
        ),
        (
            Self::ConstructNeedsStructName,
            "construct-needs-struct-name",
            "Named fields, `(name: value)`, after something that is not a struct's name.",
        ),
        (
            Self::FieldAfterMethod,
            "field-after-method",
            "A struct's field written after one of its methods; fields come first.",
        ),
    ];

    /// Its stable name.
    #[must_use]
    pub fn id(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(code, _, _)| *code == self)
            .map_or("syntax", |(_, id, _)| id)
    }
}

#[cfg(test)]
mod tests {
    use super::SyntaxCode;

    #[test]
    fn every_code_has_one_distinct_kebab_case_name() {
        let mut seen = std::collections::HashSet::new();
        for (code, id, summary) in SyntaxCode::ALL {
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
            SyntaxCode::ALL.iter().map(|(code, _, _)| code).collect();
        assert_eq!(codes.len(), SyntaxCode::ALL.len(), "a code is listed twice");
    }
}
