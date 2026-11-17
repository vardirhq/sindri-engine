use std::{collections::BTreeSet, fs, path::Path};

use serde_json::Value;

#[derive(Default)]
pub(crate) struct ProjectIndex {
    pub(crate) entity_names: BTreeSet<String>,
    pub(crate) audio_assets: BTreeSet<String>,
}

impl ProjectIndex {
    pub(crate) fn scan(root: Option<&Path>) -> Self {
        let mut index = Self::default();
        if let Some(root) = root {
            index.scan_dir(root, root);
        }
        index
    }

    fn scan_dir(&mut self, root: &Path, directory: &Path) {
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if !matches!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some(".git" | "target" | "node_modules")
                ) {
                    self.scan_dir(root, &path);
                }
                continue;
            }

            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase);
            if matches!(extension.as_deref(), Some("wav" | "ogg" | "mp3" | "flac")) {
                self.audio_assets.insert(relative.clone());
            }
            // A prefab's entities are named in the prefab, and every instance
            // of it is an entity by that name in the running world.
            if !matches!(extension.as_deref(), Some("scene" | "prefab")) {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            collect_entity_names(&value, &mut self.entity_names);
        }
    }
}

fn collect_entity_names(scene: &Value, into: &mut BTreeSet<String>) {
    let Some(entities) = scene.get("entities").and_then(Value::as_array) else {
        return;
    };
    for entity in entities {
        if let Some(name) = entity.get("name").and_then(Value::as_str) {
            into.insert(name.to_owned());
        }
        // An instance renamed in the scene is found by its new name.
        let renamed = entity
            .pointer("/prefab/overrides")
            .and_then(Value::as_object)
            .into_iter()
            .flat_map(|overrides| overrides.values())
            .filter_map(|changes| changes.get("name").and_then(Value::as_str));
        into.extend(renamed.map(str::to_owned));
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectIndex;

    /// The same project gives the same answers in the same order, whatever
    /// order the file system lists it in: everything is kept sorted.
    #[test]
    fn the_index_is_the_same_whatever_order_files_are_listed_in() {
        let root = std::env::temp_dir().join(format!("decay-lsp-index-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("assets/audio")).unwrap();
        for name in ["zebra", "apple", "mango"] {
            std::fs::write(root.join(format!("assets/audio/{name}.wav")), b"").unwrap();
            std::fs::write(
                root.join(format!("assets/{name}.scene")),
                format!(r#"{{"entities":[{{"name":"{name}-thing"}}]}}"#),
            )
            .unwrap();
        }
        let first = ProjectIndex::scan(Some(&root));
        let again = ProjectIndex::scan(Some(&root));
        std::fs::remove_dir_all(&root).unwrap();
        let names: Vec<_> = first.entity_names.iter().cloned().collect();
        assert_eq!(names, ["apple-thing", "mango-thing", "zebra-thing"]);
        let audio: Vec<_> = first.audio_assets.iter().cloned().collect();
        assert_eq!(
            audio,
            [
                "assets/audio/apple.wav",
                "assets/audio/mango.wav",
                "assets/audio/zebra.wav"
            ]
        );
        assert_eq!(first.entity_names, again.entity_names);
        assert_eq!(first.audio_assets, again.audio_assets);
    }
}
