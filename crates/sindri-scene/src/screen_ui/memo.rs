//! What a screen layout was worked out from, so an unchanged screen is not
//! worked out again.
//!
//! A game without a stylesheet lays its screen out every fixed step, because
//! that is the layout the step's clicks are hit-tested against. Most steps
//! change nothing on the screen, and laying it out again was a sixth of the
//! platformer's step. A layout is a function of the UI entities, the entities
//! above them, the screen and the measured text, so it is kept with each
//! entity's revision ([`World::revision`]) and done again only when one of
//! those moved.

use sindri_core::{EntityId, World};

use super::{ScreenExtent, UiTextSizes};

/// Every input to one layout.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct LayoutInputs {
    entities: Vec<(EntityId, u64)>,
    extent: ScreenExtent,
    text: UiTextSizes,
}

impl LayoutInputs {
    pub(super) fn of(world: &World, extent: ScreenExtent, text: &UiTextSizes) -> Self {
        let mut entities = Vec::new();
        for (entity, data) in world.entities() {
            if !data
                .components
                .keys()
                .any(|name| name.starts_with("sindri.ui."))
            {
                continue;
            }
            // The element and everything above it: a parent moved, sized or
            // switched off moves, sizes or hides what is under it.
            let mut current = Some(entity);
            while let Some(at) = current {
                entities.push((at, world.revision(at).unwrap_or_default()));
                current = world.get(at).and_then(|data| data.parent);
            }
        }
        entities.sort_unstable();
        entities.dedup();
        Self {
            entities,
            extent,
            text: text.clone(),
        }
    }
}
