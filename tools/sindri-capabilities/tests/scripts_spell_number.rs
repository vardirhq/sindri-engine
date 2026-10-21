//! Decay's numeric type is `number`; `f32`, its first spelling, still compiles
//! and always will. The repository's scripts are what people and models copy,
//! and the knowledge pack ships the examples whole, so one script in the old
//! spelling teaches it again. This keeps every checked-in script on the new one.

use std::fs;
use std::path::{Path, PathBuf};

use decay_semantic::OLD_NUMBER;
use sindri_capabilities::repository_root;

#[test]
fn no_script_spells_the_numeric_type_f32() {
    let root = repository_root();
    let mut scripts = Vec::new();
    collect_scripts(root, &mut scripts);
    assert!(!scripts.is_empty(), "the repository has Decay scripts");

    let mut found = Vec::new();
    for script in &scripts {
        let source = fs::read_to_string(script).expect("a script can be read");
        for (index, line) in source.lines().enumerate() {
            if names_old_number(line) {
                let path = script.strip_prefix(root).unwrap_or(script);
                found.push(format!("{}:{}: {}", path.display(), index + 1, line.trim()));
            }
        }
    }

    assert!(
        found.is_empty(),
        "write `number`, not `{OLD_NUMBER}`:\n{}",
        found.join("\n")
    );
}

fn collect_scripts(directory: &Path, scripts: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(directory).expect("a directory can be listed");
    for entry in entries {
        let path = entry.expect("a directory entry can be read").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if !matches!(name, "target" | "node_modules") && !name.starts_with('.') {
                collect_scripts(&path, scripts);
            }
        } else if path
            .extension()
            .is_some_and(|extension| extension == "decay")
        {
            scripts.push(path);
        }
    }
}

/// Whether `OLD_NUMBER` appears as a whole identifier, not inside one such as
/// `as_f32`.
fn names_old_number(line: &str) -> bool {
    let is_identifier = |c: char| c.is_ascii_alphanumeric() || c == '_';
    line.match_indices(OLD_NUMBER).any(|(start, _)| {
        let before = line[..start].chars().next_back();
        let after = line[start + OLD_NUMBER.len()..].chars().next();
        !before.is_some_and(is_identifier) && !after.is_some_and(is_identifier)
    })
}
