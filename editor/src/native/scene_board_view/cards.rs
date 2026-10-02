//! The board drawn: cards in rows, arrows between them, and a menu on each.

use std::path::{Path, PathBuf};

use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::project::{SceneBoard, SceneCard};
use crate::ui::icons;
use crate::ui::theme::{color, metric, text};
use crate::ui::widgets::panel;

use super::{BoardAction, ScenePictures};

/// How wide a card is drawn when there is room, and how narrow it may get.
const CARD_WIDTH: f32 = 220.0;
pub(super) const CARD_MIN_WIDTH: f32 = 150.0;
/// The space between cards, which is where arrows run.
const CARD_GAP: f32 = 36.0;
/// A picture's shape: the shape of most screens a game is played on.
const PICTURE_ASPECT: f32 = 9.0 / 16.0;

/// What the board needs from the editor that is not the board.
pub(super) struct BoardContext<'a> {
    pub(super) open: Option<&'a Path>,
    /// Scenes in the project that the board does not carry, which is what
    /// the Add menu offers.
    pub(super) unlisted: Vec<(PathBuf, String)>,
}

/// Lays the cards out in rows, draws them and the arrows between them, and
/// answers what was clicked.
pub(super) fn board_view(
    ui: &mut egui::Ui,
    board: &SceneBoard,
    pictures: &ScenePictures,
    context: &BoardContext<'_>,
) -> Option<BoardAction> {
    let mut action = None;
    board_tools(ui, board, context, &mut action);
    panel::rule_tight(ui);
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            panel::body(ui, |ui| {
                if board.cards.is_empty() {
                    panel::empty_state(
                        ui,
                        icons::SCENES,
                        "This project declares no scenes",
                        "Add one above, or set a scene as the main scene from the project browser.",
                    );
                    return;
                }
                let width = ui.available_width().max(CARD_MIN_WIDTH);
                // At least one, and the division is of two positive widths.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let columns = (((width + CARD_GAP) / (CARD_WIDTH + CARD_GAP)).floor() as usize)
                    .clamp(1, board.cards.len());
                #[allow(clippy::cast_precision_loss)]
                let card_width = ((width - CARD_GAP * (columns - 1) as f32) / columns as f32)
                    .clamp(CARD_MIN_WIDTH, CARD_WIDTH * 1.4);
                let card_height = card_height(card_width);
                let origin = ui.cursor().min;
                #[allow(clippy::cast_precision_loss)]
                let rect_of = |index: usize| {
                    let (row, column) = (index / columns, index % columns);
                    Rect::from_min_size(
                        origin
                            + Vec2::new(
                                column as f32 * (card_width + CARD_GAP),
                                row as f32 * (card_height + CARD_GAP),
                            ),
                        Vec2::new(card_width, card_height),
                    )
                };
                let rows = board.cards.len().div_ceil(columns);
                #[allow(clippy::cast_precision_loss)]
                let total = Vec2::new(
                    width,
                    rows as f32 * card_height + (rows - 1) as f32 * CARD_GAP,
                );
                ui.allocate_rect(Rect::from_min_size(origin, total), Sense::hover());
                let listed = board.cards.iter().filter(|card| !card.main).count();
                for (index, card) in board.cards.iter().enumerate() {
                    let rect = rect_of(index);
                    if let Some(chosen) = card_view(
                        ui,
                        rect,
                        card,
                        pictures.get(&card.path),
                        context.open == Some(card.path.as_path()),
                        context,
                        listed,
                        board
                            .cards
                            .iter()
                            .filter(|other| !other.main)
                            .position(|other| other.path == card.path),
                    ) {
                        action = Some(chosen);
                    }
                }
                for link in &board.links {
                    let both = board
                        .links
                        .iter()
                        .any(|back| back.from == link.to && back.to == link.from);
                    arrow(ui.painter(), rect_of(link.from), rect_of(link.to), both);
                }
            });
        });
    action
}

/// A card's height for its width: the picture, then two lines of words.
fn card_height(width: f32) -> f32 {
    width * PICTURE_ASPECT + 46.0
}

