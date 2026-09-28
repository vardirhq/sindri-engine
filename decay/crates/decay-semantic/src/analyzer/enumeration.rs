//! Enums: a type whose values are the variants it names, and the `match`
//! that takes one apart.
//!
//! `enum Phase { Lobby, Countdown, Play }` replaces a number whose meaning
//! lived in a comment — "0 is the lobby, 1 the countdown, 2 play" — with
//! names the analysis checks. A variant is always written with its enum's
//! name, `Phase.Lobby`, so reading one never needs the declaration open, and a
//! `match` on one must say what happens for every variant or say `_`.

use crate::codes::Code;
use std::collections::HashSet;

use decay_syntax::{EnumDecl, Expr, ExprKind, Item, MatchArm, Pattern, Program, Span};

use crate::diagnostic::ValueMember;
use crate::types::{Type, enum_type};

use super::Analyzer;

impl Analyzer<'_, '_> {
    /// Adds this program's own enums to the host's, checking each.
    pub(super) fn collect_enums(&mut self, program: &Program) {
        self.enums = self.environment.enums.clone();
        let mut own = HashSet::new();
        for item in &program.items {
            let Item::Enum(declared) = item else {
                continue;
            };
            if !own.insert(declared.name.clone()) {
                self.error(
                    Code::Duplicate,
                    declared.span,
                    format!("duplicate declaration `{}`", declared.name),
                );
                continue;
            }
            self.check_enum(declared);
            let variants = declared
                .variants
                .iter()
                .map(|(name, _)| name.clone())
                .collect();
            self.enums.insert(declared.name.clone(), variants);
        }
    }

    fn check_enum(&mut self, declared: &EnumDecl) {
        let name = &declared.name;
        if self.environment.ambiguous_enums.contains(name) {
            self.error(
                Code::DeclaredInSeveralFiles,
                declared.span,
                format!("enum `{name}` is declared in more than one file; keep one"),
            );
        }
        if self.is_container(name)
            || self.environment.globals.contains_key(name)
            || self.environment.get_type(name).is_some()
        {
            self.error(
                Code::NameTaken,
                declared.span,
                format!("`{name}` is already a name; an enum needs one of its own"),
            );
        }
        if declared.variants.is_empty() {
            self.error(
                Code::EmptyDeclaration,
                declared.span,
                format!("enum `{name}` has no variants, so nothing could ever be one"),
            );
        }
        let mut seen = HashSet::new();
        for (variant, span) in &declared.variants {
            if !seen.insert(variant) {
                self.error(
                    Code::Duplicate,
                    *span,
                    format!("`{name}.{variant}` is declared twice"),
                );
            }
        }
    }

    /// Every enum this program could name, for the analysis to hand on.
    pub(crate) fn known_enums(&self) -> std::collections::BTreeMap<String, Vec<String>> {
        self.enums
            .iter()
            .map(|(name, variants)| (name.clone(), variants.clone()))
            .collect()
    }

    /// The type an enum's name has in an expression, when `name` is one.
    pub(super) fn enum_name_type(&mut self, name: &str, span: Span) -> Option<Type> {
        if self.enums.contains_key(name) {
            return Some(Type::Named(enum_type(name)));
        }
        if self.environment.ambiguous_enums.contains(name) {
            self.error(
                Code::DeclaredInSeveralFiles,
                span,
                format!("enum `{name}` is declared in more than one file; keep one"),
            );
            return Some(Type::Unknown);
        }
        None
    }

    /// `Phase.Lobby`: the variant's type, noting it for the lowering. `None`
    /// when the object is not an enum's name.
    pub(super) fn variant_type(
        &mut self,
        object_type: &Type,
        field: &str,
        span: Span,
    ) -> Option<Type> {
        let Type::Named(name) = object_type else {
            return None;
        };
        let enumeration = name.strip_prefix("enum ")?;
        let variants = self.enums.get(enumeration)?;
        if variants.iter().any(|variant| variant == field) {
            self.value_members.insert(span, ValueMember::Variant);
            return Some(Type::Named(enumeration.to_owned()));
        }
        let listed = variants
            .iter()
            .map(|variant| format!("`{variant}`"))
            .collect::<Vec<_>>()
            .join(", ");
        self.error(
            Code::UnknownMember,
            span,
            format!("`{enumeration}` has no variant `{field}`; it has {listed}"),
        );
        Some(Type::Unknown)
    }

    /// The enum an unannotated field's initializer names a variant of:
    /// `var phase = Phase.Lobby;`.
    pub(super) fn variant_initializer_type(&self, initializer: Option<&Expr>) -> Option<Type> {
        let ExprKind::Member { object, field } = &initializer?.kind else {
            return None;
        };
        let ExprKind::Identifier(name) = &object.kind else {
            return None;
        };
        self.enums
            .get(name)
            .filter(|variants| variants.iter().any(|variant| variant == field))
            .map(|_| Type::Named(name.clone()))
    }

    /// Checks a `match`: the subject is an enum's value, every pattern names
    /// one of its variants once, nothing follows `_`, and every variant is
    /// covered unless `_` is there.
    pub(super) fn analyze_match(&mut self, subject: &Expr, arms: &[MatchArm], span: Span) {
        let subject_type = self.expr_type(subject);
        let enumeration = match &subject_type {
            Type::Named(name) if self.enums.contains_key(name) => Some(name.clone()),
            Type::Unknown => None,
            other => {
                self.error(
                    Code::MatchNeedsEnum,
                    subject.span,
                    format!(
                        "`match` takes a value of an enum, found `{}`",
                        other.display_name()
                    ),
                );
                None
            }
        };
        let variants = enumeration
            .as_ref()
            .and_then(|name| self.enums.get(name).cloned())
            .unwrap_or_default();
        let mut covered = HashSet::new();
        let mut wildcard = false;
        for arm in arms {
            for pattern in &arm.patterns {
                if wildcard {
                    self.error(
                        Code::UnreachableArm,
                        pattern.span(),
                        "this can never be reached: `_` above already takes everything".to_owned(),
                    );
                }
                match pattern {
                    Pattern::Wildcard(_) => wildcard = true,
                    Pattern::Variant {
                        enumeration: written,
                        variant,
                        span,
                    } => {
                        let Some(expected) = &enumeration else {
                            continue;
                        };
                        if written != expected {
                            self.error(
                                Code::WrongEnum,
                                *span,
                                format!("`{written}.{variant}` is not a `{expected}`"),
                            );
                        } else if !variants.contains(variant) {
                            self.error(
                                Code::UnknownMember,
                                *span,
                                format!("`{expected}` has no variant `{variant}`"),
                            );
                        } else if !covered.insert(variant.clone()) {
                            self.error(
                                Code::Duplicate,
                                *span,
                                format!("`{expected}.{variant}` is already matched above"),
                            );
                        }
                    }
                }
            }
            self.analyze_block(&arm.body, true);
        }
        if let Some(name) = enumeration
            && !wildcard
        {
            let missing = variants
                .iter()
                .filter(|variant| !covered.contains(*variant))
                .map(|variant| format!("`{name}.{variant}`"))
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                self.error(
                    Code::NotExhaustive,
                    span,
                    format!(
                        "this `match` does not say what happens for {} -- add an arm, or `_`",
                        missing.join(", ")
                    ),
                );
            }
        }
    }
}
