//! The welcome window's Learn panel: where the documentation is.
//!
//! Each row names a page in `docs/`. A clone has the page beside the editor
//! and opens that copy, which is the version matching the code being run; an
//! installed editor has no repository next to it and opens the page on GitHub
//! instead. Either way the system's own opener shows it, so a Markdown file
//! lands in whatever the user reads Markdown with.

use std::path::{Path, PathBuf};
use std::process::Command;

use eframe::egui::{self, Align2, Pos2};

use crate::ui::icons;
use crate::ui::theme::{color, radius, text};
use crate::ui::widgets::button;

/// Where a page is read from when the repository is not on disk.
const REMOTE: &str = "https://github.com/vardirhq/sindri-engine/blob/main";

const ROW_HEIGHT: f32 = 42.0;

/// A documentation page the panel offers.
pub(super) struct Guide {
    pub(super) glyph: egui_material_icons::MaterialIcon,
    pub(super) title: &'static str,
    pub(super) detail: &'static str,
    /// Relative to the repository root.
    pub(super) page: &'static str,
}

pub(super) const GUIDES: [Guide; 5] = [
    Guide {
        glyph: icons::SCENE,
        title: "Documentation",
        detail: "Every guide, by subject.",
        page: "docs/README.md",
    },
    Guide {
        glyph: icons::SCRIPT,
        title: "Scripting with Decay",
        detail: "Gameplay logic in typed scripts.",
        page: "docs/scripting.md",
    },
    Guide {
        glyph: icons::LIST_VIEW,
        title: "Decay API reference",
        detail: "Every type and function a script can call.",
        page: "docs/generated/decay-api.md",
    },
    Guide {
        glyph: icons::UI_ELEMENT,
        title: "UI with Weave",
        detail: "Responsive game interfaces.",
        page: "docs/weave.md",
    },
    Guide {
        glyph: icons::FOLDER,
        title: "Exporting to the web",
        detail: "Ship a project to the browser.",
        page: "docs/export.md",
    },
];

/// Draws one guide as a row, and says whether it was clicked.
pub(super) fn guide_row(ui: &mut egui::Ui, guide: &Guide) -> bool {
    let (rect, response) = button::row_sense(ui, ROW_HEIGHT);
    if response.hovered() {
        ui.painter().rect_filled(rect, radius(), color::EMBER_FAINT);
    }
    let painter = ui.painter_at(rect);
    let glyph = guide.glyph.outlined();
    painter.text(
        Pos2::new(rect.left() + 16.0, rect.center().y),
        Align2::CENTER_CENTER,
        glyph.codepoint,
        egui::FontId::new(18.0, glyph.font_family()),
        color::FORGE,
    );
    let left = rect.left() + 36.0;
    painter.text(
        Pos2::new(left, rect.top() + 6.0),
        Align2::LEFT_TOP,
        guide.title,
        egui::FontId::proportional(text::BODY),
        color::TEXT,
    );
    painter.text(
        Pos2::new(left, rect.bottom() - 6.0),
        Align2::LEFT_BOTTOM,
        guide.detail,
        egui::FontId::proportional(text::NOTE),
        color::TEXT_FAINT,
    );
    let chevron = icons::COLLAPSED.outlined();
    painter.text(
        Pos2::new(rect.right() - 10.0, rect.center().y),
        Align2::CENTER_CENTER,
        chevron.codepoint,
        egui::FontId::new(16.0, chevron.font_family()),
        if response.hovered() {
            color::TEXT
        } else {
            color::TEXT_FAINT
        },
    );
    response.on_hover_text(guide.page).clicked()
}

/// Where a page is read from: the local copy when there is one.
pub(super) fn location(page: &str, root: &Path) -> String {
    let local: PathBuf = root.join(page);
    if local.is_file() {
        std::path::absolute(&local)
            .unwrap_or(local)
            .display()
            .to_string()
    } else {
        format!("{REMOTE}/{page}")
    }
}

/// Hands a page to the system's opener.
pub(super) fn open(page: &str) -> Result<(), String> {
    let target = location(page, Path::new("."));
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", ""]);
        command
    } else if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    };
    command
        .arg(&target)
        .spawn()
        .map(drop)
        .map_err(|error| format!("Could not open {target}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_guide_names_a_page_that_exists() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        for guide in &GUIDES {
            assert!(root.join(guide.page).is_file(), "{} is missing", guide.page);
        }
    }

    #[test]
    fn a_page_missing_locally_is_read_from_github() {
        let nowhere = Path::new("/nonexistent-sindri-root");
        assert_eq!(
            location("docs/weave.md", nowhere),
            format!("{REMOTE}/docs/weave.md")
        );
    }
}
