//! Turning an analysed program into instructions.
//!
//! A new statement or expression form is a function in the matching
//! leaf and one arm in the match that dispatches to it.

mod expr;
mod stmt;

use decay_semantic::{Analysis, ConstValue, ConstantUses, ValueMember, ValueMembers};
use decay_syntax::{FunctionDecl, Item, Member, Span};

use crate::ir::{ContainerKind, Instruction, IrContainer, IrField, IrFunction, IrProgram};

/// The walk, and what the analysis told it.
///
/// Stateful because lowering is not purely syntactic: a member read may be a
/// path the host answers or a property of a value the language owns, and only
/// the analysis knows which. Guessing from the member's name would make `len`
/// a name no host type could ever use.
#[allow(clippy::zero_sized_map_values)]
pub(crate) struct Lowerer<'a> {
    value_members: &'a ValueMembers,
    constant_uses: &'a ConstantUses,
    method_calls: &'a std::collections::HashMap<Span, String>,
    struct_defaults:
        &'a std::collections::BTreeMap<String, std::collections::BTreeMap<String, ConstValue>>,
    structs: &'a std::collections::BTreeMap<String, Vec<(String, decay_semantic::Type)>>,
}

impl<'a> Lowerer<'a> {
    pub(crate) fn lower_program(analysis: &'a Analysis) -> IrProgram {
        let lowerer = Self {
            value_members: &analysis.value_members,
            constant_uses: &analysis.constant_uses,
            method_calls: &analysis.method_calls,
            struct_defaults: &analysis.struct_defaults,
            structs: &analysis.structs,
        };
        let containers = analysis
            .program
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Script(container) => {
                    Some(lowerer.lower_container(ContainerKind::Script, container))
                }
                Item::Component(container) => {
                    Some(lowerer.lower_container(ContainerKind::Component, container))
                }
                // A declaration and nothing else: an emit is a call the host
                // answers, and a handler is a function of the script it is in.
                Item::Event(_)
                | Item::State(_)
                | Item::Function(_)
                | Item::Enum(_)
                | Item::Struct(_)
                | Item::Const(_) => None,
            })
            .collect();

        let mut program = IrProgram {
            containers,
            enums: analysis.enums.clone(),
            structs: analysis.structs.clone(),
        };
        let shared = analysis
            .program
            .items
            .iter()
            .flat_map(|item| match item {
                Item::Function(function) => vec![lowerer.lower_function(function)],
                Item::Struct(declared) => declared
                    .methods
                    .iter()
                    .map(|method| lowerer.lower_method(&declared.name, method))
                    .collect(),
                _ => Vec::new(),
            })
            .collect::<Vec<_>>();
        if !shared.is_empty() {
            program.link(shared);
        }
        program
    }

    /// A struct's fields in declared order; empty for a name the analysis
    /// already refused.
    pub(super) fn struct_fields(&self, name: &str) -> Vec<String> {
        self.structs
            .get(name)
            .map(|fields| fields.iter().map(|(field, _)| field.clone()).collect())
            .unwrap_or_default()
    }

    /// The value a name at this span means, when the analysis found it names
    /// a constant: written in the name's place, so nothing is looked up.
    pub(super) fn constant_at(&self, span: Span) -> Option<crate::ir::Constant> {
        Some(Self::constant(self.constant_uses.get(&span)?))
    }

    /// A struct field's default, as the constant written for it.
    pub(super) fn default_of(&self, structure: &str, field: &str) -> Option<crate::ir::Constant> {
        Some(Self::constant(
            self.struct_defaults.get(structure)?.get(field)?,
        ))
    }

    fn constant(value: &ConstValue) -> crate::ir::Constant {
        use crate::ir::Constant;
        match value {
            ConstValue::Number(number) => Constant::Number(*number),
            ConstValue::Bool(flag) => Constant::Bool(*flag),
            ConstValue::Text(text) => Constant::String(text.clone()),
            ConstValue::Variant {
                enumeration,
                variant,
            } => Constant::Variant(format!("{enumeration}.{variant}")),
        }
    }

    /// What the analysis decided a member read at this span is, if anything.
    pub(super) fn value_member(&self, span: Span) -> Option<ValueMember> {
        self.value_members.get(&span).copied()
    }

    pub(crate) fn lower_container(
        &self,
        kind: ContainerKind,
        container: &decay_syntax::ContainerDecl,
    ) -> IrContainer {
        let mut fields = Vec::new();
        let mut functions = Vec::new();

        for member in &container.members {
            match member {
                Member::Field(field) => {
                    let initializer = field.initializer.as_ref().map(|expr| {
                        let mut instructions = Vec::new();
                        self.lower_expr(expr, &mut instructions);
                        instructions
                    });
                    fields.push(IrField {
                        name: field.name.clone(),
                        mutable: field.mutable,
                        exported: field
                            .attributes
                            .iter()
                            .any(|attribute| attribute.name == decay_syntax::vocabulary::EXPORT),
                        type_name: field.ty.as_ref().map(|ty| ty.name.clone()),
                        ty: field.ty.as_ref().map(decay_semantic::Type::from_ref),
                        initializer,
                    });
                }
                Member::Function(function) => functions.push(self.lower_function(function)),
            }
        }

        IrContainer {
            kind,
            name: container.name.clone(),
            fields,
            functions,
        }
    }

    /// The function a struct's method is called as, `this` taken first.
    pub(crate) fn lower_method(&self, structure: &str, method: &FunctionDecl) -> IrFunction {
        let mut function = self.lower_function(method);
        function.name = decay_semantic::method_function(structure, &method.name);
        function.params.insert(0, "this".to_owned());
        function
    }

    /// The function a call at this span is to a struct's method as, if it is.
    pub(super) fn method_call(&self, span: Span) -> Option<&str> {
        self.method_calls.get(&span).map(String::as_str)
    }

    pub(crate) fn lower_function(&self, function: &FunctionDecl) -> IrFunction {
        let mut instructions = Vec::new();
        // A function body is where a loop can exist, and each body starts with
        // none open; a field initializer is an expression and cannot contain one.
        self.lower_block(
            &function.body,
            &mut instructions,
            &mut stmt::Loops::default(),
        );
        if !matches!(instructions.last(), Some(Instruction::Return)) {
            instructions.push(Instruction::Return);
        }

        IrFunction {
            name: function.name.clone(),
            params: function
                .params
                .iter()
                .map(|param| param.name.clone())
                .collect(),
            instructions,
        }
    }
}
