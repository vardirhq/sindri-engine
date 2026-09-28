//! What a script declares it wants authored.
//!
//! This is the capability that justified Decay being statically typed rather
//! than embedding a dynamic language, and until now nothing used it. A property
//! panel needs a **declared, named, typed** field it can draw a widget for
//! without executing anything — and `@export let speed: f32 = 6.0;` is exactly
//! that, sitting in the IR since the language's first commit.
//!
//! The default is the one thing that does require executing something: an
//! initializer is instructions, not a value, so it is evaluated the same way a
//! real instance evaluates it. Doing anything else here would mean a second
//! answer to "what does this field start as".

use decay_ir::IrProgram;
use decay_runtime::{EmptyHost, Runtime, Value};

/// One `@export` field of a script, as an authoring surface sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct ScriptExport {
    pub name: String,
    /// The type the script declared, if it declared one.
    ///
    /// `None` where a field was written without a type annotation. A panel
    /// draws such a field from its default's shape instead, which is the best
    /// anyone can do and is why annotating is worth encouraging.
    pub type_name: Option<String>,
    /// What the field starts as when the scene says nothing.
    pub default: Value,
    /// For an enum field, its variants in order: what a panel offers to pick
    /// from. Empty for any other field.
    pub choices: Vec<String>,
    /// For a list, what one element is: its type, its choices or fields, and
    /// in `default` what a fresh one starts as. `None` for anything else.
    pub element: Option<Box<ScriptExport>>,
    /// For a struct, each field, in declared order, as an export of its own.
    /// Empty for anything else.
    pub fields: Vec<ScriptExport>,
}

impl ScriptExport {
    /// The export a value of type `ty` is, named `name` and starting as
    /// `default`: a list says what its elements are, and a struct what its
    /// fields are, all the way down.
    fn described(
        program: &IrProgram,
        name: &str,
        ty: Option<&decay_semantic::Type>,
        default: Value,
        depth: usize,
    ) -> Self {
        use decay_semantic::Type;
        let type_name = ty
            .filter(|ty| !matches!(ty, Type::Unknown))
            .map(|ty| ty.display_name().into_owned());
        let enumeration = match ty {
            Some(Type::Named(named)) if program.variants(named).is_some() => Some(named.clone()),
            _ => match &default {
                Value::Variant(variant) => variant.split('.').next().map(str::to_owned),
                _ => None,
            },
        };
        let choices = enumeration
            .as_deref()
            .and_then(|named| program.variants(named).map(<[String]>::to_vec))
            .unwrap_or_default();
        let deeper = depth < 8;
        let element = match ty {
            Some(Type::Array(element)) if deeper => Some(Box::new(Self::described(
                program,
                "",
                Some(element),
                crate::scripts::blank(program, element),
                depth + 1,
            ))),
            _ => None,
        };
        let fields = match (ty, &default) {
            (Some(Type::Named(named)), Value::Struct { fields: values, .. }) if deeper => program
                .struct_fields(named)
                .unwrap_or_default()
                .iter()
                .zip(values.iter())
                .map(|((field, field_type), value)| {
                    Self::described(program, field, Some(field_type), value.clone(), depth + 1)
                })
                .collect(),
            _ => Vec::new(),
        };
        Self {
            name: name.to_owned(),
            type_name: type_name.or(enumeration),
            default,
            choices,
            element,
            fields,
        }
    }
}

/// Every `@export` field of one container, in declaration order.
///
/// Declaration order rather than sorted: the author chose it, and a panel that
/// reorders someone's fields alphabetically is a panel that fights them.
pub(crate) fn exports_of(program: &IrProgram, script: &str) -> Option<Vec<ScriptExport>> {
    let container = program
        .containers
        .iter()
        .find(|container| container.name == script)?;

    // Instantiated with a host that answers nothing, because an initializer
    // that reached the world would be asking a question the panel has no
    // entity to answer. Such a field simply has no default here, and the panel
    // says so rather than inventing one.
    let mut runtime = Runtime::new(program, EmptyHost);
    let instance = runtime.instantiate(script).ok();

    Some(
        container
            .fields
            .iter()
            .filter(|field| field.exported)
            .map(|field| {
                let default = instance
                    .as_ref()
                    .and_then(|instance| instance.field(&field.name))
                    .cloned()
                    .unwrap_or(Value::Null);
                // A field written without a type is described by its default
                // only when that is a list or a struct, which a panel cannot
                // draw without knowing what is inside.
                let ty = crate::scripts::field_type(field.ty.as_ref(), Some(&default))
                    .filter(|ty| field.ty.is_some() || crate::scripts::is_compound(program, ty));
                let mut export =
                    ScriptExport::described(program, &field.name, ty.as_ref(), default, 0);
                // As the script wrote it where it wrote one: `Profile`, not a
                // type the panel would read differently.
                if export.choices.is_empty()
                    && field.type_name.is_some()
                    && export.element.is_none()
                    && export.fields.is_empty()
                {
                    export.type_name.clone_from(&field.type_name);
                }
                export
            })
            .collect(),
    )
}
