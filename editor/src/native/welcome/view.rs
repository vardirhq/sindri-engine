//! The Sindri launchpad.
//!
//! The welcome window is the first piece of Sindri a new developer sees. It is
//! therefore a launch surface, not a file picker wearing a logo: the brand says
//! what Sindri is, the primary actions say how to begin, and recent and shipped
//! projects make returning to work immediate.

use eframe::egui::{self, Align, Align2, Layout, Pos2, RichText, UiBuilder, Vec2};

use crate::ui::theme::{color, hairline, hairline_soft, metric, radius, text};
use crate::ui::widgets::{
    button::{self, Intent},
    panel,
};
use crate::ui::{icons, widgets::button::outline};

use super::{Listing, NewProject, Request, Sample, Welcome, hero, learn};

const CARD_HEIGHT: f32 = 54.0;
const SIDE_WIDTH: f32 = 320.0;
const EXAMPLE_HEIGHT: f32 = 46.0;

enum Clicked {
    Open,
    Forget,
}

impl Welcome {
    pub(super) fn draw(&mut self, ui: &mut egui::Ui) {
        self.footer(ui);
        self.side_panel(ui);
        self.main_panel(ui);
        self.creating_form(ui);
    }

    fn footer(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("welcome-footer")
            .exact_size(metric::TOP_BAR_HEIGHT)
            .frame(egui::Frame::new().fill(color::HEADER))
            .show(ui, |ui| {
                let base = ui.max_rect();
                ui.painter()
                    .hline(base.x_range(), base.top() + 0.5, hairline());
                ui.horizontal_centered(|ui| {
                    ui.add_space(metric::GUTTER + 8.0);
                    let mut open_last = self.open_last;
                    if ui
                        .checkbox(
                            &mut open_last,
                            RichText::new("Open my last project on launch")
                                .size(text::LABEL)
                                .color(color::TEXT_MUTED),
                        )
                        .changed()
                    {
                        self.open_last = open_last;
                        self.changed = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(metric::GUTTER + 8.0);
                        if let Some(problem) = self.problem.clone() {
                            ui.label(
                                RichText::new(problem)
                                    .size(text::LABEL)
                                    .color(color::DANGER_TEXT),
                            );
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "Sindri Editor {}  ·  pre-alpha",
                                    env!("CARGO_PKG_VERSION")
                                ))
                                .size(text::NOTE)
                                .color(color::TEXT_FAINT),
                            );
                        }
                    });
                });
            });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("welcome-side")
            .exact_size(SIDE_WIDTH)
            .frame(egui::Frame::new().fill(color::HEADER))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        panel::body(ui, |ui| {
                            section_title(ui, "START");
                            ui.add_space(metric::GAP);
                            if button::wide(
                                ui,
                                icons::ADD,
                                "Create a new project",
                                Intent::Primary,
                                "Create a Sindri project and open it",
                            )
                            .clicked()
                            {
                                self.creating = Some(NewProject::default());
                                self.problem = None;
                            }
                            ui.add_space(metric::GAP);
                            if button::wide(
                                ui,
                                icons::FOLDER,
                                "Open an existing project",
                                Intent::Normal,
                                "Open a folder containing sindri.toml",
                            )
                            .clicked()
                            {
                                self.browse();
                            }

                            ui.add_space(22.0);
                            section_title(ui, "LEARN");
                            ui.add_space(metric::GAP);
                            for guide in &learn::GUIDES {
                                if learn::guide_row(ui, guide) {
                                    self.problem = learn::open(guide.page).err();
                                }
                            }

                            if !self.samples.is_empty() {
                                ui.add_space(22.0);
                                section_title(ui, "EXAMPLES");
                                ui.add_space(metric::GAP);
                                let mut opened = None;
                                for sample in &self.samples {
                                    if example_row(ui, sample) {
                                        opened = Some(sample.root.clone());
                                    }
                                }
                                if let Some(root) = opened {
                                    self.request = Some(Request::Open(root));
                                }
                            }
                        });
                    });
            });
    }

    fn main_panel(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(color::PANEL))
            .show(ui, |ui| {
                hero::draw(ui);
                panel::body(ui, |ui| {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Recent projects")
                                .size(16.0)
                                .strong()
                                .color(color::TEXT),
                        );
                        if !self.recent.is_empty() {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.search)
                                        .hint_text("Search projects…")
                                        .desired_width(240.0),
                                );
                            });
                        }
                    });
                    ui.add_space(metric::GROUP_GAP);
                    self.project_list(ui);
                });
            });
    }

    fn project_list(&mut self, ui: &mut egui::Ui) {
        let rows = self.rows();
        if rows.is_empty() && !self.recent.is_empty() {
            panel::empty_state(
                ui,
                icons::SEARCH,
                "No projects match",
                "Search looks at each project's name and folder.",
            );
            return;
        }
        if rows.is_empty() {
            panel::empty_state(
                ui,
                icons::PROJECT,
                "Your work will appear here",
                "Create your first project, or open an existing Sindri project.",
            );
            return;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let now = crate::project::now();
                for row in &rows {
                    match project_card(ui, row, now) {
                        Some(Clicked::Open) if row.present => {
                            self.request = Some(Request::Open(row.root.clone()));
                        }
                        Some(Clicked::Open) => {
                            self.problem =
                                Some(format!("{} is not there any more", row.root.display()));
                        }
                        Some(Clicked::Forget) => {
                            self.recent.forget(&row.root.display().to_string());
                            self.changed = true;
                        }
                        None => {}
                    }
                    ui.add_space(4.0);
                }
            });
    }

    fn browse(&mut self) {
        let Some(root) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        if crate::project::manifest::is_project(&root) {
            self.request = Some(Request::Open(root));
        } else {
            self.problem = Some(format!(
                "{} is not a Sindri project: it has no {}",
                root.display(),
                crate::project::MANIFEST_NAME
            ));
        }
    }
}

