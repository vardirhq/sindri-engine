//! The models a scene draws, loaded the way its textures are.
//!
//! A model is decoded off the frame loop by the same asset queue, prepared
//! once into the renderer's shared resource and bound to the scene's
//! extractor, which every entity naming it then draws from. Until it arrives,
//! or if it will not, the extractor draws the rest of the scene around it and
//! says which entity is waiting (`SceneExtractor::tolerate_invalid_components`).

use std::collections::BTreeSet;

use sindri_assets::AssetLoadOutcome;
use sindri_core::{AssetId, World};
use sindri_scene::{SceneExtractor, referenced_models};

use super::{SceneTextures, TextureNote};

impl SceneTextures {
    /// Asks for every model the world and its prefabs name, and lets go of
    /// the rest.
    pub(super) fn request_models(
        &mut self,
        world: &World,
    ) -> (BTreeSet<AssetId>, Vec<TextureNote>) {
        let mut notes = Vec::new();
        let mut references = referenced_models(world);
        references.extend(referenced_models(&self.prefab_world));
        let wanted: BTreeSet<AssetId> = references
            .iter()
            .filter_map(|reference| AssetId::new(reference.clone()).ok())
            .collect();
        let Some(models) = &mut self.models else {
            for reference in references {
                notes.push(TextureNote::Failed(format!(
                    "{reference}: the scene has no directory to load models from"
                )));
            }
            return (wanted, notes);
        };
        self.released_models.extend(models.retain(&wanted));
        for id in &wanted {
            if self.bound_models.contains(id) {
                continue;
            }
            if let Err(error) = models.request(id.clone()) {
                notes.push(TextureNote::Failed(format!("{id}: {error}")));
            }
        }
        (wanted, notes)
    }

    /// Prepares whatever models arrived and binds them to each of `scenes`, and
    /// unbinds the ones no longer named.
    pub fn poll_models(&mut self, scenes: &mut [&mut SceneExtractor]) -> Vec<TextureNote> {
        let mut notes = Vec::new();
        for released in std::mem::take(&mut self.released_models) {
            for scene in scenes.iter_mut() {
                scene.unbind_model(&released);
            }
            self.bound_models.remove(&released);
        }
        let Some(models) = &mut self.models else {
            return notes;
        };
        for outcome in models.poll() {
            match outcome {
                AssetLoadOutcome::Ready(id) => {
                    let Some(asset) = models.get(&id).cloned() else {
                        continue;
                    };
                    let node_count = asset.nodes.len();
                    match sindri::model::prepare(asset) {
                        Ok(prepared) => {
                            for warning in prepared.warnings {
                                notes.push(TextureNote::Failed(format!("{id}: {warning}")));
                            }
                            let mut again = None;
                            for scene in scenes.iter_mut() {
                                again = scene.bind_model(id.clone(), prepared.resource.clone());
                            }
                            self.bound_models.insert(id.clone());
                            let message = format!("{id} ({node_count} nodes)");
                            notes.push(if again.is_some() {
                                TextureNote::Reloaded(format!("Reloaded {message}"))
                            } else {
                                TextureNote::Loaded(format!("Loaded {message}"))
                            });
                            if let Some(watch) = self.watch.as_mut() {
                                watch.watch(&id);
                            }
                        }
                        Err(error) => notes.push(TextureNote::Failed(format!("{id}: {error}"))),
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

    /// Unbinds every model this scene bound, before another scene's
    /// textures replace these.
    pub fn release_models(&mut self, scenes: &mut [&mut SceneExtractor]) {
        for id in std::mem::take(&mut self.bound_models) {
            for scene in scenes.iter_mut() {
                scene.unbind_model(&id);
            }
        }
    }
}
