//! Turning tokens into a program.
//!
//! A recursive-descent parser that keeps going after an error: it
//! synchronizes to the next member or statement and carries on, so one
//! mistake in a script does not hide the rest of them.

mod expr;
mod item;
mod stmt;

#[cfg(test)]
mod tests;

use crate::{
    Diagnostic, Span, Token, TokenKind,
    ast::{Item, Program},
    lex, line_column,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub program: Program,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn parse(source: &str) -> Parsed {
    let lexed = lex(source);
    let mut parser = Parser::new(source, lexed.tokens, lexed.diagnostics);
    let program = parser.parse_program();
    Parsed {
        program,
        diagnostics: parser.diagnostics,
    }
}

pub(super) struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    cursor: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    pub(super) fn new(source: &'a str, tokens: Vec<Token>, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            source,
            tokens,
            cursor: 0,
            diagnostics,
        }
    }

    pub(super) fn parse_program(&mut self) -> Program {
        let mut items = Vec::new();
        while !self.at(&TokenKind::Eof) {
            let item = if self.at(&TokenKind::Script) {
                self.parse_container(true)
            } else if self.at(&TokenKind::Component) {
                self.parse_container(false)
            } else if self.at_word(item::EVENT) {
                self.parse_event()
            } else if self.at_word(item::STATE) {
                self.parse_state()
            } else if self.at_word(item::ENUM) {
                self.parse_enum()
            } else if self.at_word(item::STRUCT) {
                self.parse_struct()
            } else if self.at(&TokenKind::Fn) {
                self.parse_function().map(Item::Function)
            } else if self.at_shared_fn() {
                self.advance();
                self.parse_function().map(|function| {
                    Item::Function(crate::ast::FunctionDecl {
                        shared: true,
                        ..function
                    })
                })
            } else {
                self.error_here(
                    "expected `script`, `component`, `enum`, `struct`, `event`, `state`, or `fn`",
                );
                self.advance();
                None
            };
            if let Some(item) = item {
                items.push(item);
            }
        }
        Program { items }
    }

    /// `shared fn`: `shared` is only a word with meaning right before `fn`.
    fn at_shared_fn(&self) -> bool {
        matches!(&self.current().kind, TokenKind::Identifier(name) if name == item::SHARED)
            && matches!(
                self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                Some(TokenKind::Fn)
            )
    }

    pub(super) fn synchronize_member(&mut self) {
        while !self.at(&TokenKind::Eof) && !self.at(&TokenKind::RightBrace) {
            if self.at(&TokenKind::Fn)
                || self.at(&TokenKind::Let)
                || self.at(&TokenKind::Var)
                || self.at(&TokenKind::At)
                || self.at_word(item::ON)
            {
                break;
            }
            self.advance();
        }
    }

    pub(super) fn synchronize_statement(&mut self) {
        while !self.at(&TokenKind::Eof) && !self.at(&TokenKind::RightBrace) {
            if self.consume_simple(&TokenKind::Semicolon).is_some() {
                break;
            }
            self.advance();
        }
    }

    pub(super) fn expect_identifier(&mut self, message: &str) -> Option<(String, Span)> {
        let token = self.current().clone();
        if let TokenKind::Identifier(name) = token.kind {
            self.advance();
            Some((name, token.span))
        } else {
            self.error_here(message);
            None
        }
    }

    pub(super) fn expect_simple(&mut self, expected: &TokenKind, message: &str) -> Option<Span> {
        if self.at(expected) {
            let span = self.current().span;
            self.advance();
            Some(span)
        } else {
            self.error_here(message);
            None
        }
    }

    pub(super) fn consume_simple(&mut self, expected: &TokenKind) -> Option<Span> {
        if self.at(expected) {
            let span = self.current().span;
            self.advance();
            Some(span)
        } else {
            None
        }
    }

    pub(super) fn at(&self, expected: &TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(expected)
    }

    pub(super) fn current(&self) -> &Token {
        &self.tokens[self.cursor.min(self.tokens.len().saturating_sub(1))]
    }

    pub(super) fn advance(&mut self) {
        if self.cursor + 1 < self.tokens.len() {
            self.cursor += 1;
        }
    }

    pub(super) fn error_here(&mut self, message: &str) {
        self.error_span(self.current().span, message);
    }

    pub(super) fn error_span(&mut self, span: Span, message: &str) {
        let (line, column) = line_column(self.source, span.start);
        self.diagnostics.push(Diagnostic {
            message: message.to_owned(),
            span,
            line,
            column,
        });
    }
}
