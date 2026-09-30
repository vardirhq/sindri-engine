//! Shared widget interaction, independent of the window and input backend.
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
        self.changed.clear();
        self.submitted = None;
        if self
            .focused
            .is_some_and(|e| !world.is_active(e) || disabled(world, e))
        {
            self.focused = None;
        }
        if input.blur || input.escape {
            self.focused = None;
        }
        if self.pointer_began {
            self.focused = self.hovered;
        }
        if input.next || input.previous {
            self.move_focus(world, input.previous);
        }
        let activated = self
            .clicked
            .or_else(|| input.activate.then_some(self.focused).flatten());
        if let Some(entity) = activated.filter(|e| world.is_active(*e) && !disabled(world, *e)) {
            self.clicked = Some(entity);
            if let Some(payload) = world
                .get_mut(entity)
                .and_then(|d| d.components.get_mut(UiToggleComponent::TYPE_NAME))
            {
                let checked = payload
                    .get("checked")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                payload["checked"] = serde_json::json!(!checked);
                self.changed.insert(entity);
            }
        }
        if let Some(entity) = self.focused {
            self.edit_text(world, entity, input);
        }
        if input.scroll.is_finite() && input.scroll.abs() > f32::EPSILON {
            if let Some(point) = self.pointer_overlay {
                let region = self
                    .rects
                    .iter()
                    .filter(|(e, element)| {
                        element.rect.contains(point)
                            && world.get(**e).is_some_and(|d| {
                                d.components.contains_key(UiScrollComponent::TYPE_NAME)
                            })
                    })
                    .max_by_key(|(e, element)| (element.layer, e.index()))
                    .map(|(e, _)| *e);
                if let Some(entity) = region {
                    self.scroll_by(world, entity, -input.scroll);
                }
            }
        }
        for (entity, data) in world
            .entities()
            .map(|(e, d)| {
                (
                    e,
                    d.components.contains_key(UiTextInputComponent::TYPE_NAME),
                )
            })
            .collect::<Vec<_>>()
        {
            if data {
                if let Some(payload) = world
                    .get_mut(entity)
                    .and_then(|d| d.components.get_mut(UiTextInputComponent::TYPE_NAME))
                {
                    payload["focused"] = serde_json::json!(self.focused == Some(entity));
                }
            }
        }
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

    fn move_focus(&mut self, world: &World, backwards: bool) {
        let mut entities: Vec<_> = self
            .rects
            .iter()
            .filter(|(e, element)| {
                element.pressable && world.is_active(**e) && !disabled(world, **e)
            })
            .collect();
        entities.sort_by(|(a, ar), (b, br)| {
            br.rect.center[1]
                .total_cmp(&ar.rect.center[1])
                .then(ar.rect.center[0].total_cmp(&br.rect.center[0]))
                .then(a.cmp(b))
        });
        let ids: Vec<_> = entities.into_iter().map(|(e, _)| *e).collect();
        if ids.is_empty() {
            self.focused = None;
            return;
        }
        let current = self
            .focused
            .and_then(|e| ids.iter().position(|id| *id == e));
        let at = current.map_or(if backwards { ids.len() - 1 } else { 0 }, |at| {
            if backwards {
                (at + ids.len() - 1) % ids.len()
            } else {
                (at + 1) % ids.len()
            }
        });
        self.focused = Some(ids[at]);
    }

    fn edit_text(&mut self, world: &mut World, entity: EntityId, input: &UiInput) {
        let Some(payload) = world
            .get_mut(entity)
            .and_then(|d| d.components.get_mut(UiTextInputComponent::TYPE_NAME))
        else {
            return;
        };
        let Ok(field) = serde_json::from_value::<UiTextInputComponent>(payload.clone()) else {
            return;
        };
        if field.disabled {
            return;
        }
        let mut value = field.value.clone();
        if input.backspace {
            value.pop();
        }
        value.push_str(&input.text);
        let value = field.coerce(&value);
        if value != field.value {
            payload["value"] = serde_json::json!(value);
            self.changed.insert(entity);
        }
        if input.submit {
            self.submitted = Some(entity);
        }
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
        let offset = scroll.coerce(scroll.offset + delta, rect.size[1]);
        if (offset - scroll.offset).abs() > f32::EPSILON {
            payload["offset"] = serde_json::json!(offset);
            self.changed.insert(entity);
        }
    }
}

fn disabled(world: &World, entity: EntityId) -> bool {
    world.get(entity).is_some_and(|d| {
        d.components
            .values()
            .any(|p| p.get("disabled").and_then(serde_json::Value::as_bool) == Some(true))
    })
}
