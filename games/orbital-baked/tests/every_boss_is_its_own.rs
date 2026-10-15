//! No two bosses are the same boss.
//!
//! Ten of the twelve used to share one prefab, one script and one hull,
//! separated only by a `kind` number and a tint. That is a shape a codebase
//! slides back into the moment somebody needs an eleventh boss in a hurry, and
//! nothing fails when it does — the game runs perfectly well with every boss
//! wearing the same hull, which is exactly the problem.
//!
//! So the roster is asserted rather than trusted: its own script, and its own
//! shape — a side count and colour no other boss draws.

use std::collections::HashSet;

use orbital_baked::Run;

/// The roster, in the order a run meets them. Index is what `boss_kind` and the
/// title's picker count in.
const ROSTER: [&str; 12] = [
    "warden",
    "harrower",
    "prism",
    "singularity",
    "crown",
    "brood",
    "mirror",
    "architect",
    "spine",
    "leviathan",
    "last-light",
    "aegis",
];

fn prefab_json(run: &Run, ident: &str) -> serde_json::Value {
    let prefab = run
        .session
        .prefabs()
        .get(&format!("prefabs/{ident}.prefab"))
        .unwrap_or_else(|| panic!("{ident} ships a prefab"));
    serde_json::to_value(prefab).expect("the prefab serialises")
}

fn component<'a>(json: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    &json["entities"][0]["components"][name]
}

#[test]
fn every_boss_runs_its_own_script() {
    let run = Run::open().expect("the project opens");
    let mut sources = HashSet::new();
    let mut names = HashSet::new();
    for ident in ROSTER {
        let json = prefab_json(&run, ident);
        let script = component(&json, "sindri.script");
        let source = script["source"].as_str().expect("a script source");
        let name = script["script"].as_str().expect("a script name");
        assert!(
            sources.insert(source.to_owned()),
            "{ident} shares {source} with another boss"
        );
        assert!(
            names.insert(name.to_owned()),
            "{ident} shares the script name {name} with another boss"
        );
        assert!(
            !script["properties"]
                .as_object()
                .is_some_and(|properties| properties.contains_key("kind")),
            "{ident} still selects itself out of a shared script with a kind number"
        );
    }
}

/// The original tells its bosses apart by side count and colour, and so does
/// this. Two bosses drawing the same polygon in the same colour would be the
/// shared hull back again under another name.
#[test]
fn every_boss_wears_its_own_shape() {
    let run = Run::open().expect("the project opens");
    let mut looks = HashSet::new();
    for ident in ROSTER {
        let json = prefab_json(&run, ident);
        let shape = component(&json, "sindri.shape");
        let kind = shape["kind"].as_str().expect("a shape kind").to_owned();
        let sides = shape["count"].as_f64().unwrap_or(0.0);
        let fill: Vec<i64> = shape["fill"]
            .as_array()
            .expect("a fill")
            .iter()
            .take(3)
            .map(|channel| (channel.as_f64().expect("a channel") * 100.0).round() as i64)
            .collect();
        assert!(
            looks.insert(format!("{kind}/{sides}/{fill:?}")),
            "{ident} draws a {sides}-sided {kind} in a colour another boss already wears"
        );
    }
}

/// The director has to be able to reach all twelve, or a boss nobody ever
/// fights is a boss that may as well not exist.
#[test]
fn the_director_can_send_every_boss() {
    let text = std::fs::read_to_string(orbital_baked::project().join("assets/orbital.scene"))
        .expect("the scene reads");
    let scene: serde_json::Value = serde_json::from_str(&text).expect("the scene parses");
    let director = scene["entities"]
        .as_array()
        .expect("entities")
        .iter()
        .find(|entity| {
            entity["components"]["sindri.script"]["source"]
                .as_str()
                .is_some_and(|source| source.ends_with("director.decay"))
        })
        .expect("the scene has a director");
    let properties = director["components"]["sindri.script"]["properties"]
        .as_object()
        .expect("director properties");

    for ident in ROSTER {
        let key = ident.replace('-', "_");
        let wanted = format!("prefabs/{ident}.prefab");
        assert_eq!(
            properties.get(&key).and_then(serde_json::Value::as_str),
            Some(wanted.as_str()),
            "the director cannot send the {ident}"
        );
    }
}
