//! What a name at a place in the source holds, and what a value of that type
//! can be asked: a local's type from where it was bound, and the members the
//! language gives a vector, a timer, text, a list or a struct.

use std::collections::BTreeMap;

use decay_semantic::members::{
    list_op_signature, string_op_signature, timer_property_type, vector_op_signature,
};
use decay_semantic::{
    Analysis, COMPONENTS, Environment, ExternalSymbol, FunctionType, LENGTH, ListOp, StringOp,
    TimerProperty, Type, VectorOp,
};

use crate::support::type_members;

/// The struct layouts a file may name: its own and the project's.
pub(crate) type Structs = BTreeMap<String, Vec<(String, Type)>>;

/// The type the nearest binding of `name` before `offset` gave it: a local,
/// a parameter, a loop's binding, or a field of the script around `offset`.
pub(crate) fn local_type(analysis: &Analysis, name: &str, offset: usize) -> Option<Type> {
    analysis
        .bindings
        .iter()
        .filter(|binding| {
            binding.name == name
                && binding.scope.start <= offset
                && offset <= binding.scope.end
                && binding.declared <= offset
                && binding.ty != Type::Unknown
        })
        .max_by_key(|binding| binding.declared)
        .map(|binding| binding.ty.clone())
}

/// The type of a field of the script around `offset`: `this.name`.
pub(crate) fn field_type(analysis: &Analysis, name: &str, offset: usize) -> Option<Type> {
    analysis
        .bindings
        .iter()
        .filter(|binding| {
            binding.name == name
                && binding.scope.start <= offset
                && offset <= binding.scope.end
                && binding.ty != Type::Unknown
        })
        .max_by_key(|binding| binding.scope.end - binding.scope.start)
        .map(|binding| binding.ty.clone())
}

fn value(ty: Type) -> ExternalSymbol {
    ExternalSymbol::Value(ty)
}

/// A property as a value, anything with parameters as a function to call.
fn member(signature: FunctionType) -> ExternalSymbol {
    if signature.params.is_empty() {
        value(signature.return_type)
    } else {
        ExternalSymbol::Function(signature)
    }
}

/// What the language gives a value of this type, or `None` for a type whose
/// members the host describes. Signatures come from
/// `decay_semantic::members`, the table the analysis checks calls against.
pub(crate) fn value_members(ty: &Type, structs: &Structs) -> Option<Vec<(String, ExternalSymbol)>> {
    Some(match ty {
        Type::Vec2 | Type::Vec3 => {
            let dimensions = ty.dimensions().unwrap_or(2);
            let mut members: Vec<(String, ExternalSymbol)> = COMPONENTS[..dimensions]
                .iter()
                .map(|component| ((*component).to_owned(), value(Type::F32)))
                .collect();
            members.extend(
                VectorOp::ALL.iter().map(|(op, name, _)| {
                    ((*name).to_owned(), member(vector_op_signature(*op, ty)))
                }),
            );
            members
        }
        Type::Timer => TimerProperty::ALL
            .iter()
            .map(|(property, name)| ((*name).to_owned(), value(timer_property_type(*property))))
            .collect(),
        Type::String => StringOp::ALL
            .iter()
            .map(|(op, name, _)| ((*name).to_owned(), member(string_op_signature(*op))))
            .collect(),
        Type::Array(element) => {
            let mut members = vec![(LENGTH.to_owned(), value(Type::F32))];
            members.extend(ListOp::ALL.iter().map(|(op, name, _)| {
                // Every list operation is a call, `pop()` included.
                let signature = list_op_signature(*op, element);
                ((*name).to_owned(), ExternalSymbol::Function(signature))
            }));
            members
        }
        Type::Named(name) => structs
            .get(name)?
            .iter()
            .map(|(field, ty)| (field.clone(), value(ty.clone())))
            .collect(),
        _ => return None,
    })
}

/// Everything a value of this type offers: the language's members for one it
/// owns, the host's description for one it does not.
pub(crate) fn members(
    environment: &Environment,
    ty: &Type,
    structs: &Structs,
) -> Vec<(String, ExternalSymbol)> {
    value_members(ty, structs).unwrap_or_else(|| {
        type_members(environment, ty).map_or_else(Vec::new, |host| {
            host.members()
                .map(|(name, symbol)| (name.to_owned(), symbol.clone()))
                .collect()
        })
    })
}

/// The type of `ty.name`, through either.
pub(crate) fn member_type(
    environment: &Environment,
    ty: &Type,
    name: &str,
    structs: &Structs,
) -> Option<Type> {
    members(environment, ty, structs)
        .into_iter()
        .find(|(member, _)| member == name)
        .map(|(_, symbol)| match symbol {
            ExternalSymbol::Value(ty) => ty,
            ExternalSymbol::Function(function) => function.return_type,
        })
}

/// The members at the end of `chain` when it starts at a value the script
/// holds — a local, a parameter, a field, `this.field` — typed by the
/// analysis and walked through the language's members as well as the
/// host's. `None` when the chain starts anywhere else.
pub(crate) fn held_members(
    environment: &Environment,
    source: &str,
    offset: usize,
    chain: &[String],
) -> Option<Vec<(String, ExternalSymbol)>> {
    let first = chain.first()?;
    let analysis = decay_semantic::analyze_with_environment(source, environment);
    let (mut ty, from) = if first == "this" {
        (field_type(&analysis, chain.get(1)?, offset)?, 2)
    } else {
        (local_type(&analysis, first, offset)?, 1)
    };
    for segment in &chain[from..] {
        let Some(next) = member_type(environment, &ty, segment, &analysis.structs) else {
            return Some(Vec::new());
        };
        ty = next;
    }
    Some(members(environment, &ty, &analysis.structs))
}

#[cfg(test)]
mod tests {
    use decay_semantic::Environment;

    use super::held_members;

    const SOURCE: &str = r#"struct Card { name: String, at: Vec2, tags: List<String> }
script Dealer {
    var timer = Timer(1.0);
    fn deal(p: Vec3) {
        let card = Card(name: "a", at: Vec2(0.0, 0.0), tags: []);
        card.
        let late = 1.0;
    }
}"#;

    fn names(chain: &[&str]) -> Vec<String> {
        let offset = SOURCE.find("card.\n").expect("the cursor") + "card.".len();
        let chain: Vec<String> = chain.iter().map(|part| (*part).to_owned()).collect();
        held_members(&Environment::new(), SOURCE, offset, &chain)
            .unwrap_or_default()
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    #[test]
    fn a_held_value_offers_what_its_type_has() {
        assert_eq!(names(&["card"]), ["name", "at", "tags"]);
        assert!(names(&["card", "at"]).starts_with(&["x".to_owned(), "y".to_owned()]));
        assert!(names(&["card", "at"]).contains(&"normalized".to_owned()));
        assert!(names(&["card", "name"]).contains(&"starts_with".to_owned()));
        assert!(names(&["card", "tags"]).contains(&"push".to_owned()));
        assert!(names(&["card", "tags"]).contains(&"length".to_owned()));
        assert_eq!(names(&["p"])[..3], ["x", "y", "z"]);
        assert_eq!(
            names(&["this", "timer"]),
            ["done", "left", "duration", "progress"]
        );
        assert_eq!(names(&["timer"]), ["done", "left", "duration", "progress"]);
    }

    #[test]
    fn a_name_bound_later_or_nowhere_offers_nothing() {
        assert!(names(&["late"]).is_empty());
        assert!(names(&["nobody"]).is_empty());
        assert!(names(&["card", "colour"]).is_empty());
    }
}
