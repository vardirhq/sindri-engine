//! Shared widget interaction, independent of the window and input backend.
use super::navigation::Toward;
use super::{ScreenUi, UiInput, UiScrollComponent, UiTextInputComponent, UiToggleComponent};
use sindri_core::{EntityId, SceneComponent, World};

impl ScreenUi {
    #[must_use]
    pub const fn focused(&self) -> Option<EntityId> {
        self.focused
    }
    #[must_use]
    pub fn changed(&self, entity: EntityId) -> bool {
        self.changed.contains(&entity)
    }
    #[must_use]
    pub fn submitted(&self, entity: EntityId) -> bool {
        self.submitted == Some(entity)
    }

    /// Call after reading pointer presses and before gameplay scripts.
    pub fn read_controls(&mut self, world: &mut World, input: &UiInput) {
        // Changes a pointer made this step (a scroll drag) were counted by
        // the pointer pass, which already started the step's set.
        if !std::mem::take(&mut self.presses_read) {
            self.changed.clear();
            self.slider_changed = None;
        }
        self.submitted = None;
        if self
            .focused
            .is_some_and(|e| !world.is_active(e) || disabled(world, e))
        {
            self.focused = None;
        }
        if input.blur {
            self.focused = None;
            self.close_dropdowns(world, None);
        }
        // Escape puts an open list away first, and only then lets go.
        if input.escape && !self.close_dropdowns(world, None) {
            self.focused = None;
        }
        self.apply_autofocus(world);
        // A press anywhere moves focus to what it landed on, and a press on
        // nothing that takes focus -- a panel, the game behind -- lets go.
        if self.pointer_began {
            let landed = self.hovered.filter(|e| self.focusable(world, *e));
            self.close_dropdowns(world, landed);
            match landed {
                Some(entity) => self.focus(world, entity),
                None => self.focused = None,
            }
        }
        if input.next || input.previous {
            self.move_focus(world, input.previous);
        }
        let editing = self.editing_text(world);
        // In a field, left and right move the caret; up and down still move
        // focus, since a single line has nowhere else to go.
        let toward = [
            (input.up, Toward::Up),
            (input.down, Toward::Down),
            (input.left && !editing, Toward::Left),
            (input.right && !editing, Toward::Right),
        ]
        .into_iter()
        .find_map(|(pressed, toward)| pressed.then_some(toward));
        if let Some(toward) = toward
            && !self.nudge_slider(world, toward)
        {
            self.move_focus_toward(world, toward);
        }
        // Space and Enter press a focused button or toggle. A focused text
        // field keeps them: Space is a letter there and Enter submits, and a
        // field that also reported a press would be answered twice.
        let keyed = self
            .focused
            .filter(|e| input.activate && !has(world, *e, UiTextInputComponent::TYPE_NAME));
        let activated = self.clicked.or(keyed);
        if let Some(entity) = activated.filter(|e| world.is_active(*e) && !disabled(world, *e)) {
            self.clicked = Some(entity);
            if has(world, entity, UiToggleComponent::TYPE_NAME) {
                self.press_toggle(world, entity);
            } else {
                self.press_choice(world, entity);
            }
        }
        if let Some(entity) = self
            .focused
            .filter(|e| has(world, *e, UiTextInputComponent::TYPE_NAME))
        {
            self.edit_text(world, entity, input);
        }
        if input.scroll.is_finite()
            && input.scroll.abs() > f32::EPSILON
            && let Some(point) = self.pointer_overlay
            && let Some(entity) = self.scroll_region_at(world, point)
        {
            self.scroll_by(world, entity, -input.scroll);
        }
        Self::sync_dropdowns(world);
    }

    /// Whether the focused element is a text input, which is where typed
    /// keys go rather than to gameplay.
    ///
    /// A host asks this after [`Self::read_controls`] and holds keys back
    /// from its scripts while it is true: typing a name that has a W in it
    /// must not also walk the ship forward.
    #[must_use]
    pub fn editing_text(&self, world: &World) -> bool {
        self.focused.is_some_and(|entity| {
            world.get(entity).is_some_and(|data| {
                data.components
                    .contains_key(UiTextInputComponent::TYPE_NAME)
            })
        })
    }

