//! Seeing what the picture does not show: colliders and laid-out UI.
//!
//! A shot shows what a player sees, and a player does not see why the hero
//! fell through the ledge or why a click missed the button. These draw what
//! decides it over the picture: every 2D collider piece as physics is given
//! it (green, a sensor yellow), and every laid-out screen element as the
//! interface placed it (magenta, one that takes presses cyan). And `probe`
//! answers for one pixel: which element is there, which world point, and
//! which colliders hold it.
//!
//! Drawn on the pixels after the frame is read back, from the same camera
//! the frame was drawn through (`world_camera_for_viewport`) and the same
//! rects a click is aimed by, so an outline is exactly where the engine
//! thinks the thing is.

use glam::{Vec3, Vec4};
use serde::Serialize;
use sindri_capture::ProjectPlayer;
use sindri_core::{EntityId, World};
use sindri_scene::{
    CameraView, Collider2d, Collider2dComponent, ColliderShape2d, TilemapCollider2dComponent,
    TilemapComponent,
};

mod solid;

/// Which overlays later pictures carry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Layers {
    pub colliders: bool,
    pub ui: bool,
}

impl Layers {
    /// `debug:`'s argument: `colliders`, `ui`, both with a comma, `all`, or
    /// `off`.
    ///
    /// # Errors
    /// A word that is none of those.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut layers = Self::default();
        for word in text.split(',').map(str::trim) {
            match word {
                "colliders" => layers.colliders = true,
                "ui" => layers.ui = true,
                "all" => {
                    layers = Self {
                        colliders: true,
                        ui: true,
                    }
                }
                "off" => layers = Self::default(),
                other => {
                    return Err(format!(
                        "debug:{text}: {other} is not a layer; debug takes colliders, ui, all or off"
                    ));
                }
            }
        }
        Ok(layers)
    }

    #[must_use]
    pub const fn any(self) -> bool {
        self.colliders || self.ui
    }
}

const COLLIDER: [u8; 4] = [64, 255, 96, 255];
const SENSOR: [u8; 4] = [255, 220, 48, 255];
const ELEMENT: [u8; 4] = [255, 64, 220, 255];
const PRESSABLE: [u8; 4] = [48, 220, 255, 255];

/// Draws `layers` over `pixels`, a frame of `size` RGBA pixels.
pub fn draw(pixels: &mut [u8], size: [u32; 2], player: &ProjectPlayer, layers: Layers) {
    let mut canvas = Canvas { pixels, size };
    if layers.ui {
        let ui = player.session.screen_ui();
        for (entity, _) in player.world.entities() {
            let Some(rect) = ui.rect(entity) else {
                continue;
            };
            let to_pixel = |x: f32, y: f32| overlay_pixel([x, y], player.size);
            let [cx, cy] = rect.center;
            let [hw, hh] = [rect.size[0] / 2.0, rect.size[1] / 2.0];
            let corners = [
                to_pixel(cx - hw, cy - hh),
                to_pixel(cx + hw, cy - hh),
                to_pixel(cx + hw, cy + hh),
                to_pixel(cx - hw, cy + hh),
            ];
            let pressable = ui.pressable_at(rect.center) == Some(entity);
            canvas.polygon(&corners, if pressable { PRESSABLE } else { ELEMENT });
        }
    }
    if layers.colliders {
        for outline in outlines(player) {
            let points: Option<Vec<[f32; 2]>> = outline
                .points
                .iter()
                .map(|&[x, y]| project(player, Vec3::new(x, y, outline.z)))
                .collect();
            if let Some(points) = points {
                canvas.polygon(&points, if outline.sensor { SENSOR } else { COLLIDER });
            }
        }
        for (sensor, lines) in solid::wireframes(player) {
            for line in lines {
                let points: Vec<Option<[f32; 2]>> =
                    line.iter().map(|point| project(player, *point)).collect();
                for pair in points.windows(2) {
                    if let [Some(from), Some(to)] = pair {
                        canvas.line(*from, *to, if sensor { SENSOR } else { COLLIDER });
                    }
                }
            }
        }
    }
}

