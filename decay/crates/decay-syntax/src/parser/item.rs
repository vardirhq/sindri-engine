//! Containers, and the members they declare.
//!
//! A new kind of member is a branch in `parse_container` and a function
//! beside the others here.

use crate::{
    TokenKind,
    ast::{
        Attribute, ContainerDecl, EventDecl, FieldDecl, FunctionDecl, Item, Member, Param,
        StateDecl, TypeRef, handler_name,
    },
};

pub(super) use crate::vocabulary::CONST;
pub(super) use crate::vocabulary::ENUM;
/// The words that start an event declaration, a state declaration, a shared
/// function, and a handler. Contextual rather than keywords: each is only special where no
/// other identifier could stand, so a script that already calls something
/// `event` or `state`, or reaches `Bolt.on(hit)`, keeps working.
pub(super) use crate::vocabulary::EVENT;
pub(super) use crate::vocabulary::ON;
pub(super) use crate::vocabulary::SHARED;
pub(super) use crate::vocabulary::STATE;
pub(super) use crate::vocabulary::STRUCT;

use super::Parser;

impl Parser<'_> {
    pub(super) fn parse_container(&mut self, script: bool) -> Option<Item> {
        let start = self.current().span;
        self.advance();
        let (name, _) = self.expect_identifier("expected a name after declaration keyword")?;
        self.expect_simple(&TokenKind::LeftBrace, "expected `{` after declaration name")?;

        let mut members = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            let attributes = self.parse_attributes();
            if self.at(&TokenKind::Fn) {
                if !attributes.is_empty() {
                    self.error_span(
                        crate::codes::SyntaxCode::AttributeNotAllowed,
                        attributes[0].span,
                        "attributes on functions are not supported yet",
                    );
                }
                if let Some(function) = self.parse_function() {
                    members.push(Member::Function(function));
                } else {
                    self.synchronize_member();
                }
            } else if self.at_word(ON) {
                if !attributes.is_empty() {
                    self.error_span(
                        crate::codes::SyntaxCode::AttributeNotAllowed,
                        attributes[0].span,
                        "attributes on handlers are not supported yet",
                    );
                }
                if let Some(handler) = self.parse_handler() {
                    members.push(Member::Function(handler));
                } else {
                    self.synchronize_member();
                }
            } else if self.at(&TokenKind::Let) || self.at(&TokenKind::Var) {
                if let Some(field) = self.parse_field(attributes) {
                    members.push(Member::Field(field));
                } else {
                    self.synchronize_member();
                }
            } else {
                self.error_here(
                    crate::codes::SyntaxCode::ExpectedMember,
                    "expected a field, function, or `on` handler in declaration body",
                );
                self.synchronize_member();
            }
        }

        let end = self
            .expect_simple(
                &TokenKind::RightBrace,
                "expected `}` after declaration body",
            )
            .unwrap_or_else(|| self.current().span);
        let decl = ContainerDecl {
            name,
            members,
            span: start.join(end),
        };
        Some(if script {
            Item::Script(decl)
        } else {
            Item::Component(decl)
        })
    }

    pub(super) fn parse_attributes(&mut self) -> Vec<Attribute> {
        let mut attributes = Vec::new();
        while self.at(&TokenKind::At) {
            let start = self.current().span;
            self.advance();
            if let Some((name, name_span)) =
                self.expect_identifier("expected attribute name after `@`")
            {
                attributes.push(Attribute {
                    name,
                    span: start.join(name_span),
                });
            } else {
                break;
            }
        }
        attributes
    }

    pub(super) fn parse_field(&mut self, attributes: Vec<Attribute>) -> Option<FieldDecl> {
        let start = self.current().span;
        let mutable = self.at(&TokenKind::Var);
        self.advance();
        let (name, _) = self.expect_identifier("expected field name")?;
        let ty = self.parse_optional_type();
        let initializer = if self.consume_simple(&TokenKind::Equal).is_some() {
            Some(self.parse_expression()?)
        } else {
            None
        };
        let end = self.expect_simple(&TokenKind::Semicolon, "expected `;` after field")?;
        Some(FieldDecl {
            attributes,
            mutable,
            name,
            ty,
            initializer,
            span: start.join(end),
        })
    }

    pub(super) fn parse_function(&mut self) -> Option<FunctionDecl> {
        let start = self.expect_simple(&TokenKind::Fn, "expected `fn`")?;
        let (name, _) = self.expect_identifier("expected function name")?;
        self.expect_simple(&TokenKind::LeftParen, "expected `(` after function name")?;
        let params = self.parse_params()?;
        let return_type = if self.consume_simple(&TokenKind::Arrow).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        let span = start.join(body.span);
        Some(FunctionDecl {
            name,
            handles: None,
            shared: false,
            params,
            return_type,
            body,
            span,
        })
    }

    /// `on GoalScored(team: f32) { ... }`: a function the host calls when the
    /// event is emitted. It returns nothing, because nothing waits for it.
    fn parse_handler(&mut self) -> Option<FunctionDecl> {
        let start = self.current().span;
        self.advance();
        let (event, event_span) = self.expect_identifier("expected an event name after `on`")?;
        self.expect_simple(&TokenKind::LeftParen, "expected `(` after event name")?;
        let params = self.parse_params()?;
        if self.at(&TokenKind::Arrow) {
            self.error_here(
                crate::codes::SyntaxCode::HandlerReturnsValue,
                "a handler returns nothing: nothing waits for it",
            );
            self.advance();
            self.parse_type()?;
        }
        let body = self.parse_block()?;
        let span = start.join(body.span);
        Some(FunctionDecl {
            name: handler_name(&event),
            handles: Some((event, event_span)),
            shared: false,
            params,
            return_type: None,
            body,
            span,
        })
    }

    /// `event GoalScored(team: f32);`
    pub(super) fn parse_event(&mut self) -> Option<Item> {
        let start = self.current().span;
        self.advance();
        let (name, _) = self.expect_identifier("expected an event name after `event`")?;
        self.expect_simple(&TokenKind::LeftParen, "expected `(` after event name")?;
        let params = self.parse_params()?;
        let end = self.expect_simple(&TokenKind::Semicolon, "expected `;` after event")?;
        Some(Item::Event(EventDecl {
            name,
            params,
            span: start.join(end),
        }))
    }

    /// `enum Phase { Lobby, Countdown, Play }`: names, comma-separated, and a
    /// trailing comma if you like.
    pub(super) fn parse_enum(&mut self) -> Option<Item> {
        let start = self.current().span;
        self.advance();
        let (name, _) = self.expect_identifier("expected a name after `enum`")?;
        self.expect_simple(&TokenKind::LeftBrace, "expected `{` after the enum's name")?;
        let mut variants = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            let (variant, span) = self.expect_identifier("expected a variant name")?;
            variants.push((variant, span));
            if self.consume_simple(&TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.expect_simple(&TokenKind::RightBrace, "expected `}` after the variants")?;
        Some(Item::Enum(crate::ast::EnumDecl {
            name,
            variants,
            span: start.join(end),
        }))
    }

    /// `struct Card { root: Entity, name: String }`: typed fields,
    /// comma-separated, and a trailing comma if you like.
    pub(super) fn parse_struct(&mut self) -> Option<Item> {
        let start = self.current().span;
        self.advance();
        let (name, _) = self.expect_identifier("expected a name after `struct`")?;
        self.expect_simple(
            &TokenKind::LeftBrace,
            "expected `{` after the struct's name",
        )?;
        let mut fields = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            let (field, span) = self.expect_identifier("expected a field name")?;
            self.expect_simple(
                &TokenKind::Colon,
                "expected `:` and the field's type -- a struct's fields are always typed",
            )?;
            let ty = self.parse_type()?;
            fields.push(crate::ast::StructField {
                name: field,
                span: span.join(ty.span),
                ty,
            });
            if self.consume_simple(&TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.expect_simple(&TokenKind::RightBrace, "expected `}` after the fields")?;
        Some(Item::Struct(crate::ast::StructDecl {
            name,
            fields,
            span: start.join(end),
        }))
    }

    /// `const ARENA: f32 = 12.0;`: always typed, always given a value.
    pub(super) fn parse_const(&mut self, shared: bool) -> Option<Item> {
        let start = self.current().span;
        self.advance();
        let (name, name_span) = self.expect_identifier("expected a name after `const`")?;
        self.expect_simple(
            &TokenKind::Colon,
            "expected `:` and the constant's type -- a constant is always typed",
        )?;
        let ty = self.parse_type()?;
        self.expect_simple(
            &TokenKind::Equal,
            "expected `=` and the constant's value -- a constant always has one",
        )?;
        let value = self.parse_expression()?;
        let end = self.expect_simple(&TokenKind::Semicolon, "expected `;` after the constant")?;
        Some(Item::Const(crate::ast::ConstDecl {
            name,
            shared,
            ty,
            value,
            name_span,
            span: start.join(end),
        }))
    }

    /// `state Game { var score: f32 = 0.0; }`: fields and nothing else.
    pub(super) fn parse_state(&mut self) -> Option<Item> {
        let start = self.current().span;
        self.advance();
        let (name, _) = self.expect_identifier("expected a name after `state`")?;
        self.expect_simple(&TokenKind::LeftBrace, "expected `{` after state name")?;
        let mut fields = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            let attributes = self.parse_attributes();
            if let Some(attribute) = attributes.first() {
                self.error_span(
                    crate::codes::SyntaxCode::AttributeNotAllowed,
                    attribute.span,
                    "attributes on state fields are not supported",
                );
            }
            if self.at(&TokenKind::Let) || self.at(&TokenKind::Var) {
                if let Some(field) = self.parse_field(Vec::new()) {
                    fields.push(field);
                } else {
                    self.synchronize_member();
                }
            } else {
                self.error_here(
                    crate::codes::SyntaxCode::StateFieldsOnly,
                    "a state holds only fields: `var name: f32 = 0.0;`",
                );
                // Past the offending token first: recovery stops at `fn`,
                // which is exactly what a state cannot hold.
                self.advance();
                self.synchronize_member();
            }
        }
        let end = self
            .expect_simple(&TokenKind::RightBrace, "expected `}` after state body")
            .unwrap_or_else(|| self.current().span);
        Some(Item::State(StateDecl {
            name,
            fields,
            span: start.join(end),
        }))
    }

    /// A parameter list after its `(`, up to and including the `)`.
    fn parse_params(&mut self) -> Option<Vec<Param>> {
        let mut params = Vec::new();
        if !self.at(&TokenKind::RightParen) {
            loop {
                let (name, name_span) = self.expect_identifier("expected parameter name")?;
                let ty = self.parse_optional_type();
                let end = ty.as_ref().map_or(name_span, |ty| ty.span);
                params.push(Param {
                    name,
                    ty,
                    span: name_span.join(end),
                });
                if self.consume_simple(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        self.expect_simple(&TokenKind::RightParen, "expected `)` after parameters")?;
        Some(params)
    }

    /// Whether the current token is this contextual word, with a name after
    /// it: `on Hit` starts a handler, `on` alone does not.
    pub(super) fn at_word(&self, word: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Identifier(name) if name == word)
            && matches!(
                self.tokens.get(self.cursor + 1).map(|token| &token.kind),
                Some(TokenKind::Identifier(_))
            )
    }

    pub(super) fn parse_optional_type(&mut self) -> Option<TypeRef> {
        if self.consume_simple(&TokenKind::Colon).is_some() {
            self.parse_type()
        } else {
            None
        }
    }

    pub(super) fn parse_type(&mut self) -> Option<TypeRef> {
        let (name, span) = self.expect_identifier("expected type name")?;
        if self.consume_simple(&TokenKind::Less).is_none() {
            return Some(TypeRef::plain(name, span));
        }
        let argument = self.parse_type()?;
        let end = self.expect_simple(&TokenKind::Greater, "expected `>` after type argument")?;
        Some(TypeRef {
            name,
            argument: Some(Box::new(argument)),
            span: span.join(end),
        })
    }
}
