//! Pointing the scene's sprites and tilemaps at a saved map.
//!
//! The map only does anything once something names it. Everything in the
//! open scene that draws from the studio's picture is found by what its
//! texture reference resolves to, and pointed at the map in one undoable
//! step, through the same `SetComponent` commands an inspector edit makes.

use std::path::{Path, PathBuf};

use serde_json::Value;
use sindri_core::{CommandBuffer, EntityId, SceneComponent, SpriteRef, WorldCommand};
use sindri_scene::{SpriteComponent, TilemapComponent};

use crate::normal_studio::relative_to;

use super::super::EditorApp;

/// The file a texture reference names, under the folder the scene's
/// references resolve against.
fn referenced_file(reference: &str, root: &Path) -> Option<PathBuf> {
    let sprite = SpriteRef::parse(reference).ok()?;
    Some(root.join(sprite.texture()))
}

/// Whether two paths are the same file, judged as the file system sees it
/// when both exist, and by their spelling otherwise.
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// Whether a component payload draws from `source`.
fn draws_from(type_name: &str, payload: &Value, source: &Path, root: &Path) -> bool {
    let names = |reference: &Value| {
        reference
            .as_str()
            .and_then(|reference| referenced_file(reference, root))
            .is_some_and(|file| same_file(&file, source))
    };
    if type_name == SpriteComponent::TYPE_NAME {
        payload.get("texture").is_some_and(names)
    } else if type_name == TilemapComponent::TYPE_NAME {
        // A map's palette names sprites of its texture by name alone, or by
        // full references to any texture.
        payload.get("texture").is_some_and(names)
            || payload
                .get("palette")
                .and_then(Value::as_array)
                .is_some_and(|palette| palette.iter().any(names))
    } else {
        false
    }
}

impl EditorApp {
    /// The folder the open scene's texture references resolve against.
    fn reference_root(&self) -> Option<PathBuf> {
        self.file
            .path()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
    }

    /// Every sprite and tilemap in the scene that draws the studio's picture,
    /// with the component that does.
    pub(super) fn normal_map_users(&self) -> Vec<(EntityId, &'static str)> {
        let (Some(studio), Some(root)) = (self.normal_studio.as_ref(), self.reference_root())
        else {
            return Vec::new();
        };
        let mut users = Vec::new();
        for (entity, data) in self.world.entities() {
            for type_name in [SpriteComponent::TYPE_NAME, TilemapComponent::TYPE_NAME] {
                if data
                    .components
                    .get(type_name)
                    .is_some_and(|payload| draws_from(type_name, payload, studio.source(), &root))
                {
                    users.push((entity, type_name));
                }
            }
        }
        users
    }

    /// How many of the scene's sprites and tilemaps draw the picture, and
    /// how many of those already name the saved map.
    pub(super) fn normal_map_reach(&self) -> (usize, usize) {
        let users = self.normal_map_users();
        let reference = self
            .normal_studio
            .as_ref()
            .zip(self.reference_root())
            .and_then(|(studio, root)| relative_to(studio.map_path(), &root));
        let wearing = users
            .iter()
            .filter(|(entity, type_name)| {
                self.world
                    .get(*entity)
                    .and_then(|data| data.components.get(*type_name))
                    .and_then(|payload| payload.get("normal_map"))
                    .and_then(Value::as_str)
                    .is_some_and(|named| Some(named) == reference.as_deref())
            })
            .count();
        (users.len(), wearing)
    }

    /// Names the saved map as the `normal_map` of everything that draws the
    /// picture, as one step in the history.
    pub(super) fn use_normal_map_on_sprites(&mut self) {
        let (Some(studio), Some(root)) = (self.normal_studio.as_ref(), self.reference_root())
        else {
            return;
        };
        let Some(reference) = relative_to(studio.map_path(), &root) else {
            self.console.error(format!(
                "{} is outside the folder this scene's textures are named from",
                studio.map_path().display()
            ));
            return;
        };
        let mut commands = CommandBuffer::new();
        let mut count = 0_usize;
        for (entity, type_name) in self.normal_map_users() {
            let Some(mut payload) = self
                .world
                .get(entity)
                .and_then(|data| data.components.get(type_name))
                .cloned()
            else {
                continue;
            };
            if payload.get("normal_map").and_then(Value::as_str) == Some(reference.as_str()) {
                continue;
            }
            payload["normal_map"] = Value::String(reference.clone());
            commands.push(WorldCommand::SetComponent {
                entity,
                type_name: type_name.to_owned(),
                payload,
            });
            count += 1;
        }
        if count == 0 {
            self.console.info(format!(
                "Everything that draws this picture already uses {reference}"
            ));
            return;
        }
        let label = format!("Use normal map {reference}");
        match self.apply_edit(commands.into_transaction(&label)) {
            Ok(()) => self.console.info(format!("{label} on {count}")),
            Err(error) => self.console.error(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn sprites_and_tilemaps_are_found_by_the_file_they_draw() {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path();
        std::fs::create_dir(root.join("textures")).unwrap();
        let source = root.join("textures/tiles.png");
        std::fs::write(&source, b"").unwrap();
        let sprite = SpriteComponent::TYPE_NAME;
        let tilemap = TilemapComponent::TYPE_NAME;
        let finds = |type_name, payload| draws_from(type_name, &payload, &source, root);
        assert!(finds(sprite, json!({ "texture": "textures/tiles.png" })));
        assert!(finds(
            sprite,
            json!({ "texture": "textures/tiles.png#grass-0" })
        ));
        assert!(!finds(sprite, json!({ "texture": "textures/hero.png" })));
        assert!(finds(
            tilemap,
            json!({ "texture": "textures/tiles.png", "palette": ["grass"] })
        ));
        assert!(finds(
            tilemap,
            json!({ "palette": ["textures/tiles.png#grass"] })
        ));
        assert!(!finds(tilemap, json!({ "palette": ["grass"] })));
        assert!(!finds(
            "sindri.light2d",
            json!({ "texture": "textures/tiles.png" })
        ));
    }
}
