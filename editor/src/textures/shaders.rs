//! The sprite shaders a scene names, loaded the way its textures are.
//!
//! A shader arrives as text and is checked as it is bound, so a source that
//! does not compile is a console line naming the file and the line, and the
//! sprites using it draw with the built-in look until it is fixed. Saving the
//! file reloads it, which is what makes writing a shader in the editor's
//! company an edit-and-look loop rather than a restart.

use std::collections::BTreeSet;
use std::path::Path;

use sindri_assets::{AssetLoadOutcome, AssetLoader, FileSystemAssetSource, TextAssetDecoder};
use sindri_core::{AssetId, World};
use sindri_scene::referenced_sprite_shaders;

use super::{QUEUE, SceneTextures, TextureNote, manifest_beside};

/// A loader for the shaders under `root`.
pub(super) fn loader(root: &Path) -> Option<AssetLoader<TextAssetDecoder>> {
    let loader =
        AssetLoader::new(FileSystemAssetSource::new(root), QUEUE, TextAssetDecoder).ok()?;
    Some(match manifest_beside(root) {
        Some(manifest) => loader.with_manifest(manifest),
        None => loader,
    })
}

impl SceneTextures {
    /// Asks for every shader the world and its prefabs name, and lets go of
    /// the rest.
    pub(super) fn request_shaders(
        &mut self,
        world: &World,
    ) -> (BTreeSet<AssetId>, Vec<TextureNote>) {
        let mut references = referenced_sprite_shaders(world);
        references.extend(referenced_sprite_shaders(&self.prefab_world));
        let wanted: BTreeSet<AssetId> = references
            .iter()
            .filter_map(|reference| AssetId::new(reference.clone()).ok())
            .collect();
        let mut notes = Vec::new();
        let Some(shaders) = &mut self.shaders else {
            for reference in references {
                notes.push(TextureNote::Failed(format!(
                    "{reference}: the scene has no directory to load shaders from"
                )));
            }
            return (wanted, notes);
        };
        for released in shaders.retain(&wanted) {
            self.bindings.shaders_mut().unbind(released.as_str());
        }
        for id in &wanted {
            if self.bindings.shaders().get(id.as_str()).is_some() {
                continue;
            }
            if let Err(error) = shaders.request(id.clone()) {
                notes.push(TextureNote::Failed(format!("{id}: {error}")));
            }
        }
        for reference in references {
            if AssetId::new(reference.clone()).is_err() {
                notes.push(TextureNote::Failed(format!(
                    "{reference}: not a loadable shader asset reference"
                )));
            }
        }
        (wanted, notes)
    }

    /// Takes delivery of whatever shaders arrived, checking each as it binds.
    pub(super) fn poll_shaders(&mut self) -> Vec<TextureNote> {
        let mut notes = Vec::new();
        let Some(shaders) = &mut self.shaders else {
            return notes;
        };
        for outcome in shaders.poll() {
            match outcome {
                AssetLoadOutcome::Ready(id) => {
                    let Some(source) = shaders.get(&id) else {
                        continue;
                    };
                    let again = self.bindings.shaders().get(id.as_str()).is_some();
                    match self.bindings.shaders_mut().bind(id.as_str(), source) {
                        Ok(()) => notes.push(if again {
                            TextureNote::Reloaded(format!("Reloaded {id}"))
                        } else {
                            TextureNote::Loaded(format!("Loaded {id}"))
                        }),
                        // The error names the file and its line already.
                        Err(error) => notes.push(TextureNote::Failed(error.to_string())),
                    }
                    // Watched whether or not it compiled: a broken shader is
                    // exactly the file whose next save has to be noticed.
                    if let Some(watch) = self.watch.as_mut() {
                        watch.watch(&id);
                    }
                }
                AssetLoadOutcome::Failed(error) => notes.push(TextureNote::Failed(format!(
                    "{}: {}",
                    error.id(),
                    error.message()
                ))),
            }
        }
        notes
    }
}
