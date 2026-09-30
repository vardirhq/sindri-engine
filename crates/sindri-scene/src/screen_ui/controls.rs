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
        // A press anywhere moves focus to what it landed on, and a press on
        // nothing that takes focus -- a panel, the game behind -- lets go.
        if self.pointer_began {
            self.focused = self.hovered.filter(|e| self.focusable(world, *e));
        }
        if input.next || input.previous {
            self.move_focus(world, input.previous);
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
        if input.scroll.is_finite()
            && input.scroll.abs() > f32::EPSILON
            && let Some(point) = self.pointer_overlay
            && let Some(entity) = self.scroll_region_at(world, point)
        {
            self.scroll_by(world, entity, -input.scroll);
        }
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
    fn focusable(&self, world: &World, entity: EntityId) -> bool {
        self.rects.get(&entity).is_some_and(|element| {
            element.pressable
                && element.clip.is_none_or(|clip| {
                    clip.size[0] > 0.0 && clip.size[1] > 0.0 && overlaps(clip, element.rect)
                })
        }) && world.is_active(entity)
            && !disabled(world, entity)
            && !has(world, entity, UiScrollComponent::TYPE_NAME)
    }

    fn move_focus(&mut self, world: &World, backwards: bool) {
        let mut entities: Vec<_> = self
            .rects
            .iter()
            .filter(|(e, _)| self.focusable(world, **e))
            .collect();
        // Document order, as a browser tabs: the order the scene is written
        // in, parents before children. Reading the screen top to bottom
        // instead zig-zags between side-by-side panels.
        let order = document_order(world);
        entities.sort_by_key(|(entity, _)| order.get(*entity).copied().unwrap_or(usize::MAX));
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
        let measured = self.scroll_content(entity).unwrap_or(0.0);
        let offset = scroll.coerce(scroll.offset + delta, rect.size[1], measured);
        if (offset - scroll.offset).abs() > f32::EPSILON {
            payload["offset"] = serde_json::json!(offset);
            self.changed.insert(entity);
        }
    }
}

/// Every entity's place in a depth-first walk of the hierarchy, roots and
/// siblings in the order they were made.
fn document_order(world: &World) -> std::collections::BTreeMap<EntityId, usize> {
    let mut order = std::collections::BTreeMap::new();
    let mut pending: Vec<EntityId> = world
        .entities()
        .filter(|(_, data)| data.parent.is_none())
        .map(|(entity, _)| entity)
        .collect();
    pending.reverse();
    while let Some(entity) = pending.pop() {
        if order.contains_key(&entity) {
            continue;
        }
        order.insert(entity, order.len());
        if let Some(data) = world.get(entity) {
            pending.extend(data.children.iter().rev().copied());
        }
    }
    order
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

/// Whether two rectangles share any area.
fn overlaps(a: super::ScreenRect, b: super::ScreenRect) -> bool {
    (0..2).all(|axis| (a.center[axis] - b.center[axis]).abs() * 2.0 < a.size[axis] + b.size[axis])
}
