//! What the Assistant panel looks like in each state.
//!
//! Every state is one card with one thing to do, and every word is written for
//! someone who has never heard of a model runner: what the assistant does, what
//! it costs, what is happening now, and what to do if something went wrong.

use std::fmt::Write as _;
use std::time::Duration;

use eframe::egui::{self, RichText};
use egui_material_icons::MaterialIcon;

use crate::assistant::install::Progress;
use crate::assistant::{Feature, Offer, Setup, Step, Tier, setup, size};
use crate::ui::icons;
use crate::ui::theme::{color, metric, text};
use crate::ui::widgets::button::{self, Intent};

use super::{EditorApp, Shown, Speed};

/// How a step is drawn in the list.
#[derive(Clone, Copy, PartialEq)]
enum Mark {
    Done,
    Active,
    Waiting,
    Failed,
}

impl EditorApp {
    pub(super) fn assistant_card(&mut self, ui: &mut egui::Ui) {
        ui.add_space(14.0);
        if self.setup_in_progress(ui) {
            return;
        }
        let Some(machine) = self.assistant.machine.clone() else {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.add(egui::Spinner::new().color(color::FORGE));
                ui.add_space(8.0);
                ui.label(muted("Looking at this computer…"));
            });
            return;
        };
        if let Some(failure) = &self.assistant.failure {
            hero(
                ui,
                "Setup did not finish",
                "Nothing is broken; it can pick up where it stopped.",
            );
            let again = card(ui, |ui| failed(ui, failure));
            if again {
                self.begin_setup();
            }
            return;
        }
        self.state_card(ui, setup(&machine));
    }

    /// The steps while setup runs; whether it is running.
    fn setup_in_progress(&mut self, ui: &mut egui::Ui) -> bool {
        let Some(job) = &mut self.assistant.job else {
            return false;
        };
        {
            let progress = job.shared.progress();
            job.speed.sample(progress.arrived);
            hero(
                ui,
                "Setting up your assistant",
                "You can keep working while this runs.",
            );
            let stop = card(ui, |ui| {
                working(ui, &progress, &job.speed);
                ui.add_space(10.0);
                button::labelled(
                    ui,
                    "Stop",
                    Intent::Quiet,
                    "Stop setting up. Nothing half-done is kept in use.",
                )
                .clicked()
            });
            if stop {
                job.shared.stop();
            }
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
        true
    }

    /// The card for a settled state.
    fn state_card(&mut self, ui: &mut egui::Ui, state: Setup) {
        match state {
            Setup::Unavailable => {
                hero(
                    ui,
                    "Not available on this computer yet",
                    "Sindri's local assistant is published for Linux (64-bit Intel and AMD) and \
                     for Macs with Apple silicon so far.",
                );
            }
            Setup::Offer(offer) => {
                hero(
                    ui,
                    "Fix broken scripts with a local AI",
                    "An assistant that runs on your own computer and proposes fixes for Decay \
                     scripts that do not compile.",
                );
                if card(ui, |ui| offered(ui, &offer)) {
                    self.begin_setup();
                }
            }
            Setup::Unchecked => {
                hero(
                    ui,
                    "Almost done",
                    "Everything is downloaded. One last check and it is ready.",
                );
                if card(ui, |ui| {
                    button::wide(
                        ui,
                        icons::ASSISTANT,
                        "Finish setting up",
                        Intent::Primary,
                        "Start the model and check it",
                    )
                    .clicked()
                }) {
                    self.begin_setup();
                }
            }
            Setup::Ready { verified } => self.ready_card(ui, &verified),
            Setup::Unusable => {
                hero(
                    ui,
                    "The model did not pass",
                    "It could not fix Sindri's two test scripts, so the editor will not offer its \
                     fixes. Checking again sometimes helps.",
                );
                if card(ui, |ui| {
                    button::wide(
                        ui,
                        icons::REFRESH,
                        "Check again",
                        Intent::Primary,
                        "Run the check again",
                    )
                    .clicked()
                }) {
                    self.begin_setup();
                }
                self.remove_row(ui);
            }
        }
    }

    fn ready_card(&mut self, ui: &mut egui::Ui, verified: &[Feature]) {
        hero(
            ui,
            "Your assistant is ready",
            "Open a script that does not compile from the project browser, and press \
             Propose a fix under its errors.",
        );
        let running = self.assistant.running();
        let (name, footprint) = self.assistant.machine.as_ref().map_or_else(
            || (String::new(), 0),
            |machine| {
                (
                    machine
                        .model
                        .as_ref()
                        .map(|model| model.display_name.clone())
                        .unwrap_or_default(),
                    machine.footprint,
                )
            },
        );
        let stop = card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&name).size(text::BODY).color(color::TEXT));
                ui.label(faint(&format!("· {} on this computer", size(footprint))));
            });
            ui.add_space(8.0);
            for feature in Feature::ALL.into_iter().filter(|feature| feature.tested()) {
                let proved = verified.contains(&feature);
                step_row(
                    ui,
                    if proved { Mark::Done } else { Mark::Failed },
                    feature.label(),
                    None,
                );
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                dot(
                    ui,
                    if running {
                        color::SUCCESS
                    } else {
                        color::TEXT_FAINT
                    },
                );
                ui.label(muted(if running {
                    "Running — using memory until you stop it or close Sindri"
                } else {
                    "Resting — starts by itself when you ask for a fix"
                }));
            });
            running
                && button::labelled(
                    ui,
                    "Stop the model",
                    Intent::Quiet,
                    "Free the memory it is using",
                )
                .clicked()
        });
        if stop {
            self.stop_model();
        }
        self.remove_row(ui);
    }

    /// The quiet way out, with a confirmation, because it deletes gigabytes.
    fn remove_row(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        inset(ui, |ui| {
            if self.assistant.confirm_remove {
                ui.label(muted(
                    "Remove the assistant and its model from this computer? You can set it \
                     up again any time.",
                ));
                ui.horizontal(|ui| {
                    if button::labelled(
                        ui,
                        "Remove",
                        Intent::Danger,
                        "Delete the assistant's files",
                    )
                    .clicked()
                    {
                        self.remove_assistant();
                    }
                    if button::labelled(ui, "Keep it", Intent::Quiet, "Leave everything as it is")
                        .clicked()
                    {
                        self.assistant.confirm_remove = false;
                    }
                });
            } else if button::labelled(
                ui,
                "Remove from this computer",
                Intent::Quiet,
                "Delete the model and runner to free disk space",
            )
            .clicked()
            {
                self.assistant.confirm_remove = true;
            }
        });
    }
}

