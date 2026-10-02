//! The project's scenes as a board: one card for each scene the project
//! carries, and the doors between them.
//!
//! A project with more than one scene used to be a list in `sindri.toml` that
//! nothing in the editor showed. Which scenes a build carried, and which scene
//! led to which, were both things a person had to work out by reading the
//! manifest and grepping scripts for `Scene.go` — and the first anybody heard
//! of a door to a scene the project did not carry was a build where the door
//! went nowhere.
//!
//! So the board reads both. A card is a scene the manifest declares. A link is
//! a `Scene.go("…")` with a literal name in a script one of that scene's
//! entities runs, or in a prefab it places. A name that matches no card is a
//! stray, and is shown on the card it leaves from, because that is a door that
//! opens onto nothing in a build.
//!
//! Kept apart from the drawing so that "what does this project's board say" is
//! a question a test can ask with a temporary folder and no window.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::Project;

/// How many prefabs deep a scene is followed for the scripts it runs.
///
/// A prefab can place another; a project that nested them in a cycle would
/// otherwise be read for ever. Deeper than any real nesting.
const PREFAB_DEPTH: usize = 8;

/// One scene the project carries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SceneCard {
    pub path: PathBuf,
    /// The scene's file name, which is how a script names it: `Scene.go`
    /// takes this, and an export names the scene by it.
    pub name: String,
    /// Whether this is the scene the project opens on.
    pub main: bool,
    /// Declared and not on disk.
    pub missing: bool,
    /// What the scene's file says about itself, when it says it is
    /// unreadable: a card whose scene does not parse should say so rather
    /// than show an empty scene.
    pub problem: Option<String>,
    /// Scenes its scripts go to that the project does not carry.
    pub strays: Vec<String>,
    /// How many entities it has, as a hint of what is in it.
    pub entities: usize,
}

/// A door from one card's scene to another's.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SceneLink {
    pub from: usize,
    pub to: usize,
}

/// A project's scenes and the doors between them, as read from disk.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SceneBoard {
    pub cards: Vec<SceneCard>,
    pub links: Vec<SceneLink>,
}

impl SceneBoard {
    /// Reads the board for a project: its manifest's scenes, and every script
    /// they run.
    pub fn read(project: &Project) -> Self {
        let main = project.declared_main_scene();
        let mut cards = Vec::new();
        let mut leaving = Vec::new();
        for path in project.scenes() {
            let name = path.file_name().map_or_else(
                || path.display().to_string(),
                |name| name.to_string_lossy().into_owned(),
            );
            let missing = !path.is_file();
            let (doors, entities, problem) = if missing {
                (Vec::new(), 0, None)
            } else {
                read_scene(&path)
            };
            cards.push(SceneCard {
                main: main.as_ref() == Some(&path),
                path,
                name,
                missing,
                problem,
                strays: Vec::new(),
                entities,
            });
            leaving.push(doors);
        }
        let mut links = Vec::new();
        for (from, doors) in leaving.into_iter().enumerate() {
            for door in doors {
                match cards.iter().position(|card| card.name == door) {
                    Some(to) => {
                        if !links.contains(&SceneLink { from, to }) {
                            links.push(SceneLink { from, to });
                        }
                    }
                    None => cards[from].strays.push(door),
                }
            }
        }
        Self { cards, links }
    }

    /// The cards a card's scene leads to.
    pub fn leads_to(&self, card: usize) -> impl Iterator<Item = usize> + '_ {
        self.links
            .iter()
            .filter(move |link| link.from == card)
            .map(|link| link.to)
    }

    /// The cards that lead to a card's scene.
    pub fn reached_from(&self, card: usize) -> impl Iterator<Item = usize> + '_ {
        self.links
            .iter()
            .filter(move |link| link.to == card)
            .map(|link| link.from)
    }
}

/// The scene names a scene's scripts go to, how many entities it has, and
/// why it could not be read if it could not.
fn read_scene(path: &Path) -> (Vec<String>, usize, Option<String>) {
    let document = match std::fs::read_to_string(path)
        .map_err(|error| error.to_string())
        .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|error| error.to_string()))
    {
        Ok(document) => document,
        Err(error) => return (Vec::new(), 0, Some(error)),
    };
    let entities = document
        .get("entities")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    // Script and prefab references resolve against the scene's own
    // directory, which is where the asset loader is rooted.
    let assets = path.parent().unwrap_or(Path::new(""));
    let mut scripts = BTreeSet::new();
    let mut prefabs = BTreeSet::new();
    let mut read = BTreeSet::new();
    sources(&document, &mut scripts, &mut prefabs);
    for _ in 0..PREFAB_DEPTH {
        let unread: Vec<String> = prefabs.difference(&read).cloned().collect();
        if unread.is_empty() {
            break;
        }
        for prefab in unread {
            if let Some(placed) = std::fs::read_to_string(assets.join(&prefab))
                .ok()
                .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            {
                sources(&placed, &mut scripts, &mut prefabs);
            }
            read.insert(prefab);
        }
    }
    let mut doors: Vec<String> = Vec::new();
    for script in scripts {
        let Ok(source) = std::fs::read_to_string(assets.join(&script)) else {
            continue;
        };
        for door in sindri_decay::scene_links(&source) {
            if !doors.contains(&door) {
                doors.push(door);
            }
        }
    }
    (doors, entities, None)
}

