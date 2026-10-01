//! Editing a single line of text: a caret, a selection, and the clipboard.
//!
//! Where the caret is belongs to the person editing rather than to the
//! scene, so it lives here beside the world, counted in Unicode scalars like
//! the field's limit. A field that gains focus puts the caret at its end. A
//! script reads it through [`ScreenUi::caret`] to draw a caret or a
//! highlight, and a value a script rewrites keeps the caret inside it.
//!
//! Copy and cut hand their text to the host through [`ScreenUi::take_copied`],
//! because the clipboard is the window's; paste arrives as committed text,
//! which is what a host's clipboard read turns into.
use sindri_core::{EntityId, SceneComponent, World};

use super::{ScreenUi, UiInput, UiTextInputComponent};

/// Where a field's caret is and where its selection began, in characters.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UiCaret {
    /// Where the selection started; equal to `caret` when nothing is selected.
    pub anchor: usize,
    pub caret: usize,
}

impl UiCaret {
    /// The selected characters, start to end.
    #[must_use]
    pub fn selection(self) -> std::ops::Range<usize> {
        self.anchor.min(self.caret)..self.anchor.max(self.caret)
    }

    fn clamped(self, len: usize) -> Self {
        Self {
            anchor: self.anchor.min(len),
            caret: self.caret.min(len),
        }
    }

    fn at(position: usize) -> Self {
        Self {
            anchor: position,
            caret: position,
        }
    }
}

impl ScreenUi {
    /// Where the caret and selection are in a text field, or `None` for
    /// anything else. A field nobody has edited reports its end.
    #[must_use]
    pub fn caret(&self, world: &World, entity: EntityId) -> Option<UiCaret> {
        let value = field(world, entity)?.value;
        let len = value.chars().count();
        Some(
            self.carets
                .get(&entity)
                .map_or(UiCaret::at(len), |caret| caret.clamped(len)),
        )
    }

    /// Text that a copy or cut put on the clipboard this step, for the host
    /// to hand to the system clipboard.
    pub fn take_copied(&mut self) -> Option<String> {
        self.copied.take()
    }

    pub(super) fn edit_text(&mut self, world: &mut World, entity: EntityId, input: &UiInput) {
        let Some(field) = field(world, entity) else {
            return;
        };
        if field.disabled {
            return;
        }
        let mut chars: Vec<char> = field.value.chars().collect();
        let mut caret = self
            .carets
            .get(&entity)
            .map_or(UiCaret::at(chars.len()), |caret| caret.clamped(chars.len()));
        let selected = caret.selection();
        if input.select_all {
            caret = UiCaret {
                anchor: 0,
                caret: chars.len(),
            };
        }
        if (input.copy || input.cut) && !selected.is_empty() {
            self.copied = Some(chars[selected.clone()].iter().collect());
            if input.cut {
                chars.drain(selected.clone());
                caret = UiCaret::at(selected.start);
            }
        }
        caret = moved(caret, input, chars.len());
        let selected = caret.selection();
        if input.backspace || input.delete {
            if !selected.is_empty() {
                chars.drain(selected.clone());
                caret = UiCaret::at(selected.start);
            } else if input.backspace && caret.caret > 0 {
                chars.remove(caret.caret - 1);
                caret = UiCaret::at(caret.caret - 1);
            } else if input.delete && caret.caret < chars.len() {
                chars.remove(caret.caret);
            }
        }
        let typed: Vec<char> = input.text.chars().filter(|c| !c.is_control()).collect();
        if !typed.is_empty() {
            let selected = caret.selection();
            chars.drain(selected.clone());
            // What does not fit is not typed, as a browser's maxlength does.
            let room = field.max_length.saturating_sub(chars.len());
            let fits = typed.len().min(room);
            chars.splice(
                selected.start..selected.start,
                typed[..fits].iter().copied(),
            );
            caret = UiCaret::at(selected.start + fits);
        }
        let value: String = chars.into_iter().collect();
        if value != field.value
            && let Some(payload) = world
                .get_mut(entity)
                .and_then(|d| d.components.get_mut(UiTextInputComponent::TYPE_NAME))
        {
            payload["value"] = serde_json::json!(value);
            self.changed.insert(entity);
        }
        self.carets.insert(entity, caret);
        if input.submit {
            self.submitted = Some(entity);
        }
    }
}

/// The caret after Left, Right, Home and End, growing the selection while
/// Shift is held and otherwise collapsing it as a text box does.
fn moved(caret: UiCaret, input: &UiInput, len: usize) -> UiCaret {
    let selected = caret.selection();
    let target = if input.home {
        Some(0)
    } else if input.end {
        Some(len)
    } else if input.left {
        Some(if !input.extend && !selected.is_empty() {
            selected.start
        } else {
            caret.caret.saturating_sub(1)
        })
    } else if input.right {
        Some(if !input.extend && !selected.is_empty() {
            selected.end
        } else {
            (caret.caret + 1).min(len)
        })
    } else {
        None
    };
    match target {
        Some(to) if input.extend => UiCaret {
            anchor: caret.anchor,
            caret: to,
        },
        Some(to) => UiCaret::at(to),
        None => caret,
    }
}

fn field(world: &World, entity: EntityId) -> Option<UiTextInputComponent> {
    let payload = world
        .get(entity)?
        .components
        .get(UiTextInputComponent::TYPE_NAME)?;
    serde_json::from_value(payload.clone()).ok()
}
