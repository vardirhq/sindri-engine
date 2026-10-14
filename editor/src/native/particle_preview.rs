//! Playing the selected particle emitter in the Scene view without Play.
//!
//! An emitter's numbers are hard to judge from the inspector, so the one that
//! is selected runs in the editor's still pool while nothing is playing, and
//! only that one: a scene full of chimneys would otherwise smoke the whole
//! time anyone edited it. Selecting something else starts from nothing.

use std::time::{Duration, Instant};

use sindri_core::{ComponentSchemaRegistry, EntityId, SceneComponent, World};
use sindri_scene::{Effects2d, ParticlesComponent};

use crate::selection::Selection;

/// The longest step a preview takes, so a pause in redrawing does not dump a
/// second's particles at once.
const LONGEST_STEP: Duration = Duration::from_millis(100);

#[derive(Default)]
pub(super) struct ParticlePreview {
    last: Option<Instant>,
    previewing: Vec<EntityId>,
}

impl ParticlePreview {
    /// Steps the selected emitters. True while there is anything to watch.
    pub(super) fn step(
        &mut self,
        effects: &mut Effects2d,
        world: &World,
        components: &ComponentSchemaRegistry,
        selection: &Selection,
    ) -> bool {
        let emitters: Vec<EntityId> = selection
            .all()
            .iter()
            .copied()
            .filter(|entity| {
                world
                    .get(*entity)
                    .is_some_and(|data| data.components.contains_key(ParticlesComponent::TYPE_NAME))
            })
            .collect();
        if emitters != self.previewing {
            effects.clear();
            self.previewing.clone_from(&emitters);
            self.last = None;
        }
        if emitters.is_empty() && effects.live() == 0 {
            self.last = None;
            return false;
        }
        let now = Instant::now();
        let step = self
            .last
            .map_or(Duration::ZERO, |last| (now - last).min(LONGEST_STEP));
        self.last = Some(now);
        effects.advance(step);
        // A broken emitter is the inspector's to show; the preview just
        // throws nothing for it.
        let _ = effects.emit_where(world, components, step, |entity| emitters.contains(&entity));
        true
    }
}