fn board_tools(
    ui: &mut egui::Ui,
    board: &SceneBoard,
    context: &BoardContext<'_>,
    action: &mut Option<BoardAction>,
) {
    ui.horizontal(|ui| {
        ui.set_height(metric::TOOLBAR_HEIGHT);
        ui.add_space(metric::GUTTER);
        let doors = board.links.len();
        ui.label(
            RichText::new(format!(
                "{} scene{} · {doors} door{}",
                board.cards.len(),
                if board.cards.len() == 1 { "" } else { "s" },
                if doors == 1 { "" } else { "s" },
            ))
            .size(text::LABEL)
            .color(color::TEXT_MUTED),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(metric::GUTTER);
            ui.add_enabled_ui(!context.unlisted.is_empty(), |ui| {
                ui.menu_button(
                    RichText::new(format!("{} Add scene", icons::ADD.codepoint)).size(text::LABEL),
                    |ui| {
                        for (path, relative) in &context.unlisted {
                            if ui.button(relative.as_str()).clicked() {
                                *action = Some(BoardAction::Add(path.clone()));
                                ui.close();
                            }
                        }
                    },
                )
                .response
                .on_disabled_hover_text("Every scene in the project is on the board")
                .on_hover_text("Carry another of the project's scenes in its builds");
            });
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn card_view(
    ui: &mut egui::Ui,
    rect: Rect,
    card: &SceneCard,
    picture: Option<(egui::TextureId, [u32; 2])>,
    open: bool,
    context: &BoardContext<'_>,
    listed: usize,
    position: Option<usize>,
) -> Option<BoardAction> {
    let mut action = None;
    let response = ui.interact(
        rect,
        ui.id().with(("scene card", &card.path)),
        Sense::click(),
    );
    let painter = ui.painter_at(rect.expand(2.0));
    let outline = if open {
        Stroke::new(metric::SELECT_RULE, color::FORGE)
    } else if response.hovered() {
        Stroke::new(1.0, color::FORGE_DIM)
    } else {
        Stroke::new(1.0, color::LINE)
    };
    painter.rect_filled(rect, metric::RADIUS, color::RAISED);
    let picture_rect = Rect::from_min_size(
        rect.min,
        Vec2::new(rect.width(), rect.width() * PICTURE_ASPECT),
    );
    paint_picture(&painter, picture_rect, card, picture);
    let name_at = Pos2::new(rect.left() + 8.0, picture_rect.bottom() + 13.0);
    let mut name = card.name.clone();
    if card.main {
        name = format!("{} {name}", icons::MAIN_SCENE.codepoint);
    }
    painter.text(
        name_at,
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(text::BODY),
        if open {
            color::FORGE_BRIGHT
        } else {
            color::TEXT
        },
    );
    let (note, tint) = if let Some(stray) = card.strays.first() {
        (
            format!("Goes to {stray}, which the project does not carry"),
            color::WARNING,
        )
    } else if card.main {
        ("Opens first".to_owned(), color::TEXT_FAINT)
    } else if open {
        ("Open".to_owned(), color::TEXT_FAINT)
    } else {
        ("Click to open".to_owned(), color::TEXT_FAINT)
    };
    painter.text(
        name_at + Vec2::new(0.0, 17.0),
        Align2::LEFT_CENTER,
        note,
        FontId::proportional(text::NOTE),
        tint,
    );
    painter.rect_stroke(rect, metric::RADIUS, outline, egui::StrokeKind::Inside);

    let hover = if card.strays.is_empty() {
        card.path.display().to_string()
    } else {
        format!(
            "{}\nIts scripts go to {}, which the project does not carry: a build would have \
             nowhere to go. Add the scene to the project, or fix the name.",
            card.path.display(),
            card.strays.join(", ")
        )
    };
    let response = response.on_hover_text(hover);
    if response.clicked() && !card.missing {
        action = Some(BoardAction::Open(card.path.clone()));
    }
    response.context_menu(|ui| {
        if let Some(chosen) = card_menu(ui, card, open, context, listed, position) {
            action = Some(chosen);
        }
    });
    action
}

/// A card's picture: the scene's last frame, or what it holds when it has
/// none, or why it cannot be shown.
fn paint_picture(
    painter: &egui::Painter,
    picture_rect: Rect,
    card: &SceneCard,
    picture: Option<(egui::TextureId, [u32; 2])>,
) {
    painter.rect_filled(picture_rect, metric::RADIUS, color::WELL);
    match picture {
        Some((id, size)) if !card.missing => {
            painter.image(
                id,
                picture_rect,
                cover(size, picture_rect.aspect_ratio()),
                Color32::WHITE,
            );
        }
        _ => {
            let (glyph, words, tint) = if card.missing {
                (
                    icons::STRAY,
                    "The file is missing".to_owned(),
                    color::DANGER_TEXT,
                )
            } else if let Some(problem) = &card.problem {
                (
                    icons::STRAY,
                    format!("Unreadable: {problem}"),
                    color::DANGER_TEXT,
                )
            } else {
                (
                    icons::SCENE,
                    format!(
                        "{} entit{} · open it for a picture",
                        card.entities,
                        if card.entities == 1 { "y" } else { "ies" }
                    ),
                    color::TEXT_FAINT,
                )
            };
            painter.text(
                picture_rect.center() - Vec2::new(0.0, 8.0),
                Align2::CENTER_CENTER,
                glyph.codepoint,
                FontId::proportional(22.0),
                tint.gamma_multiply(0.8),
            );
            painter.text(
                picture_rect.center() + Vec2::new(0.0, 14.0),
                Align2::CENTER_CENTER,
                words,
                FontId::proportional(text::NOTE),
                tint,
            );
        }
    }
}

/// What a card's menu offers, and what was chosen from it.
fn card_menu(
    ui: &mut egui::Ui,
    card: &SceneCard,
    open: bool,
    context: &BoardContext<'_>,
    listed: usize,
    position: Option<usize>,
) -> Option<BoardAction> {
    let mut action = None;
    if ui
        .add_enabled(!card.missing && !open, egui::Button::new("Open"))
        .clicked()
    {
        action = Some(BoardAction::Open(card.path.clone()));
        ui.close();
    }
    if ui
        .add_enabled(
            !card.main && !card.missing,
            egui::Button::new("Set as main scene"),
        )
        .clicked()
    {
        action = Some(BoardAction::SetMain(card.path.clone()));
        ui.close();
    }
    if let Some(at) = position {
        if ui
            .add_enabled(at > 0, egui::Button::new("Move earlier"))
            .clicked()
        {
            action = Some(BoardAction::Move(card.path.clone(), at - 1));
            ui.close();
        }
        if ui
            .add_enabled(at + 1 < listed, egui::Button::new("Move later"))
            .clicked()
        {
            action = Some(BoardAction::Move(card.path.clone(), at + 1));
            ui.close();
        }
    }
    for stray in &card.strays {
        if let Some((path, _)) = context.unlisted.iter().find(|(path, _)| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy() == stray.as_str())
        }) && ui.button(format!("Add {stray} to the project")).clicked()
        {
            action = Some(BoardAction::Add(path.clone()));
            ui.close();
        }
    }
    if ui
        .add_enabled(!card.missing, egui::Button::new("Show in project browser"))
        .clicked()
    {
        action = Some(BoardAction::ShowInProject(card.path.clone()));
        ui.close();
    }
    ui.separator();
    if ui
        .add_enabled(!card.main, egui::Button::new("Remove from project"))
        .on_disabled_hover_text("Set another scene as the main scene first")
        .on_hover_text("The project stops carrying this scene; its file stays")
        .clicked()
    {
        action = Some(BoardAction::Remove(card.path.clone()));
        ui.close();
    }
    action
}

/// The part of a picture that fills a frame of another shape without
/// stretching, cut from its middle.
fn cover(size: [u32; 2], frame_aspect: f32) -> Rect {
    #[allow(clippy::cast_precision_loss)]
    let aspect = size[0] as f32 / size[1].max(1) as f32;
    if aspect > frame_aspect {
        let keep = frame_aspect / aspect;
        Rect::from_min_max(
            Pos2::new((1.0 - keep) / 2.0, 0.0),
            Pos2::new(f32::midpoint(1.0, keep), 1.0),
        )
    } else {
        let keep = aspect / frame_aspect;
        Rect::from_min_max(
            Pos2::new(0.0, (1.0 - keep) / 2.0),
            Pos2::new(1.0, f32::midpoint(1.0, keep)),
        )
    }
}

/// A door from one card to another: from edge to edge, with its head at the
/// scene it opens onto. A pair of doors both ways is drawn as two arrows side
/// by side rather than one line with two heads, so each can be told apart.
fn arrow(painter: &egui::Painter, from: Rect, to: Rect, both: bool) {
    let direction = (to.center() - from.center()).normalized();
    if !direction.is_finite() {
        return;
    }
    let side = Vec2::new(-direction.y, direction.x) * if both { 6.0 } else { 0.0 };
    let start = edge(from, direction) + side;
    let end = edge(to, -direction) + side;
    let stroke = Stroke::new(1.6, color::FORGE);
    painter.line_segment([start, end], stroke);
    let head = 8.0;
    let back = end - direction * head;
    let across = Vec2::new(-direction.y, direction.x) * head * 0.5;
    painter.add(egui::Shape::convex_polygon(
        vec![end, back + across, back - across],
        color::FORGE,
        Stroke::NONE,
    ));
}

/// Where a ray from a rect's centre in `direction` leaves it.
fn edge(rect: Rect, direction: Vec2) -> Pos2 {
    let half = rect.size() / 2.0;
    let x = if direction.x.abs() > f32::EPSILON {
        half.x / direction.x.abs()
    } else {
        f32::INFINITY
    };
    let y = if direction.y.abs() > f32::EPSILON {
        half.y / direction.y.abs()
    } else {
        f32::INFINITY
    };
    rect.center() + direction * x.min(y)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use eframe::egui::{self, Pos2, Rect, Vec2};

    use super::super::{BoardAction, ScenePictures};
    use super::{BoardContext, board_view, cover, edge};
    use crate::project::{SceneBoard, SceneCard, SceneLink};

    fn card(name: &str, main: bool) -> SceneCard {
        SceneCard {
            path: PathBuf::from("/game/assets").join(name),
            name: name.to_owned(),
            main,
            missing: false,
            problem: None,
            strays: Vec::new(),
            entities: 3,
        }
    }

    /// Drives the board through real frames, pressing at `at` (a fraction of
    /// the panel), and reports what it asked for.
    fn clicked(board: &SceneBoard, at: Vec2) -> Option<BoardAction> {
        let context = egui::Context::default();
        egui_material_icons::initialize(&context);
        let pictures = ScenePictures::default();
        let open = board.cards[0].path.clone();
        let acted = std::cell::RefCell::new(None);
        let size = Vec2::new(900.0, 600.0);
        let draw = |events: Vec<egui::Event>| {
            context
                .run_ui(
                    egui::RawInput {
                        events,
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |ui| {
                        let chosen = board_view(
                            ui,
                            board,
                            &pictures,
                            &BoardContext {
                                open: Some(&open),
                                unlisted: Vec::new(),
                            },
                        );
                        if let Some(chosen) = chosen {
                            *acted.borrow_mut() = Some(chosen);
                        }
                    },
                )
                .drop_without_applying_deltas();
        };
        draw(Vec::new());
        let target = Pos2::new(size.x * at.x, size.y * at.y);
        let button = |pressed| egui::Event::PointerButton {
            pos: target,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        draw(vec![egui::Event::PointerMoved(target), button(true)]);
        draw(vec![button(false)]);
        acted.into_inner()
    }

    /// Cards sit in a row across a wide panel, and a click on one opens its
    /// scene; a click between them opens nothing.
    #[test]
    fn clicking_a_card_opens_its_scene() {
        let board = SceneBoard {
            cards: vec![card("title.scene", true), card("level.scene", false)],
            links: vec![SceneLink { from: 0, to: 1 }],
        };
        assert_eq!(
            clicked(&board, Vec2::new(0.4, 0.25)),
            Some(BoardAction::Open(board.cards[1].path.clone()))
        );
        assert_eq!(clicked(&board, Vec2::new(0.9, 0.9)), None);
    }

    #[test]
    fn a_ray_leaves_a_card_at_its_edge() {
        let card = Rect::from_center_size(Pos2::ZERO, Vec2::new(200.0, 100.0));
        assert_eq!(edge(card, Vec2::new(1.0, 0.0)), Pos2::new(100.0, 0.0));
        assert_eq!(edge(card, Vec2::new(0.0, -1.0)), Pos2::new(0.0, -50.0));
    }

    #[test]
    fn a_wide_picture_is_cut_from_its_middle_to_fill_a_narrower_frame() {
        let cut = cover([1600, 900], 1.0);
        assert!((cut.width() - 0.5625).abs() < 1.0e-4 && (cut.height() - 1.0).abs() < 1.0e-6);
        assert!((cut.center().x - 0.5).abs() < 1.0e-4);
        let tall = cover([900, 1600], 16.0 / 9.0);
        assert!((tall.width() - 1.0).abs() < 1.0e-6 && tall.height() < 0.4);
    }
}
