//! Moving focus without a pointer: Tab through the scene's order, and the
//! arrows or a pad's d-pad toward whichever control lies that way.
//!
//! Both land on a control a person could press, and a row inside a scroll
//! region is scrolled into view when the keyboard reaches it, as a browser
//! scrolls a focused element into view.
use std::collections::{BTreeMap, BTreeSet};

use glam::Vec2;
use sindri_core::{EntityId, SceneComponent, World};

use super::{ScreenRect, ScreenUi, UiScrollComponent};

/// A direction the arrows, d-pad or stick asked focus to move.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Toward {
    Left,
    Right,
    Up,
    Down,
}

impl Toward {
    fn unit(self) -> Vec2 {
        match self {
            Self::Left => Vec2::NEG_X,
            Self::Right => Vec2::X,
            Self::Up => Vec2::Y,
            Self::Down => Vec2::NEG_Y,
        }
    }
}

impl ScreenUi {
    /// Every control focus can land on, in the order the scene is written.
    fn focus_order(&self, world: &World) -> Vec<EntityId> {
        let order = document_order(world);
        let mut ids: Vec<EntityId> = self
            .rects
            .keys()
            .copied()
            .filter(|e| self.focusable(world, *e))
            .collect();
        ids.sort_by_key(|entity| order.get(entity).copied().unwrap_or(usize::MAX));
        ids
    }

    /// Tab and Shift+Tab: the next or previous control in document order.
    pub(super) fn move_focus(&mut self, world: &mut World, backwards: bool) {
        let ids = self.focus_order(world);
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
        self.focus(world, ids[at]);
        // Tabbing out of an open list puts it away.
        self.close_dropdowns(world, Some(ids[at]));
    }

    /// The arrows and d-pad: the nearest control whose centre lies that way,
    /// preferring one in line over one off to the side. With nothing focused
    /// yet, the first control in document order, so a pad can start a menu.
    pub(super) fn move_focus_toward(&mut self, world: &mut World, toward: Toward) {
        let Some(from) = self
            .focused
            .and_then(|e| self.rects.get(&e))
            .map(|e| e.rect)
        else {
            // Nothing focused: only a screen that asked for focus takes it.
            // A game that moves with the arrows and jumps with Space must not
            // have its pause button focused, and then pressed, by walking.
            if let Some(first) = self
                .focus_order(world)
                .into_iter()
                .find(|e| autofocus(world, *e))
            {
                self.focus(world, first);
            }
            return;
        };
        let origin = Vec2::from_array(from.center);
        let unit = toward.unit();
        // An open list keeps the arrows, as a native select does: whatever
        // its popup happens to cover is not somewhere they go.
        let open_list = self
            .focused
            .and_then(|f| super::choice::dropdown_of(world, f))
            .filter(|dropdown| super::choice::is_open(world, *dropdown))
            .map(|dropdown| super::choice::options_of(world, dropdown));
        let best = self
            .focus_order(world)
            .into_iter()
            .filter(|e| open_list.as_ref().is_none_or(|options| options.contains(e)))
            .filter(|e| Some(*e) != self.focused)
            .filter_map(|e| {
                let rect = self.rects.get(&e)?.rect;
                let to = Vec2::from_array(rect.center) - origin;
                let along = to.dot(unit);
                // Ahead of the control, not merely beside it.
                if along <= 1.0e-4 {
                    return None;
                }
                let across = (to - unit * along).length();
                // Anything in the focused control's beam -- overlapping it
                // across the way focus moves -- comes before anything off to
                // the side, nearest first, as a browser's spatial navigation
                // does; off the beam, the nearer in line wins.
                let in_beam = in_beam(from, rect, toward);
                let score = if in_beam {
                    along
                } else {
                    1.0e3 + along + across * 2.0
                };
                Some((e, score, across))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.2.total_cmp(&b.2)));
        if let Some((entity, _, _)) = best {
            self.focus(world, entity);
        }
    }