/// One collider piece, in world units, on the layer its entity is on.
struct Outline {
    entity: EntityId,
    sensor: bool,
    points: Vec<[f32; 2]>,
    z: f32,
}

/// Every 2D collider piece on an active entity, placed where physics places
/// it: the entity's world position and turn, then the piece's own.
fn outlines(player: &ProjectPlayer) -> Vec<Outline> {
    let registry = player.scene.components();
    let mut found = Vec::new();
    for (entity, _) in player.world.entities() {
        let Some(place) = player.world.world_transform(entity) else {
            continue;
        };
        // Its own pieces, and a tilemap's solid tiles as physics merges them.
        let mut pieces: Vec<Collider2d> = registry
            .get_parts::<Collider2dComponent>(&player.world, entity)
            .unwrap_or_default()
            .into_iter()
            .flat_map(|(_, Collider2dComponent(pieces))| pieces)
            .collect();
        if let (Ok(Some(collider)), Ok(Some(tilemap))) = (
            registry.get::<TilemapCollider2dComponent>(&player.world, entity),
            registry.get::<TilemapComponent>(&player.world, entity),
        ) && let Ok(tiles) = collider.pieces(&tilemap, place.scale_2d())
        {
            pieces.extend(tiles);
        }
        let turn = 2.0 * place.rotation[2].atan2(place.rotation[3]);
        let at = [place.position[0], place.position[1]];
        {
            for piece in pieces {
                let local = shape(&piece.shape);
                let points = local
                    .into_iter()
                    .map(|point| {
                        let point = rotate(point, piece.rotation);
                        let point = [point[0] + piece.offset[0], point[1] + piece.offset[1]];
                        let point = rotate(point, turn);
                        [point[0] + at[0], point[1] + at[1]]
                    })
                    .collect();
                found.push(Outline {
                    entity,
                    sensor: piece.sensor,
                    points,
                    z: place.position[2],
                });
            }
        }
    }
    found
}

/// A shape's outline around its own middle.
fn shape(shape: &ColliderShape2d) -> Vec<[f32; 2]> {
    use std::f32::consts::{PI, TAU};
    const ROUND: usize = 24;
    let arc = |middle: [f32; 2], radius: f32, from: f32, to: f32, steps: usize| {
        (0..=steps).map(move |step| {
            #[allow(clippy::cast_precision_loss)]
            let angle = from + (to - from) * step as f32 / steps as f32;
            [
                middle[0] + radius * angle.cos(),
                middle[1] + radius * angle.sin(),
            ]
        })
    };
    match *shape {
        ColliderShape2d::Box {
            half_extents: [w, h],
        } => vec![[-w, -h], [w, -h], [w, h], [-w, h]],
        ColliderShape2d::Circle { radius } => arc([0.0, 0.0], radius, 0.0, TAU, ROUND).collect(),
        // Upright, as physics makes one: two half circles joined.
        ColliderShape2d::Capsule {
            half_height,
            radius,
        } => arc([0.0, half_height], radius, 0.0, PI, ROUND / 2)
            .chain(arc([0.0, -half_height], radius, PI, TAU, ROUND / 2))
            .collect(),
    }
}

fn rotate([x, y]: [f32; 2], angle: f32) -> [f32; 2] {
    let (sin, cos) = angle.sin_cos();
    [x * cos - y * sin, x * sin + y * cos]
}

