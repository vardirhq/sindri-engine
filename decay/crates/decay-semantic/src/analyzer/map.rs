//! Maps: writing one, reading an entry, and what a map can be asked or told.
//!
//! A map is a value, like a list: assigning one copies it. So a change —
//! `m[key] = value`, `remove`, `clear` — is made to the map one variable or
//! field holds, and needs that variable or field to be one the script may
//! change.

use crate::codes::Code;
use decay_syntax::{Expr, MapOp, Span};

use crate::diagnostic::ValueMember;
use crate::types::Type;

use super::Analyzer;

impl Analyzer<'_, '_> {
    /// `["a": 1.0, "b": 2.0]`: a map of the first entry's key and value
    /// types, which every other entry must fit. An empty one, `[:]`, fits any
    /// map it is put in.
    pub(super) fn map_literal_type(&mut self, entries: &[(Expr, Expr)]) -> Type {
        let mut key_type = Type::Unknown;
        let mut value_type = Type::Unknown;
        for (position, (key, value)) in entries.iter().enumerate() {
            let this_key = self.expr_type(key);
            let this_value = self.expr_type(value);
            if position == 0 {
                if !this_key.is_key() {
                    self.error(
                        Code::TypeMismatch,
                        key.span,
                        format!(
                            "a map is keyed by text, numbers, flags, enum variants or entities, \
                             not `{}`",
                            this_key.display_name()
                        ),
                    );
                }
                key_type = this_key;
                value_type = this_value;
                continue;
            }
            for (wanted, given, span, what) in [
                (&key_type, &this_key, key.span, "key"),
                (&value_type, &this_value, value.span, "value"),
            ] {
                if !self.compatible(wanted, given) {
                    self.error(
                        Code::MixedList,
                        span,
                        format!(
                            "a map holds one {what} type: this is `{}`, and the first is `{}`",
                            given.display_name(),
                            wanted.display_name()
                        ),
                    );
                }
            }
        }
        Type::Map(Box::new(key_type), Box::new(value_type))
    }

    /// `map[key]`: the value's type, having checked the key.
    pub(super) fn map_index_type(&mut self, key: &Type, value: &Type, index: &Expr) -> Type {
        let given = self.expr_type(index);
        self.require_type(&given, key, index.span);
        value.clone()
    }

    /// `map[key] = value`: the value's type, once the map is one the script
    /// may change. Noted for the lowering, which cannot tell a map's entry
    /// from a list's element by how it is written.
    pub(super) fn map_entry_target_type(
        &mut self,
        object: &Expr,
        key: &Type,
        value: &Type,
        index: &Expr,
        target: Span,
    ) -> Type {
        let given = self.expr_type(index);
        self.require_type(&given, key, index.span);
        self.check_map_place(object, "[]");
        self.value_members
            .insert(target, ValueMember::Map(MapOp::Set));
        value.clone()
    }

    /// `map.op(args)`.
    pub(super) fn map_call_type(
        &mut self,
        object: &Expr,
        (key, value): (&Type, &Type),
        field: &str,
        args: &[Expr],
        span: Span,
    ) -> Type {
        let Some((op, _)) = MapOp::named(field) else {
            let message = if crate::types::is_length(field) {
                format!("`{field}` is a property, not a function -- write `.{field}`")
            } else {
                missing_member(key, value, field)
            };
            self.error(Code::UnknownMember, span, message);
            for argument in args {
                self.expr_type(argument);
            }
            return Type::Unknown;
        };
        let signature = crate::members::map_op_signature(op, key, value);
        if op.changes() {
            self.check_map_place(object, field);
        }
        self.check_call(&signature, args, span);
        self.value_members.insert(span, ValueMember::Map(op));
        signature.return_type
    }

    /// A map's member read without calling it: `length`, or a mistake.
    pub(super) fn map_member_type(
        &mut self,
        key: &Type,
        value: &Type,
        field: &str,
        span: Span,
    ) -> Type {
        if crate::types::is_length(field) {
            self.value_members.insert(span, ValueMember::Length);
            return Type::F32;
        }
        if MapOp::named(field).is_some() {
            self.error(
                Code::FunctionNotCalled,
                span,
                format!("`{field}` on a map is a function -- call it: `.{field}(...)`"),
            );
        } else {
            self.error(Code::UnknownMember, span, missing_member(key, value, field));
        }
        Type::Unknown
    }

    /// A change is made to the map a variable, parameter or field of this
    /// script holds, and only when that one may change.
    fn check_map_place(&mut self, object: &Expr, what: &str) {
        let doing = if what == "[]" {
            "an entry can be set".to_owned()
        } else {
            format!("`{what}` can change it")
        };
        match self.place_root(object) {
            Some((_, true)) => {}
            Some((name, false)) => self.error(
                Code::Immutable,
                object.span,
                format!("`{name}` is a `let`, so it cannot change -- declare it `var` if {doing}"),
            ),
            None => self.error(
                Code::NotAPlace,
                object.span,
                format!(
                    "only a map in a variable or a field of this script can change, and this is \
                     neither -- put it in a `var` first if {doing}"
                ),
            ),
        }
    }
}

fn missing_member(key: &Type, value: &Type, field: &str) -> String {
    let operations = MapOp::ALL
        .iter()
        .map(|(_, name, _)| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "`{}` has no member `{field}`; it has `{}`, and {operations}",
        Type::Map(Box::new(key.clone()), Box::new(value.clone())).display_name(),
        crate::types::LENGTH
    )
}
