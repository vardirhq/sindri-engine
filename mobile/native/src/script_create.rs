//! Create a script file and attach it to an object without replacing existing work.

use std::fs::{self, OpenOptions};
use std::io::Write;

use crate::{Editing, OpenedScene};
use sindri_core::{CommandBuffer, WorldCommand};

fn write_new(path: &std::path::Path, source: &str) -> Result<(), String> {
    let temporary = path.with_extension("decay.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| error.to_string())?;
    let written = file
        .write_all(source.as_bytes())
        .and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = written {
        let _ = fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    let reserved = match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(file) => file,
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            return Err(error.to_string());
        }
    };
    drop(reserved);
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        let _ = fs::remove_file(path);
        return Err(error.to_string());
    }
    Ok(())
}

impl Editing {
    /// Create and attach a fresh behavior. Undo detaches it but retains the file.
    pub fn create_script(&mut self, opened: &mut OpenedScene) -> Result<String, String> {
        let entity = self.selected.ok_or("Nothing is selected")?;
        let data = opened.world.get(entity).ok_or("Object no longer exists")?;
        if data.components.contains_key("sindri.script") {
            return Err("This object already has a script".to_owned());
        }
        let assets = sindri_runtime::project::assets_root(&opened.project);
        let scripts = assets.join("scripts");
        fs::create_dir_all(&scripts).map_err(|error| error.to_string())?;
        let canonical_assets = assets.canonicalize().map_err(|error| error.to_string())?;
        let canonical_scripts = scripts.canonicalize().map_err(|error| error.to_string())?;
        if !canonical_scripts.starts_with(&canonical_assets) {
            return Err("Scripts folder must be inside the project".to_owned());
        }
        let sources = sindri_runtime::project::files_under(&assets, ".decay");
        let mut number = 1u32;
        loop {
            let name = format!("MobileBehavior{number}");
            let source = format!("script {name} {{\n    fn update(dt: number) {{\n    }}\n}}\n");
            let used = sources.values().any(|bytes| {
                String::from_utf8_lossy(bytes)
                    .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                    .any(|word| word == name)
            });
            let path = scripts.join(format!("behavior-{number}.decay"));
            if used || path.exists() || path.with_extension("decay.tmp").exists() {
                number = number.checked_add(1).ok_or("No script names remain")?;
                continue;
            }
            let checked =
                sindri_decay::check_source_in(&source, opened.assets.scripts.environment());
            if !checked.compiles() {
                return Err("The new script did not pass the project checker".to_owned());
            }
            let asset = format!("scripts/behavior-{number}.decay");
            let payload = serde_json::json!({"source": asset, "script": name, "properties": {}});
            opened
                .extractor
                .components()
                .validate_payload("sindri.script", &payload)
                .map_err(|error| error.to_string())?;
            write_new(&path, &source)?;
            let mut commands = CommandBuffer::new();
            commands.push(WorldCommand::SetComponent {
                entity,
                type_name: "sindri.script".to_owned(),
                payload,
            });
            self.history.break_merge_run();
            if let Err(error) = self.history.apply(
                commands.into_transaction("Attach script"),
                &mut opened.world,
            ) {
                let _ = fs::remove_file(&path);
                return Err(error.to_string());
            }
            opened.assets.scripts.insert(asset.clone(), source);
            return Ok(asset);
        }
    }
}
