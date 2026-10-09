//! Do the Decay scripting contracts describe the surface a script actually has?
//!
//! The main scripting contract and focused runtime-camera contract list, in
//! tables, every path and call a script can reach. Those lists are believed by
//! authors, so a surface that grows without its documentation growing with it
//! fails here, and so does documentation that promises something withdrawn.
//!
//! When this fails, the fix is in the documentation, not in the assertion.

use std::collections::{BTreeMap, BTreeSet};

use decay_semantic::{Environment, ExternalSymbol, Type};
use sindri_decay::environment;

const DOC: &str = concat!(
    include_str!("../../../docs/scripting.md"),
    "\n",
    include_str!("../../../docs/camera-runtime-scripting.md")
);

/// The first backticked cell of every table row in the contracts.
fn documented_cells() -> Vec<String> {
    DOC.lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("| `")?;
            let (cell, _) = rest.split_once('`')?;
            Some(cell.to_owned())
        })
        .collect()
}

/// Expands `position.{x,y,z}` into three names.
fn expand(name: &str) -> Vec<String> {
    let Some((head, rest)) = name.split_once('{') else {
        return vec![name.to_owned()];
    };
    let Some((choices, tail)) = rest.split_once('}') else {
        return vec![name.to_owned()];
    };
    choices
        .split(',')
        .map(|choice| format!("{head}{}{tail}", choice.trim()))
        .collect()
}

struct Documented {
    entity_paths: BTreeSet<String>,
    namespaces: BTreeMap<String, BTreeSet<String>>,
}

fn documented() -> Documented {
    let mut documented = Documented {
        entity_paths: BTreeSet::new(),
        namespaces: BTreeMap::new(),
    };
    for cell in documented_cells() {
        for name in expand(&cell) {
            let name = name
                .split_once('(')
                .map_or(name.clone(), |(head, _)| head.to_owned());
            if name.starts_with("this.") {
                documented.entity_paths.insert(name);
            } else if let Some((namespace, member)) = name.split_once('.') {
                documented
                    .namespaces
                    .entry(namespace.to_owned())
                    .or_default()
                    .insert(member.to_owned());
            }
        }
    }
    documented
}

fn described_entity_paths(environment: &Environment) -> BTreeSet<String> {
    fn walk(
        environment: &Environment,
        prefix: String,
        symbol: &ExternalSymbol,
        into: &mut BTreeSet<String>,
    ) {
        if matches!(symbol, ExternalSymbol::Function(_)) {
            into.insert(prefix);
            return;
        }
        let (ExternalSymbol::Value(ty) | ExternalSymbol::ReadOnlyValue(ty)) = symbol else {
            return;
        };
        if let Some(dimensions) = ty.dimensions() {
            for component in &["x", "y", "z"][..dimensions] {
                into.insert(format!("{prefix}.{component}"));
            }
            into.insert(prefix);
            return;
        }
        if *ty == Type::Color {
            for channel in decay_semantic::CHANNELS {
                into.insert(format!("{prefix}.{channel}"));
            }
            into.insert(prefix);
            return;
        }
        let Type::Named(name) = ty else {
            into.insert(prefix);
            return;
        };
        let described = environment
            .get_type(name)
            .unwrap_or_else(|| panic!("`{name}` is named by the surface but never described"));
        for (field, member) in described.members() {
            walk(environment, format!("{prefix}.{field}"), member, into);
        }
    }

    let mut paths = BTreeSet::new();
    for (member, symbol) in environment.this().members() {
        walk(environment, format!("this.{member}"), symbol, &mut paths);
    }
    paths
}

fn described_members(environment: &Environment, namespace: &str) -> BTreeSet<String> {
    environment
        .get_type(namespace)
        .unwrap_or_else(|| panic!("`{namespace}` is not described"))
        .members()
        .map(|(name, _)| name.to_owned())
        .collect()
}

#[test]
fn the_document_lists_exactly_the_paths_a_script_can_reach() {
    let environment = environment();
    let documented = documented();

    assert_eq!(
        documented.entity_paths,
        described_entity_paths(&environment),
        "the Decay scripting contracts and host surface disagree about what a script can reach on its entity"
    );
}

#[test]
fn the_document_lists_exactly_the_namespaces_a_script_can_reach() {
    let environment = environment();
    let documented = documented();

    let offered: BTreeSet<String> = environment
        .globals()
        .filter_map(|(name, symbol)| match symbol {
            ExternalSymbol::Value(Type::Named(_)) => Some(name.to_owned()),
            _ => None,
        })
        .collect();
    assert_eq!(
        documented
            .namespaces
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        offered,
        "the Decay scripting contracts and host disagree about which namespaces exist"
    );

    for namespace in offered {
        assert_eq!(
            documented.namespaces[&namespace],
            described_members(&environment, &namespace),
            "the Decay scripting contracts and host disagree about `{namespace}`"
        );
    }
}

#[test]
fn the_document_lists_exactly_the_maths_a_script_can_do() {
    let sentence = DOC
        .split_once("That is the entire standard library")
        .expect("the document still says what the standard library is")
        .0;
    let listed: BTreeSet<String> = sentence
        .rsplit("### Maths")
        .next()
        .expect("the maths section")
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect();

    let described: BTreeSet<String> = environment()
        .globals()
        .filter(|(name, symbol)| matches!(symbol, ExternalSymbol::Function(_)) && *name != "print")
        .map(|(name, _)| name.to_owned())
        .collect();

    assert_eq!(
        listed, described,
        "docs/scripting.md and the host disagree about the standard library"
    );
}