/// The offer: what it does, what it costs, whether it fits. Returns whether
/// the button was pressed.
fn offered(ui: &mut egui::Ui, offer: &Offer) -> bool {
    benefit(
        ui,
        icons::SCRIPT,
        "When a script will not compile, it proposes a fix. You see the change and decide; nothing is written until you accept.",
    );
    benefit(
        ui,
        icons::PRIVATE,
        "Runs entirely on your computer. Nothing you write is sent anywhere.",
    );
    benefit(
        ui,
        icons::DOWNLOAD,
        &format!(
            "{} to download, once. It is kept in Sindri's own folder and can be removed any time.",
            size(offer.download)
        ),
    );
    let (tint, fit) = match offer.fit {
        Some(Tier::Recommended) => (color::SUCCESS, "Runs well on this computer."),
        Some(Tier::Supported) => (
            color::SUCCESS,
            "Fits on this computer, with a little room to spare.",
        ),
        Some(Tier::BestEffort) => (color::WARNING, "Fits on this computer, but will be slow."),
        Some(Tier::Unsupported) => (
            color::WARNING,
            "This computer may not have enough memory for it. It could be very slow or fail to start.",
        ),
        None => (
            color::TEXT_FAINT,
            "Sindri could not tell how much memory this computer has.",
        ),
    };
    ui.horizontal(|ui| {
        ui.label(icons::MEMORY.outlined().rich_text().size(15.0).color(tint));
        ui.add(egui::Label::new(muted(fit)).wrap());
    });
    ui.add_space(12.0);
    let label = if offer.partly_done {
        "Continue setting up"
    } else {
        "Set up the assistant"
    };
    button::wide(
        ui,
        icons::DOWNLOAD,
        label,
        Intent::Primary,
        &format!(
            "Downloads {} and sets up {}",
            size(offer.download),
            offer.model_name
        ),
    )
    .clicked()
}

/// The steps while setup runs, with the current one's progress.
fn working(ui: &mut egui::Ui, progress: &Progress, speed: &Speed) {
    let current = progress.step.unwrap_or(Step::Runner);
    for step in Step::ALL {
        let mark = match step.cmp(&current) {
            std::cmp::Ordering::Less => Mark::Done,
            std::cmp::Ordering::Equal => Mark::Active,
            std::cmp::Ordering::Greater => Mark::Waiting,
        };
        let detail = (mark == Mark::Active).then(|| step.doing());
        step_row(ui, mark, step.title(), detail);
        if mark == Mark::Active && progress.total > 0 {
            downloading(ui, progress, speed);
        }
    }
}

fn downloading(ui: &mut egui::Ui, progress: &Progress, speed: &Speed) {
    #[allow(clippy::cast_precision_loss)] // a fraction for a bar
    let fraction = progress.arrived as f32 / progress.total.max(1) as f32;
    ui.horizontal(|ui| {
        ui.add_space(26.0);
        ui.vertical(|ui| {
            ui.add(
                egui::ProgressBar::new(fraction.clamp(0.0, 1.0))
                    .desired_height(6.0)
                    .fill(color::FORGE),
            );
            ui.add_space(3.0);
            let said = if progress.checking {
                "Checking the download is exactly the file Sindri expects…".to_owned()
            } else {
                let mut said = format!("{} of {}", size(progress.arrived), size(progress.total));
                if speed.rate > 0.0 {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let per_second = speed.rate as u64;
                    let left = progress.total.saturating_sub(progress.arrived) / per_second.max(1);
                    let _ = write!(said, " · {}/s · {}", size(per_second), remaining(left));
                }
                said
            };
            ui.label(faint(&said));
        });
    });
    ui.add_space(4.0);
}