    /// The frontmost scroll region under `point` that is not switched off:
    /// where the wheel goes.
    fn scroll_region_at(&self, world: &World, point: [f32; 2]) -> Option<EntityId> {
        self.rects
            .iter()
            .filter(|(entity, element)| {
                element.rect.contains(point)
                    && element.clip.is_none_or(|clip| clip.contains(point))
                    && !disabled(world, **entity)
                    && world.get(**entity).is_some_and(|data| {
                        data.components.contains_key(UiScrollComponent::TYPE_NAME)
                    })
            })
            .max_by_key(|(entity, element)| (element.layer, entity.index()))
            .map(|(entity, _)| *entity)
    }

    pub(super) fn read_scroll_drag(
        &mut self,
        world: &mut World,
        extent: super::ScreenExtent,
        presses: &sindri_core::Presses,
    ) -> bool {
        if let Some((entity, id, last, origin, mut dragging)) = self.scroll_drag {
            let Some(press) = presses.get(id) else {
                self.scroll_drag = None;
                return false;
            };
            if press.phase() == sindri_core::PressPhase::Cancelled {
                self.scroll_drag = None;
                return false;
            }
            if let Some(point) = extent.pointer(press.position()) {
                dragging |= (point[1] - origin).abs() > 0.025;
                if dragging {
                    self.scroll_by(world, entity, point[1] - last);
                }
                self.scroll_drag = Some((entity, id, point[1], origin, dragging));
            }
            if press.phase() == sindri_core::PressPhase::Ended {
                self.scroll_drag = None;
            }
            return dragging;
        }
        for press in presses.began() {
            let Some(point) = extent.pointer(press.position()) else {
                continue;
            };
            let mut current = self.topmost_at(point);
            for _ in 0..64 {
                let Some(e) = current else {
                    break;
                };
                let Some(data) = world.get(e) else {
                    break;
                };
                if data.components.contains_key(UiScrollComponent::TYPE_NAME) && !disabled(world, e)
                {
                    self.scroll_drag = Some((e, press.id(), point[1], point[1], false));
                    break;
                }
                current = data.parent;
            }
        }
        false
    }

    /// Whether `entity` can hold focus: something pressable and seen, live
    /// and switched on. A scroll region is moved through, not activated, and
    /// a row scrolled out of its region's view is not somewhere the keyboard
    /// should land.
    pub(super) fn focusable(&self, world: &World, entity: EntityId) -> bool {
        self.rects.get(&entity).is_some_and(|element| {
            element.pressable
                && element
                    .clip
                    .is_none_or(|clip| clip.size[0] > 0.0 && clip.size[1] > 0.0)
        }) && world.is_active(entity)
            && !disabled(world, entity)
            && !has(world, entity, UiScrollComponent::TYPE_NAME)
    }

    pub(super) fn scroll_by(&mut self, world: &mut World, entity: EntityId, delta: f32) {
        let Some(rect) = self.rect(entity) else {
            return;
        };
        let Some(payload) = world
            .get_mut(entity)
            .and_then(|d| d.components.get_mut(UiScrollComponent::TYPE_NAME))
        else {
            return;
        };
        let Ok(scroll) = serde_json::from_value::<UiScrollComponent>(payload.clone()) else {
            return;
        };
        if scroll.disabled {
            return;
        }
        let measured = self.scroll_content(entity).unwrap_or(0.0);
        let offset = scroll.coerce(scroll.offset + delta, rect.size[1], measured);
        if (offset - scroll.offset).abs() > f32::EPSILON {
            payload["offset"] = serde_json::json!(offset);
            self.changed.insert(entity);
        }
    }
}

fn has(world: &World, entity: EntityId, component: &str) -> bool {
    world
        .get(entity)
        .is_some_and(|data| data.components.contains_key(component))
}

fn disabled(world: &World, entity: EntityId) -> bool {
    world.get(entity).is_some_and(|d| {
        d.components
            .values()
            .any(|p| p.get("disabled").and_then(serde_json::Value::as_bool) == Some(true))
    })
}