    /// Focuses an `autofocus` control that has just come into view, when
    /// nothing has focus: a menu that asks for it is ready for a pad or the
    /// arrows the moment it appears, as a page's `autofocus` field is.
    pub(super) fn apply_autofocus(&mut self, world: &mut World) {
        let order = self.focus_order(world);
        let wanting: BTreeSet<EntityId> = order
            .iter()
            .copied()
            .filter(|e| autofocus(world, *e))
            .collect();
        let arrived = order
            .into_iter()
            .find(|e| wanting.contains(e) && !self.autofocused.contains(e));
        self.autofocused = wanting;
        if self.focused.is_none()
            && let Some(entity) = arrived
        {
            self.focus(world, entity);
        }
    }

    /// Gives `entity` the keyboard, scrolled into view if a region holds it.
    pub(super) fn focus(&mut self, world: &mut World, entity: EntityId) {
        if self.focused != Some(entity) {
            // A field reached by the keyboard edits from its end.
            self.carets.remove(&entity);
        }
        self.focused = Some(entity);
        self.scroll_into_view(world, entity);
    }

    /// Scrolls the region around `entity`, if any, just far enough to show it.
    fn scroll_into_view(&mut self, world: &mut World, entity: EntityId) {
        let Some(element) = self.rects.get(&entity).map(|e| e.rect) else {
            return;
        };
        let mut current = world.get(entity).and_then(|d| d.parent);
        for _ in 0..64 {
            let Some(parent) = current else { return };
            let Some(data) = world.get(parent) else {
                return;
            };
            if data.components.contains_key(UiScrollComponent::TYPE_NAME) {
                let Some(region) = self.rects.get(&parent).map(|e| e.rect) else {
                    return;
                };
                let delta = reveal(region, element);
                if delta.abs() > f32::EPSILON {
                    self.scroll_by(world, parent, delta);
                }
                return;
            }
            current = data.parent;
        }
    }
}

/// Whether a control asks for focus when it appears: `"autofocus": true`
/// on any of its components.
fn autofocus(world: &World, entity: EntityId) -> bool {
    world.get(entity).is_some_and(|data| {
        data.components.values().any(|payload| {
            payload
                .get("autofocus")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
        })
    })
}

/// Whether `to` overlaps `from` across the direction focus is moving.
fn in_beam(from: ScreenRect, to: ScreenRect, toward: Toward) -> bool {
    let axis = match toward {
        Toward::Left | Toward::Right => 1,
        Toward::Up | Toward::Down => 0,
    };
    (from.center[axis] - to.center[axis]).abs() * 2.0 < from.size[axis] + to.size[axis]
}

/// How far to scroll a region so `element` is inside it: positive moves the
/// content up, to show something below; negative shows something above.
fn reveal(region: ScreenRect, element: ScreenRect) -> f32 {
    let region_top = region.center[1] + region.size[1] / 2.0;
    let region_bottom = region.center[1] - region.size[1] / 2.0;
    let top = element.center[1] + element.size[1] / 2.0;
    let bottom = element.center[1] - element.size[1] / 2.0;
    if bottom < region_bottom {
        region_bottom - bottom
    } else if top > region_top {
        region_top - top
    } else {
        0.0
    }
}

/// Every entity's place in a depth-first walk of the hierarchy, roots and
/// siblings in the order they were made.
pub(super) fn document_order(world: &World) -> BTreeMap<EntityId, usize> {
    let mut order = BTreeMap::new();
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

#[cfg(test)]
mod tests {
    use super::{ScreenRect, reveal};

    fn rect(y: f32, h: f32) -> ScreenRect {
        ScreenRect {
            center: [0.0, y],
            size: [1.0, h],
        }
    }

    #[test]
    fn a_row_below_or_above_the_view_is_scrolled_just_into_it() {
        let region = rect(0.0, 1.0);
        assert!((reveal(region, rect(-0.7, 0.2)) - 0.3).abs() < 1.0e-6);
        assert!((reveal(region, rect(0.7, 0.2)) + 0.3).abs() < 1.0e-6);
        assert!(reveal(region, rect(0.0, 0.2)).abs() < 1.0e-6);
    }
}
