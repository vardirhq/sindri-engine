//! What the canvas draws, as a player would see it: the edited world with the
//! project's stylesheets applied, so the HUD and menus look as they will when
//! played, and with the screen UI hidden while it is switched off.
//!
//! Built only when what it is built from changes (an edit, the selection, a
//! drag, the screen's size, the switch), not every frame: styling copies the
//! world.

use sindri_core::{EntityId, World};
use sindri_runtime::StylePreview;

use crate::Editing;

/// What a presented world was built for.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PresentKey {
    revision: u64,
    selected: Option<EntityId>,
    selected_revision: Option<u64>,
    additional: Vec<EntityId>,
    outline: u32,
    size: [u32; 2],
    screen_ui: bool,
}

impl Editing {
    /// The world to draw: [`Editing::drawn`] (the selection outlined), styled
    /// by `styles`, with the screen UI taken out while it is switched off.
    ///
    /// # Errors
    /// A stylesheet declaration that will not apply, named.
    pub fn presented<'a>(
        &'a mut self,
        world: &'a World,
        size: [f32; 2],
        outline: f32,
        styles: Option<&StylePreview>,
    ) -> Result<&'a World, String> {
        if styles.is_none() && self.screen_ui {
            return Ok(self.drawn(world, size, outline));
        }
        let key = PresentKey {
            revision: self.history.revision(),
            selected: self.selected,
            selected_revision: self.selected.and_then(|entity| world.revision(entity)),
            additional: self.additional.clone(),
            outline: outline.to_bits(),
            size: [size[0].to_bits(), size[1].to_bits()],
            screen_ui: self.screen_ui,
        };
        if self.presented.as_ref().map(|(built, _)| built) != Some(&key) {
            let drawn = self.drawn(world, size, outline).clone();
            let mut shown = match styles {
                Some(styles) => styles.present(&drawn, size)?,
                None => drawn,
            };
            if !self.screen_ui {
                without_screen_ui(&mut shown);
            }
            self.presented = Some((key, shown));
        }
        Ok(&self.presented.as_ref().expect("just built").1)
    }
}

/// Takes every screen UI element out of what is drawn: its UI components go,
/// and the world drawn under it stays.
fn without_screen_ui(world: &mut World) {
    let screen: Vec<EntityId> = world
        .entities()
        .filter(|(_, data)| {
            data.components
                .keys()
                .any(|name| name.starts_with("sindri.ui."))
        })
        .map(|(entity, _)| entity)
        .collect();
    for entity in screen {
        if let Some(data) = world.get_mut(entity) {
            data.components
                .retain(|name, _| !name.starts_with("sindri.ui."));
        }
    }
}
