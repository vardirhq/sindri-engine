//! Choosing one of several: radio groups and dropdowns.
//!
//! A radio button is a toggle with a `group`; choosing one unchecks the rest.
//! A dropdown is a pressable header with `sindri.ui.option` rows authored
//! under it. Whichever of its direct children holds the options is its popup,
//! shown only while it is open -- the engine switches it, so a closed popup
//! is neither drawn nor pressed nor reached by the keyboard. Each option is
//! numbered by its place among the dropdown's options in scene order, carries
//! `checked` while it is the one selected (Weave's `:checked`), and the
//! dropdown carries `open` while it is (Weave's `:open`).
use serde::Deserialize;
use sindri_core::{EntityId, SceneComponent, World};

use super::{ScreenUi, UiToggleComponent};

/// A header that opens a list of options, of which one is selected.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct UiDropdownComponent {
    #[serde(default)]
    pub label: String,
    /// Which option is chosen, counted from zero in scene order.
    #[serde(default)]
    pub selected: usize,
    #[serde(default)]
    pub open: bool,
    #[serde(default)]
    pub disabled: bool,
    /// Takes focus when it appears and nothing else has it, so a menu is
    /// ready for a pad or the arrows; as HTML's `autofocus`.
    #[serde(default)]
    pub autofocus: bool,
}
impl SceneComponent for UiDropdownComponent {
    const TYPE_NAME: &'static str = "sindri.ui.dropdown";
}

/// One choice inside a dropdown.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct UiOptionComponent {
    #[serde(default)]
    pub label: String,
    /// Kept by the engine: whether this is the dropdown's selected option.
    #[serde(default)]
    pub checked: bool,
    #[serde(default)]
    pub disabled: bool,
}
impl SceneComponent for UiOptionComponent {
    const TYPE_NAME: &'static str = "sindri.ui.option";
}

/// How deep a dropdown's options may sit below it.
const MAX_DEPTH: usize = 16;

impl ScreenUi {
    /// Presses `entity` as a toggle: flips a switch, or chooses a radio
    /// button and lets go of the rest of its group.
    pub(super) fn press_toggle(&mut self, world: &mut World, entity: EntityId) {
        let Some(toggle) = component::<UiToggleComponent>(world, entity) else {
            return;
        };
        if toggle.group.is_empty() {
            set_flag(
                world,
                entity,
                UiToggleComponent::TYPE_NAME,
                "checked",
                !toggle.checked,
            );
            self.changed.insert(entity);
            return;
        }
        let peers: Vec<EntityId> = world
            .entities()
            .filter(|(other, data)| {
                *other != entity
                    && data
                        .components
                        .get(UiToggleComponent::TYPE_NAME)
                        .and_then(|payload| payload.get("group"))
                        .and_then(serde_json::Value::as_str)
                        == Some(toggle.group.as_str())
            })
            .map(|(other, _)| other)
            .collect();
        for peer in peers {
            if component::<UiToggleComponent>(world, peer).is_some_and(|p| p.checked) {
                set_flag(world, peer, UiToggleComponent::TYPE_NAME, "checked", false);
                self.changed.insert(peer);
            }
        }
        if !toggle.checked {
            set_flag(world, entity, UiToggleComponent::TYPE_NAME, "checked", true);
            self.changed.insert(entity);
        }
    }

    /// Presses a dropdown's header, which opens or closes it, or one of its
    /// options, which chooses it and closes the list. True if `entity` was
    /// either.
    pub(super) fn press_choice(&mut self, world: &mut World, entity: EntityId) -> bool {
        if let Some(dropdown) = component::<UiDropdownComponent>(world, entity) {
            let open = !dropdown.open;
            set_flag(world, entity, UiDropdownComponent::TYPE_NAME, "open", open);
            if open {
                // Opened from the keyboard, focus lands on the chosen option
                // so the arrows walk the list from where it stands.
                let options = options_of(world, entity);
                if self.focused == Some(entity) {
                    self.focused = options.get(dropdown.selected).copied();
                }
            }
            return true;
        }
        if !world
            .get(entity)
            .is_some_and(|data| data.components.contains_key(UiOptionComponent::TYPE_NAME))
        {
            return false;
        }
        let Some(dropdown) = dropdown_of(world, entity) else {
            return true;
        };
        let index = options_of(world, dropdown)
            .iter()
            .position(|o| *o == entity);
        if let (Some(index), Some(current)) =
            (index, component::<UiDropdownComponent>(world, dropdown))
            && index != current.selected
        {
            if let Some(payload) = world
                .get_mut(dropdown)
                .and_then(|d| d.components.get_mut(UiDropdownComponent::TYPE_NAME))
            {
                payload["selected"] = serde_json::json!(index);
            }
            self.changed.insert(dropdown);
        }
        set_flag(
            world,
            dropdown,
            UiDropdownComponent::TYPE_NAME,
            "open",
            false,
        );
        if self.focused == Some(entity) {
            self.focused = Some(dropdown);
        }
        true
    }