fn remaining(seconds: u64) -> String {
    match seconds {
        0..60 => "under a minute left".to_owned(),
        60..5400 => format!("about {} min left", seconds.div_ceil(60)),
        _ => format!("about {} hours left", seconds.div_ceil(3600)),
    }
}

/// A failed setup: which step, what to do, and the runner's own words when it
/// gave any. Returns whether "Try again" was pressed.
fn failed(ui: &mut egui::Ui, failure: &Shown) -> bool {
    for step in Step::ALL {
        let mark = match step.cmp(&failure.step) {
            std::cmp::Ordering::Less => Mark::Done,
            std::cmp::Ordering::Equal => Mark::Failed,
            std::cmp::Ordering::Greater => Mark::Waiting,
        };
        step_row(
            ui,
            mark,
            step.title(),
            (mark == Mark::Failed).then_some(failure.message.as_str()),
        );
    }
    if let Some(details) = &failure.details {
        ui.add_space(4.0);
        egui::CollapsingHeader::new(faint("Details"))
            .id_salt("assistant-failure-details")
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(details.trim())
                                .font(egui::FontId::monospace(text::NOTE))
                                .color(color::TEXT_FAINT),
                        );
                    });
            });
    }
    ui.add_space(10.0);
    button::wide(
        ui,
        icons::REFRESH,
        "Try again",
        Intent::Primary,
        "Pick up where it stopped",
    )
    .clicked()
}

/// One step: its mark, its title, and while it matters, a line about it.
fn step_row(ui: &mut egui::Ui, mark: Mark, title: &str, detail: Option<&str>) {
    ui.horizontal(|ui| {
        let size = egui::vec2(18.0, 18.0);
        match mark {
            Mark::Active => {
                ui.add_sized(size, egui::Spinner::new().size(14.0).color(color::FORGE));
            }
            Mark::Done => glyph(ui, icons::DONE, color::SUCCESS),
            Mark::Waiting => glyph(ui, icons::PENDING, color::LINE.gamma_multiply(1.6)),
            Mark::Failed => glyph(ui, icons::FAILED, color::DANGER),
        }
        ui.label(RichText::new(title).size(text::LABEL).color(match mark {
            Mark::Active | Mark::Failed => color::TEXT,
            Mark::Done => color::TEXT_MUTED,
            Mark::Waiting => color::TEXT_FAINT,
        }));
    });
    if let Some(detail) = detail {
        ui.horizontal(|ui| {
            ui.add_space(26.0);
            ui.add(
                egui::Label::new(RichText::new(detail).size(text::NOTE).color(
                    if mark == Mark::Failed {
                        color::DANGER_TEXT
                    } else {
                        color::TEXT_FAINT
                    },
                ))
                .wrap(),
            );
        });
    }
    ui.add_space(5.0);
}

/// The icon, headline and one line that open every state.
fn hero(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    let width = (ui.available_width() - 2.0 * metric::GUTTER).max(120.0);
    ui.vertical_centered(|ui| {
        ui.set_max_width(width);
        ui.label(
            icons::ASSISTANT
                .outlined()
                .rich_text()
                .size(30.0)
                .color(color::FORGE),
        );
        ui.add_space(6.0);
        ui.label(RichText::new(title).size(text::HEADING).color(color::TEXT));
        ui.add_space(4.0);
        ui.add(egui::Label::new(muted(subtitle)).wrap());
    });
    ui.add_space(14.0);
}

/// A raised card holding one state's content.
fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    inset(ui, |ui| {
        egui::Frame::new()
            .fill(color::RAISED)
            .stroke(egui::Stroke::new(1.0, color::LINE_SOFT))
            .corner_radius(8.0)
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                add(ui)
            })
            .inner
    })
}

fn inset<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.add_space(metric::GUTTER);
        let width = ui.available_width() - metric::GUTTER;
        ui.vertical(|ui| {
            ui.set_width(width);
            add(ui)
        })
        .inner
    })
    .inner
}

fn benefit(ui: &mut egui::Ui, icon: MaterialIcon, said: &str) {
    ui.horizontal_top(|ui| {
        ui.label(
            icon.outlined()
                .rich_text()
                .size(16.0)
                .color(color::FORGE_DIM.gamma_multiply(1.6)),
        );
        ui.add(
            egui::Label::new(
                RichText::new(said)
                    .size(text::LABEL)
                    .color(color::TEXT_MUTED),
            )
            .wrap(),
        );
    });
    ui.add_space(8.0);
}

fn glyph(ui: &mut egui::Ui, icon: MaterialIcon, tint: egui::Color32) {
    ui.add_sized(
        egui::vec2(18.0, 18.0),
        egui::Label::new(icon.outlined().rich_text().size(16.0).color(tint)),
    );
}

fn dot(ui: &mut egui::Ui, tint: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.5, tint);
}

fn muted(said: &str) -> RichText {
    RichText::new(said)
        .size(text::NOTE)
        .color(color::TEXT_MUTED)
}

fn faint(said: &str) -> RichText {
    RichText::new(said)
        .size(text::NOTE)
        .color(color::TEXT_FAINT)
}
