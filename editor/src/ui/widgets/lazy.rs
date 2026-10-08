//! Lists that lay out only the rows in sight.
//!
//! A panel's list is drawn every frame it is shown, and a list of a thousand
//! rows laid out a thousand rows to show twenty. Inside a scroll area's
//! viewport, a row out of sight is only the space it takes: its height, as
//! measured the last time it was drawn, or a guess before it has been.

use std::collections::HashMap;

use eframe::egui::{Id, Rect, Ui};

/// How far beyond the visible edge rows are still drawn, so a row sliding in
/// is already laid out and keyboard focus moving onto one finds it there.
const MARGIN: f32 = 48.0;

/// Draws `rows` inside a scroll area's `show_viewport`, where `shown` is the
/// part of the content in sight. Each row has a key that is stable while it
/// means the same row, which its measured height is remembered by; `guess` is
/// the height of one not yet measured. `always` rows are drawn wherever they
/// are — one being renamed, say, which must keep its text field.
pub fn rows<T>(
    ui: &mut Ui,
    shown: Rect,
    id: Id,
    guess: f32,
    rows: impl IntoIterator<Item = (u64, T)>,
    always: impl Fn(&T) -> bool,
    mut draw: impl FnMut(&mut Ui, T),
) {
    let mut heights: HashMap<u64, f32> = ui.data_mut(|data| data.get_temp(id).unwrap_or_default());
    let mut measured = HashMap::with_capacity(heights.len());
    let top = ui.min_rect().top();
    let gap = ui.spacing().item_spacing.y;
    let mut unseen = 0.0;
    for (key, row) in rows {
        let height = heights.remove(&key).unwrap_or(guess);
        let at = ui.cursor().top() - top + unseen;
        let in_sight = at + height >= shown.top() - MARGIN && at <= shown.bottom() + MARGIN;
        if !in_sight && !always(&row) {
            unseen += height + gap;
            measured.insert(key, height);
            continue;
        }
        if unseen > 0.0 {
            ui.add_space(unseen);
            unseen = 0.0;
        }
        let before = ui.cursor().top();
        draw(ui, row);
        measured.insert(key, (ui.cursor().top() - before - gap).max(0.0));
    }
    if unseen > 0.0 {
        ui.add_space(unseen);
    }
    ui.data_mut(|data| data.insert_temp(id, measured));
}

/// A key for a row from anything hashable about it.
#[must_use]
pub fn key(of: impl std::hash::Hash) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    of.hash(&mut hasher);
    hasher.finish()
}
