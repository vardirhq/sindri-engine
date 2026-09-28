//! The words and symbols Decay is written in, as tables the lexer and parser
//! read, so a tool listing them — a documentation generator, an editor — reads
//! the same tables and cannot drift from what the compiler accepts.

use crate::TokenKind;
use crate::ast::{AssignOp, BinaryOp, UnaryOp};

/// This language's version: the syntax crate's, which every Decay crate
/// shares.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Every reserved word, and the token it lexes to.
pub const KEYWORDS: [(&str, TokenKind); 17] = [
    ("script", TokenKind::Script),
    ("component", TokenKind::Component),
    ("fn", TokenKind::Fn),
    ("let", TokenKind::Let),
    ("var", TokenKind::Var),
    ("if", TokenKind::If),
    ("else", TokenKind::Else),
    ("while", TokenKind::While),
    ("for", TokenKind::For),
    ("in", TokenKind::In),
    ("break", TokenKind::Break),
    ("continue", TokenKind::Continue),
    ("return", TokenKind::Return),
    ("match", TokenKind::Match),
    ("true", TokenKind::True),
    ("false", TokenKind::False),
    ("null", TokenKind::Null),
];

/// A word special only in one position, and an ordinary name everywhere else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextualWord {
    pub word: &'static str,
    /// Where it is special, in words: "the start of an item".
    pub position: &'static str,
}

pub const EVENT: &str = "event";
pub const STATE: &str = "state";
pub const ENUM: &str = "enum";
pub const STRUCT: &str = "struct";
pub const SHARED: &str = "shared";
pub const ON: &str = "on";

/// Every contextual word, where the parser treats it as one.
pub const CONTEXTUAL_KEYWORDS: [ContextualWord; 6] = [
    ContextualWord {
        word: EVENT,
        position: "the start of an item",
    },
    ContextualWord {
        word: STATE,
        position: "the start of an item",
    },
    ContextualWord {
        word: ENUM,
        position: "the start of an item",
    },
    ContextualWord {
        word: STRUCT,
        position: "the start of an item",
    },
    ContextualWord {
        word: SHARED,
        position: "right before `fn` at the start of an item",
    },
    ContextualWord {
        word: ON,
        position: "the start of a member, followed by an event's name",
    },
];

/// The words that may start an item, in the order a diagnostic lists them.
pub const ITEM_WORDS: [&str; 7] = ["script", "component", ENUM, STRUCT, EVENT, STATE, "fn"];

/// The attribute that marks a field a scene may author.
pub const EXPORT: &str = "export";

/// Every attribute there is.
pub const ATTRIBUTES: [&str; 1] = [EXPORT];

/// Every binary operator: its token, its precedence (higher binds tighter;
/// every one groups left to right) and what it does.
pub const BINARY_OPERATORS: [(TokenKind, u8, BinaryOp); 13] = [
    (TokenKind::OrOr, 1, BinaryOp::Or),
    (TokenKind::AndAnd, 2, BinaryOp::And),
    (TokenKind::EqualEqual, 3, BinaryOp::Equal),
    (TokenKind::BangEqual, 3, BinaryOp::NotEqual),
    (TokenKind::Less, 4, BinaryOp::Less),
    (TokenKind::LessEqual, 4, BinaryOp::LessEqual),
    (TokenKind::Greater, 4, BinaryOp::Greater),
    (TokenKind::GreaterEqual, 4, BinaryOp::GreaterEqual),
    (TokenKind::Plus, 5, BinaryOp::Add),
    (TokenKind::Minus, 5, BinaryOp::Subtract),
    (TokenKind::Star, 6, BinaryOp::Multiply),
    (TokenKind::Slash, 6, BinaryOp::Divide),
    (TokenKind::Percent, 6, BinaryOp::Modulo),
];

/// Every assignment, plain and compound. Looser than any binary operator,
/// and grouping right to left.
pub const ASSIGNMENT_OPERATORS: [(TokenKind, AssignOp); 6] = [
    (TokenKind::Equal, AssignOp::Assign),
    (TokenKind::PlusEqual, AssignOp::Add),
    (TokenKind::MinusEqual, AssignOp::Subtract),
    (TokenKind::StarEqual, AssignOp::Multiply),
    (TokenKind::SlashEqual, AssignOp::Divide),
    (TokenKind::PercentEqual, AssignOp::Modulo),
];

/// Every prefix operator. Tighter than any binary operator, and grouping
/// right to left.
pub const UNARY_OPERATORS: [(TokenKind, UnaryOp); 2] = [
    (TokenKind::Minus, UnaryOp::Negate),
    (TokenKind::Bang, UnaryOp::Not),
];

impl BinaryOp {
    /// How it is written.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Modulo => "%",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::And => "&&",
            Self::Or => "||",
        }
    }
}

impl AssignOp {
    /// How it is written.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::Add => "+=",
            Self::Subtract => "-=",
            Self::Multiply => "*=",
            Self::Divide => "/=",
            Self::Modulo => "%=",
        }
    }
}

impl UnaryOp {
    /// How it is written.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Negate => "-",
            Self::Not => "!",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ASSIGNMENT_OPERATORS, BINARY_OPERATORS, KEYWORDS, UNARY_OPERATORS};
    use crate::{ExprKind, Item, Member, Stmt, lex, parse};

    #[test]
    fn every_keyword_lexes_as_itself() {
        for (word, kind) in KEYWORDS {
            let lexed = lex(word);
            assert_eq!(lexed.tokens[0].kind, kind, "{word}");
        }
    }

    /// Each symbol, written between two operands, parses as the operator the
    /// table says it is.
    #[test]
    fn every_operator_symbol_parses_as_its_operator() {
        let expression = |text: &str| {
            let source = format!("script T {{ fn f() {{ {text}; }} }}");
            let parsed = parse(&source);
            assert!(
                parsed.diagnostics.is_empty(),
                "{text}: {:?}",
                parsed.diagnostics
            );
            let Item::Script(container) = &parsed.program.items[0] else {
                panic!("a script");
            };
            let Member::Function(function) = &container.members[0] else {
                panic!("a function");
            };
            let Stmt::Expr { expr, .. } = &function.body.statements[0] else {
                panic!("an expression");
            };
            expr.kind.clone()
        };
        for (_, _, op) in BINARY_OPERATORS {
            let kind = expression(&format!("a {} b", op.symbol()));
            assert!(
                matches!(kind, ExprKind::Binary { op: parsed, .. } if parsed == op),
                "{}",
                op.symbol()
            );
        }
        for (_, op) in ASSIGNMENT_OPERATORS {
            let kind = expression(&format!("a {} b", op.symbol()));
            assert!(
                matches!(kind, ExprKind::Assign { op: parsed, .. } if parsed == op),
                "{}",
                op.symbol()
            );
        }
        for (_, op) in UNARY_OPERATORS {
            let kind = expression(&format!("{}a", op.symbol()));
            assert!(
                matches!(kind, ExprKind::Unary { op: parsed, .. } if parsed == op),
                "{}",
                op.symbol()
            );
        }
    }
}