    /// Closes every open dropdown that does not hold `keep`: a press
    /// elsewhere, or Escape, puts a list away.
    pub(super) fn close_dropdowns(&mut self, world: &mut World, keep: Option<EntityId>) -> bool {
        let open: Vec<EntityId> = world
            .entities()
            .filter(|(_, data)| {
                data.components
                    .get(UiDropdownComponent::TYPE_NAME)
                    .and_then(|payload| payload.get("open"))
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
            })
            .map(|(entity, _)| entity)
            .collect();
        let mut closed = false;
        for dropdown in open {
            if keep
                .is_some_and(|keep| keep == dropdown || dropdown_of(world, keep) == Some(dropdown))
            {
                continue;
            }
            set_flag(
                world,
                dropdown,
                UiDropdownComponent::TYPE_NAME,
                "open",
                false,
            );
            if self
                .focused
                .is_some_and(|f| dropdown_of(world, f) == Some(dropdown))
            {
                self.focused = Some(dropdown);
            }
            closed = true;
        }
        closed
    }

    /// Lays every dropdown's state onto its parts: the popup switched on only
    /// while open, and the selected option checked.
    pub(super) fn sync_dropdowns(world: &mut World) {
        let dropdowns: Vec<(EntityId, UiDropdownComponent)> = world
            .entities()
            .filter_map(|(entity, _)| {
                Some((entity, component::<UiDropdownComponent>(world, entity)?))
            })
            .collect();
        for (dropdown, state) in dropdowns {
            let options = options_of(world, dropdown);
            for (index, option) in options.iter().enumerate() {
                let chosen = index == state.selected;
                if component::<UiOptionComponent>(world, *option)
                    .is_some_and(|o| o.checked != chosen)
                {
                    set_flag(
                        world,
                        *option,
                        UiOptionComponent::TYPE_NAME,
                        "checked",
                        chosen,
                    );
                }
            }
            let popups: Vec<EntityId> = world.get(dropdown).map_or_else(Vec::new, |data| {
                data.children
                    .iter()
                    .copied()
                    .filter(|child| options.iter().any(|o| is_within(world, *o, *child)))
                    .collect()
            });
            for popup in popups {
                // Only on a change: a write is an edit to the entity, and a
                // list left as it was is not one.
                if world
                    .get(popup)
                    .is_some_and(|data| data.disabled == state.open)
                    && let Some(data) = world.get_mut(popup)
                {
                    data.disabled = !state.open;
                }
            }
        }
    }
}

/// A dropdown's options, in scene order: depth first through its children.
/// The first is option zero.
#[must_use]
pub fn dropdown_options(world: &World, dropdown: EntityId) -> Vec<EntityId> {
    options_of(world, dropdown)
}

pub(super) fn options_of(world: &World, dropdown: EntityId) -> Vec<EntityId> {
    let mut found = Vec::new();
    let mut pending: Vec<(EntityId, usize)> = world.get(dropdown).map_or_else(Vec::new, |data| {
        data.children.iter().rev().map(|c| (*c, 1)).collect()
    });
    while let Some((entity, depth)) = pending.pop() {
        let Some(data) = world.get(entity) else {
            continue;
        };
        if data.components.contains_key(UiOptionComponent::TYPE_NAME) {
            found.push(entity);
        }
        if depth < MAX_DEPTH {
            pending.extend(data.children.iter().rev().map(|c| (*c, depth + 1)));
        }
    }
    found
}

/// Whether a dropdown's list is showing.
pub(super) fn is_open(world: &World, dropdown: EntityId) -> bool {
    component::<UiDropdownComponent>(world, dropdown).is_some_and(|d| d.open)
}

/// The dropdown an option belongs to: its nearest ancestor that is one.
pub(super) fn dropdown_of(world: &World, entity: EntityId) -> Option<EntityId> {
    let mut current = world.get(entity)?.parent;
    for _ in 0..MAX_DEPTH {
        let parent = current?;
        let data = world.get(parent)?;
        if data.components.contains_key(UiDropdownComponent::TYPE_NAME) {
            return Some(parent);
        }
        current = data.parent;
    }
    None
}

fn is_within(world: &World, entity: EntityId, ancestor: EntityId) -> bool {
    let mut current = Some(entity);
    for _ in 0..=MAX_DEPTH {
        let Some(at) = current else { return false };
        if at == ancestor {
            return true;
        }
        current = world.get(at).and_then(|data| data.parent);
    }
    false
}

fn component<T: SceneComponent + serde::de::DeserializeOwned>(
    world: &World,
    entity: EntityId,
) -> Option<T> {
    let payload = world.get(entity)?.components.get(T::TYPE_NAME)?;
    serde_json::from_value(payload.clone()).ok()
}

fn set_flag(world: &mut World, entity: EntityId, component: &str, field: &str, value: bool) {
    if let Some(payload) = world
        .get_mut(entity)
        .and_then(|data| data.components.get_mut(component))
    {
        payload[field] = serde_json::json!(value);
    }
}