/// A world point's pixel, through the camera the frame was drawn with;
/// `None` behind the camera.
fn project(player: &ProjectPlayer, point: Vec3) -> Option<[f32; 2]> {
    let [width, height] = player.size;
    let camera = player
        .scene
        .world_camera_for_viewport(&player.world, width / height, CameraView::default())
        .ok()??;
    let clip = camera.view_projection * point.extend(1.0);
    if clip.w <= f32::EPSILON {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    Some([
        f32::midpoint(ndc.x, 1.0) * width,
        f32::midpoint(-ndc.y, 1.0) * height,
    ])
}

/// A pixel's world point on the plane the 2D world is drawn in.
fn pixel_world(player: &ProjectPlayer, [px, py]: [f32; 2]) -> Option<[f32; 2]> {
    let [width, height] = player.size;
    let camera = player
        .scene
        .world_camera_for_viewport(&player.world, width / height, CameraView::default())
        .ok()??;
    let inverse = camera.view_projection.inverse();
    let ndc = [px / width * 2.0 - 1.0, 1.0 - py / height * 2.0];
    let point = |depth: f32| {
        let world = inverse * Vec4::new(ndc[0], ndc[1], depth, 1.0);
        world.truncate() / world.w
    };
    let (near, far): (Vec3, Vec3) = (point(0.0), point(1.0));
    let along = far - near;
    if along.z.abs() < f32::EPSILON {
        return Some([near.x, near.y]);
    }
    let t = -near.z / along.z;
    let hit = near + along * t;
    Some([hit.x, hit.y])
}

/// Overlay units (the screen's half height is 1, y up) as a pixel.
fn overlay_pixel([x, y]: [f32; 2], size: [f32; 2]) -> [f32; 2] {
    let half = size[1] / 2.0;
    [size[0] / 2.0 + x * half, half - y * half]
}

/// What is at one pixel.
#[derive(Debug, Serialize)]
pub struct Probe {
    pub frame: u32,
    pub pixel: [f32; 2],
    /// The laid-out element there, by ID, the innermost when they nest.
    pub element: Option<String>,
    /// The element a press there would reach, which can differ.
    pub pressable: Option<String>,
    /// The world point there, on the plane the 2D world is drawn in.
    pub world: Option<[f32; 2]>,
    /// The entities whose colliders hold that world point.
    pub colliders: Vec<String>,
}

/// `probe:<x>,<y>`'s argument, a pixel from the top left.
///
/// # Errors
/// Not two numbers.
pub fn parse_pixel(text: &str) -> Result<[f32; 2], String> {
    let wrong = || format!("probe:{text} is probe:<x>,<y>, a pixel from the top left");
    let (x, y) = text.split_once(',').ok_or_else(wrong)?;
    Ok([
        x.trim().parse().map_err(|_| wrong())?,
        y.trim().parse().map_err(|_| wrong())?,
    ])
}

#[must_use]
pub fn probe(player: &ProjectPlayer, pixel: [f32; 2], frame: u32) -> Probe {
    let ui = player.session.screen_ui();
    let half = player.size[1] / 2.0;
    let overlay = [
        (pixel[0] - player.size[0] / 2.0) / half,
        (half - pixel[1]) / half,
    ];
    let name =
        |entity: Option<EntityId>| entity.and_then(|entity| stable_id(&player.world, entity));
    let world = pixel_world(player, pixel);
    let mut colliders: Vec<String> = world
        .map(|point| {
            outlines(player)
                .into_iter()
                .filter(|outline| contains(&outline.points, point))
                .filter_map(|outline| stable_id(&player.world, outline.entity))
                .collect()
        })
        .unwrap_or_default();
    colliders.dedup();
    Probe {
        frame,
        pixel,
        element: name(ui.element_at(overlay)),
        pressable: name(ui.pressable_at(overlay)),
        world,
        colliders,
    }
}

fn stable_id(world: &World, entity: EntityId) -> Option<String> {
    let data = world.get(entity)?;
    data.source_id
        .as_ref()
        .map(|id| id.as_str().to_owned())
        .or_else(|| data.name.clone())
}

/// Whether a closed outline holds `point`, by counting crossings.
fn contains(outline: &[[f32; 2]], [x, y]: [f32; 2]) -> bool {
    let mut inside = false;
    let mut previous = outline.len().wrapping_sub(1);
    for (index, &[xi, yi]) in outline.iter().enumerate() {
        let [xj, yj] = outline[previous];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        previous = index;
    }
    inside
}

/// RGBA pixels to draw lines on.
struct Canvas<'a> {
    pixels: &'a mut [u8],
    size: [u32; 2],
}