/// Every script and prefab a document names, wherever in it they are: an
/// entity's script, a placed prefab, or an override inside one.
fn sources(value: &Value, scripts: &mut BTreeSet<String>, prefabs: &mut BTreeSet<String>) {
    match value {
        Value::Object(fields) => {
            if let Some(Value::String(source)) = fields.get("source") {
                match Path::new(source).extension().and_then(|e| e.to_str()) {
                    Some("decay") => {
                        scripts.insert(source.clone());
                    }
                    Some("prefab") => {
                        prefabs.insert(source.clone());
                    }
                    _ => {}
                }
            }
            for field in fields.values() {
                sources(field, scripts, prefabs);
            }
        }
        Value::Array(items) => {
            for item in items {
                sources(item, scripts, prefabs);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{SceneBoard, SceneLink};
    use crate::project::Project;

    fn write(root: &std::path::Path, path: &str, text: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("folders");
        std::fs::write(path, text).expect("a file");
    }

    fn scene(script: &str) -> String {
        format!(
            r#"{{"entities":[{{"id":"door","components":{{"sindri.script":{{"source":"{script}","script":"Door"}}}}}}]}}"#
        )
    }

    /// The title's script goes to the level, a prefab the level places goes
    /// back to the title, and a door to a scene the project does not carry is
    /// a stray on the card it leaves from.
    #[test]
    fn a_board_draws_the_doors_scripts_open_and_the_ones_that_go_nowhere() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let root = directory.path();
        write(
            root,
            "sindri.toml",
            "format_version = 1\n[project]\nname = \"Doors\"\nmain_scene = \"assets/title.scene\"\nscenes = [\"assets/level.scene\", \"assets/gone.scene\"]\n",
        );
        write(root, "assets/title.scene", &scene("scripts/title.decay"));
        write(
            root,
            "assets/scripts/title.decay",
            "script Door { fn on_press() { Scene.go(\"level.scene\"); Scene.go(\"shop.scene\"); } }",
        );
        write(
            root,
            "assets/level.scene",
            r#"{"entities":[{"id":"exit","prefab":{"source":"prefabs/exit.prefab"}}]}"#,
        );
        write(
            root,
            "assets/prefabs/exit.prefab",
            &scene("scripts/exit.decay"),
        );
        write(
            root,
            "assets/scripts/exit.decay",
            "script Door { fn on_press() { Scene.go(\"title.scene\"); } }",
        );

        let board = SceneBoard::read(&Project::open(root).expect("the project opens"));
        let names: Vec<&str> = board.cards.iter().map(|card| card.name.as_str()).collect();
        assert_eq!(names, ["title.scene", "level.scene", "gone.scene"]);
        assert!(board.cards[0].main && !board.cards[1].main);
        assert!(board.cards[2].missing);
        assert_eq!(
            board.links,
            [SceneLink { from: 0, to: 1 }, SceneLink { from: 1, to: 0 }]
        );
        assert_eq!(board.cards[0].strays, ["shop.scene"]);
        assert_eq!(board.leads_to(0).collect::<Vec<_>>(), [1]);
        assert_eq!(board.reached_from(0).collect::<Vec<_>>(), [1]);
    }
}

/// The projects this repository ships, read as the board reads them.
#[cfg(test)]
mod shipped {
    use std::path::PathBuf;

    use super::{SceneBoard, SceneLink};
    use crate::project::Project;

    fn shipped() -> Vec<PathBuf> {
        let repository = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
        let mut projects = Vec::new();
        for folder in ["games", "examples"] {
            for entry in std::fs::read_dir(repository.join(folder)).expect("the folder reads") {
                let path = entry.expect("an entry").path();
                if path.join("sindri.toml").is_file() {
                    projects.push(path);
                }
            }
        }
        projects.sort();
        projects
    }

    /// Orbital's title goes to the combat lab and the lab's button comes back:
    /// two cards with a door each way.
    #[test]
    fn orbital_has_a_door_each_way_between_its_two_scenes() {
        let root = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../games/orbital-baked"
        ));
        let board = SceneBoard::read(&Project::open(&root).expect("Orbital opens"));
        let names: Vec<&str> = board.cards.iter().map(|card| card.name.as_str()).collect();
        assert_eq!(names, ["orbital.scene", "combat-lab.scene"]);
        assert_eq!(
            board.links,
            [SceneLink { from: 0, to: 1 }, SceneLink { from: 1, to: 0 }]
        );
    }

    /// No shipped project declares a scene it does not have, or has a script
    /// go to a scene it does not carry: either is a build with a door that
    /// opens onto nothing.
    #[test]
    fn every_shipped_board_is_whole() {
        let projects = shipped();
        assert!(projects.len() > 5, "{projects:?}");
        for root in projects {
            let board = SceneBoard::read(&Project::open(&root).expect("the project opens"));
            assert!(!board.cards.is_empty(), "{} has no scenes", root.display());
            for card in &board.cards {
                assert!(
                    !card.missing && card.problem.is_none() && card.strays.is_empty(),
                    "{}: {card:?}",
                    root.display()
                );
            }
        }
    }
}
