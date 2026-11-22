//! Which finger is holding which button.
//!
//! Every press that lands on something pressable holds it, not only the first
//! press on the screen. A phone held sideways has a thumb on a stick and the
//! other thumb on the buttons, and the thumb on the stick is down first: a
//! screen that only listened to the oldest press could not be pressed at all
//! while the player was steering, and a game had to read the second finger by
//! hand to fake a jump button.
//!
//! A press holds what it *landed* on. Sliding off lets go of the button without
//! handing it to whatever the finger moved onto, and coming back takes it up
//! again, the way a mouse button held over a desktop button behaves. Letting
//! go over it clicks it.

use sindri_core::{EntityId, PressId, PressPhase, Presses, SceneComponent, World};

use super::{ScreenExtent, ScreenUi, UiSliderComponent};

/// One press on a button: what it landed on and where it has got to.
#[derive(Clone, Copy, Debug)]
pub(super) struct Hold {
    entity: EntityId,
    /// Whether the press is over what it landed on now.
    over: bool,
    /// Where the press is, in overlay units.
    at: Option<[f32; 2]>,
    /// How far it moved this step, in overlay units.
    moved: [f32; 2],
}

impl ScreenUi {
    /// Follows every press over the screen's buttons for one step.
    pub(super) fn read_holds(&mut self, world: &World, extent: ScreenExtent, presses: &Presses) {
        let scrolling = self
            .scroll_drag
            .filter(|(_, _, _, _, dragging)| *dragging)
            .map(|(_, id, _, _, _)| id);
        for press in presses.began() {
            if press.phase() == PressPhase::Cancelled {
                continue;
            }
            let Some(entity) = extent
                .pointer(press.origin())
                .and_then(|point| self.topmost_at(point))
            else {
                continue;
            };
            // A slider follows its finger itself, as a drag rather than a hold.
            if Self::is_slider(world, entity) {
                continue;
            }
            self.holds.insert(
                press.id(),
                Hold {
                    entity,
                    over: false,
                    at: extent.pointer(press.origin()),
                    moved: [0.0, 0.0],
                },
            );
        }
        let ids: Vec<PressId> = self.holds.keys().copied().collect();
        for id in ids {
            let Some(&hold) = self.holds.get(&id) else {
                continue;
            };
            let entity = hold.entity;
            let Some(press) = presses.get(id) else {
                self.holds.remove(&id);
                continue;
            };
            // A press that turned into a list being dragged is the list's now.
            if scrolling == Some(id) || press.phase() == PressPhase::Cancelled {
                self.holds.remove(&id);
                continue;
            }
            let at = extent.pointer(press.position());
            let over = at.and_then(|point| self.topmost_at(point)) == Some(entity);
            let moved = match (hold.at, at) {
                (Some(from), Some(to)) => [to[0] - from[0], to[1] - from[1]],
                _ => [0.0, 0.0],
            };
            if press.phase() == PressPhase::Ended {
                // Gone at once, so the step a button is clicked is not also a
                // step it reads as held.
                self.holds.remove(&id);
                if over && self.clicked.is_none() {
                    self.clicked = Some(entity);
                }
            } else {
                self.holds.insert(
                    id,
                    Hold {
                        entity,
                        over,
                        at: at.or(hold.at),
                        moved,
                    },
                );
            }
        }
    }

    /// Whether any press is on `entity`, having landed on it.
    pub(super) fn is_held_by_press(&self, entity: EntityId) -> bool {
        self.holds
            .values()
            .any(|hold| hold.entity == entity && hold.over)
    }

    /// How far the press that landed on `entity` moved this step, in overlay
    /// units, whether or not it is still over it: zero when nothing is on it.
    ///
    /// What a touch pad reads. A look pad is an element a thumb drags across,
    /// and the thumb wandering off its edge is still the same look.
    #[must_use]
    pub fn drag(&self, entity: EntityId) -> [f32; 2] {
        self.holds
            .values()
            .find(|hold| hold.entity == entity)
            .map_or([0.0, 0.0], |hold| hold.moved)
    }

    /// The element the oldest press holding something is over, for `:active`.
    pub(super) fn held_element(&self) -> Option<EntityId> {
        self.holds
            .values()
            .find(|hold| hold.over)
            .map(|hold| hold.entity)
    }

    /// Whether the press `id` landed on something on screen that took it.
    ///
    /// What a host asks before it lets the same finger drive a control of its
    /// own, such as a stick: a thumb on a button is pressing the button.
    #[must_use]
    pub fn holds_press(&self, id: PressId) -> bool {
        self.holds.contains_key(&id)
            || self.slider_drag.is_some_and(|(_, drag)| drag == id)
            || self.scroll_drag.is_some_and(|(_, drag, ..)| drag == id)
    }

    pub(super) fn is_slider(world: &World, entity: EntityId) -> bool {
        world
            .get(entity)
            .and_then(|data| data.components.get(UiSliderComponent::TYPE_NAME))
            .and_then(|payload| serde_json::from_value::<UiSliderComponent>(payload.clone()).ok())
            .is_some_and(|slider| !slider.disabled)
    }
}