impl Canvas<'_> {
    fn polygon(&mut self, points: &[[f32; 2]], color: [u8; 4]) {
        for (index, &from) in points.iter().enumerate() {
            let to = points[(index + 1) % points.len()];
            self.line(from, to, color);
        }
    }

    /// Two pixels wide, so it reads on a busy frame and when scaled down.
    fn line(&mut self, [x0, y0]: [f32; 2], [x1, y1]: [f32; 2], color: [u8; 4]) {
        let steps = (x1 - x0)
            .abs()
            .max((y1 - y0).abs())
            .ceil()
            .clamp(1.0, 20_000.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        for step in 0..=(steps as u32) {
            #[allow(clippy::cast_precision_loss)]
            let t = step as f32 / steps;
            let (x, y) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
            for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
                self.plot(x + dx, y + dy, color);
            }
        }
    }

    fn plot(&mut self, x: f32, y: f32, color: [u8; 4]) {
        if !(x >= 0.0 && y >= 0.0) {
            return;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (x, y) = (x as u32, y as u32);
        if x >= self.size[0] || y >= self.size[1] {
            return;
        }
        let at = ((y * self.size[0] + x) * 4) as usize;
        if let Some(pixel) = self.pixels.get_mut(at..at + 4) {
            pixel.copy_from_slice(&color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_read() {
        assert_eq!(
            Layers::parse("colliders").unwrap(),
            Layers {
                colliders: true,
                ui: false
            }
        );
        assert_eq!(
            Layers::parse("ui, colliders").unwrap(),
            Layers {
                colliders: true,
                ui: true
            }
        );
        assert_eq!(
            Layers::parse("all").unwrap(),
            Layers::parse("colliders,ui").unwrap()
        );
        assert!(!Layers::parse("off").unwrap().any());
        assert!(Layers::parse("lights").is_err());
        assert_eq!(
            parse_pixel("640, 360").unwrap().map(f32::to_bits),
            [640.0_f32, 360.0].map(f32::to_bits)
        );
        assert!(parse_pixel("640").is_err());
    }

    #[test]
    fn shapes_and_insides() {
        let square = shape(&ColliderShape2d::Box {
            half_extents: [1.0, 0.5],
        });
        assert!(contains(&square, [0.9, 0.4]));
        assert!(!contains(&square, [1.1, 0.0]));
        let round = shape(&ColliderShape2d::Circle { radius: 1.0 });
        assert!(contains(&round, [0.0, 0.95]) && !contains(&round, [0.8, 0.8]));
        let capsule = shape(&ColliderShape2d::Capsule {
            half_height: 1.0,
            radius: 0.5,
        });
        assert!(contains(&capsule, [0.0, 1.4]) && !contains(&capsule, [0.6, 0.0]));
        let turned = rotate([1.0, 0.0], std::f32::consts::FRAC_PI_2);
        assert!(turned[0].abs() < 1e-6 && (turned[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_line_is_drawn_inside_the_canvas_only() {
        let mut pixels = vec![0; 8 * 8 * 4];
        let mut canvas = Canvas {
            pixels: &mut pixels,
            size: [8, 8],
        };
        canvas.line([-4.0, 2.0], [20.0, 2.0], [255, 0, 0, 255]);
        assert_eq!(
            &pixels[(2 * 8 + 3) * 4..(2 * 8 + 3) * 4 + 4],
            &[255, 0, 0, 255]
        );
        assert_eq!(pixels[(6 * 8 + 3) * 4], 0);
    }
}