fn section_title(ui: &mut egui::Ui, label: &str) {
    ui.label(
        RichText::new(label)
            .size(text::NOTE)
            .strong()
            .color(color::TEXT_FAINT),
    );
}

fn example_row(ui: &mut egui::Ui, sample: &Sample) -> bool {
    let (rect, response) = button::row_sense(ui, EXAMPLE_HEIGHT);
    if response.hovered() {
        ui.painter().rect_filled(rect, radius(), color::EMBER_FAINT);
    }
    let painter = ui.painter_at(rect);
    let tile = egui::Rect::from_min_size(
        Pos2::new(rect.left() + 4.0, rect.top() + 5.0),
        Vec2::new(54.0, EXAMPLE_HEIGHT - 10.0),
    );
    paint_monogram(&painter, tile, &sample.name);
    let left = tile.right() + 12.0;
    painter.text(
        Pos2::new(left, rect.top() + 8.0),
        Align2::LEFT_TOP,
        &sample.name,
        egui::FontId::proportional(text::BODY),
        color::TEXT,
    );
    painter.text(
        Pos2::new(left, rect.bottom() - 8.0),
        Align2::LEFT_BOTTOM,
        sample.summary,
        egui::FontId::proportional(text::NOTE),
        color::TEXT_FAINT,
    );
    response
        .on_hover_text(sample.root.display().to_string())
        .clicked()
}

/// A stand-in thumbnail: the project's initial on a tile tinted by its name.
///
/// The editor has no image loading yet, so there are no screenshots to show.
/// The tint is derived from the name so a project keeps its colour between
/// launches and two projects side by side rarely share one.
fn paint_monogram(painter: &egui::Painter, tile: egui::Rect, name: &str) {
    let hash = name.bytes().fold(2_166_136_261_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
    });
    let hue = f32::from(u16::try_from(hash % 360).unwrap_or(0)) / 360.0;
    let fill: egui::Color32 = egui::ecolor::Hsva::new(hue, 0.45, 0.32, 1.0).into();
    let edge: egui::Color32 = egui::ecolor::Hsva::new(hue, 0.5, 0.55, 1.0).into();
    painter.rect_filled(tile, radius(), fill);
    painter.rect_stroke(
        tile,
        radius(),
        egui::Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    let initial: String = name.chars().next().into_iter().collect();
    painter.text(
        tile.center(),
        Align2::CENTER_CENTER,
        initial,
        egui::FontId::proportional(tile.height() * 0.55),
        egui::Color32::from_white_alpha(220),
    );
}

fn project_card(ui: &mut egui::Ui, listing: &Listing, now: Option<u64>) -> Option<Clicked> {
    let (rect, response) = button::row_sense(ui, CARD_HEIGHT);
    let fill = if response.hovered() {
        color::EMBER_FAINT
    } else {
        color::RAISED
    };
    ui.painter().rect_filled(rect, radius(), fill);
    outline(
        ui,
        rect,
        if response.hovered() {
            hairline()
        } else {
            hairline_soft()
        },
    );

    let named = if listing.present {
        color::TEXT
    } else {
        color::TEXT_FAINT
    };
    let painter = ui.painter_at(rect);
    let tile = egui::Rect::from_min_size(
        Pos2::new(rect.left() + 8.0, rect.top() + 8.0),
        Vec2::new(56.0, CARD_HEIGHT - 16.0),
    );
    paint_monogram(&painter, tile, &listing.name);
    let left = tile.right() + 12.0;
    let name = painter.layout_no_wrap(
        listing.name.clone(),
        egui::FontId::proportional(13.0),
        named,
    );
    painter.galley(Pos2::new(left, rect.top() + 9.0), name.clone(), named);
    if !listing.present {
        painter.text(
            Pos2::new(left + name.size().x + 8.0, rect.top() + 10.0),
            Align2::LEFT_TOP,
            "MISSING",
            egui::FontId::proportional(text::NOTE),
            color::DANGER_TEXT,
        );
    }
    painter.text(
        Pos2::new(left, rect.bottom() - 9.0),
        Align2::LEFT_BOTTOM,
        listing.root.display().to_string(),
        egui::FontId::proportional(text::NOTE),
        color::TEXT_FAINT,
    );

    if let (Some(opened), Some(now)) = (listing.opened, now) {
        painter.text(
            Pos2::new(rect.right() - 44.0, rect.center().y),
            Align2::RIGHT_CENTER,
            crate::project::ago(opened, now),
            egui::FontId::proportional(text::NOTE),
            color::TEXT_FAINT,
        );
    }

    let corner = egui::Rect::from_min_size(
        Pos2::new(rect.right() - 32.0, rect.center().y - 9.0),
        Vec2::splat(18.0),
    );
    let mut forget = false;
    ui.scope_builder(UiBuilder::new().max_rect(corner), |ui| {
        forget = button::row_icon(
            ui,
            icons::CLOSE,
            Intent::Quiet,
            "Remove from recent projects. Nothing on disk is touched.",
        )
        .clicked();
    });
    if forget {
        return Some(Clicked::Forget);
    }
    response.clicked().then_some(Clicked::Open)
}
