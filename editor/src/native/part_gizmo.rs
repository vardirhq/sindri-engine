//! The parts of a selected object as the Scene view draws them: its art's
//! frame, each emitter's shape and its contact shadow, each with a handle at
//! its center that moves it and one at its corner (or on its ring) that sizes
//! it.
//!
//! The same parts the phone's object workspace gives handles to
//! (`mobile/android/.../ObjectParts.kt`), so both editors place a part the
//! same way; `docs/object-parts.md` is the design. Lights have theirs in
//! `light2d_gizmo` and collider pieces in `collider_gizmo`.
//!
//! A sprite's `size` and `offset` are in the object's own space, so they turn
//! and scale with it; an emitter's `offset` and a shadow's `contact_offset`
//! are world units from the object's position. Each handle carries the space
//! it is measured in, and a drag is one undo step.

use eframe::egui::{self, Color32, LayerId, Order, Pos2, Rect, Response, Stroke};
use glam::{Mat4, Vec3};
use serde_json::{Value, json};
use sindri_core::{
    CommandBuffer, ComponentSchemaRegistry, EntityId, SceneComponent, World, WorldCommand, part_key,
};
use sindri_scene::{EmissionShape, ParticlesComponent, Shadow2dComponent, SpriteComponent};

use super::EditorApp;
use super::projection::{project_point, project_segment, unproject_to_plane};

const PART_LAYER: &str = "sindri-part-overlay";
const PART_DRAG: &str = "sindri-part-drag";

const ART_TEAL: Color32 = Color32::from_rgb(90, 210, 200);
const EMITTER_PINK: Color32 = Color32::from_rgb(255, 130, 190);
const SHADOW_GREY: Color32 = Color32::from_rgb(200, 200, 210);

const HANDLE_REACH: f32 = 8.0;
const HANDLE_SIZE: f32 = 7.0;
/// The least a dragged size can be, so a part dragged through its center is
/// not lost at zero.
const SMALLEST: f32 = 0.02;
const ROUND_SEGMENTS: usize = 40;

/// What a handle writes, in the space its handle carries.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Grip {
    /// The part's offset: the pointer, less `lift`, the part's center's
    /// distance from its offset (a sprite sheet's anchor, scaled by its size).
    Move { lift: [f32; 2] },
    /// A size vector from the corner of a box or ellipse centered at
    /// `center`; each axis keeps the sign it had, so mirrored art stays so.
    Corner { center: [f32; 2], was: [f32; 2] },
    /// The width of a round emitter, from a point on its ring.
    Across { center: [f32; 2], was: [f32; 2] },
}

/// Which handle of which part: what a drag holds on to across frames, while
/// the handle's own numbers change under it.
#[derive(Clone, Debug, PartialEq)]
struct Held {
    entity: EntityId,
    /// The component key it writes.
    key: String,
    /// The field it writes.
    field: &'static str,
}

/// A handle on screen: what it is, what it writes, where it is drawn, and the
/// space a dragged point is measured in.
struct Handle {
    held: Held,
    grip: Grip,
    at: Pos2,
    space: Mat4,
    color: Color32,
}

/// The value `grip` writes for a pointer at `local`, in the handle's space.
fn dragged(grip: Grip, local: [f32; 2]) -> Value {
    let across = |reach: f32, was: f32| (2.0 * reach.abs()).max(SMALLEST).copysign(was);
    match grip {
        Grip::Move { lift } => json!([local[0] - lift[0], local[1] - lift[1]]),
        Grip::Corner { center, was } => json!([
            across(local[0] - center[0], was[0]),
            across(local[1] - center[1], was[1]),
        ]),
        Grip::Across { center, was } => json!([
            (2.0 * (local[0] - center[0]).hypot(local[1] - center[1])).max(SMALLEST),
            was[1],
        ]),
    }
}

/// The corners of a box `size` across centered at `center`, around.
fn box_points(center: [f32; 2], size: [f32; 2]) -> Vec<[f32; 2]> {
    let [x, y] = [size[0].abs() / 2.0, size[1].abs() / 2.0];
    [[-x, -y], [x, -y], [x, y], [-x, y]]
        .map(|[dx, dy]| [center[0] + dx, center[1] + dy])
        .to_vec()
}

/// An ellipse `size` across centered at `center`, around.
fn round_points(center: [f32; 2], size: [f32; 2]) -> Vec<[f32; 2]> {
    #[allow(clippy::cast_precision_loss)]
    (0..ROUND_SEGMENTS)
        .map(|index| {
            let angle = index as f32 / ROUND_SEGMENTS as f32 * std::f32::consts::TAU;
            [
                center[0] + angle.cos() * size[0].abs() / 2.0,
                center[1] + angle.sin() * size[1].abs() / 2.0,
            ]
        })
        .collect()
}

/// Where a sized part's corner handle sits: its top right.
fn corner(center: [f32; 2], size: [f32; 2]) -> [f32; 2] {
    [
        center[0] + size[0].abs() / 2.0,
        center[1] + size[1].abs() / 2.0,
    ]
}

/// The shape of one part, ready to draw and hold.
struct Placed {
    key: String,
    color: Color32,
    space: Mat4,
    outline: Vec<[f32; 2]>,
    /// The offset field and its handle.
    moves: (&'static str, [f32; 2], Grip),
    /// The size field and its handle, when the part has a size to drag.
    sizes: Option<(&'static str, [f32; 2], Grip)>,
}

impl EditorApp {
    /// Draws the selected objects' art frames, emitter shapes and contact
    /// shadows, and lets their handles be dragged.
    ///
    /// Returns whether a handle has the pointer, so the move gizmo and the
    /// camera leave the drag alone.
    pub(super) fn part_overlay(
        &mut self,
        context: &egui::Context,
        response: &Response,
        blocked: bool,
    ) -> bool {
        let rect = response.rect;
        let aspect = rect.width() / rect.height().max(1.0);
        let Some(camera) = self
            .scene
            .world_camera_for_viewport(&self.world, aspect, self.scene_camera())
            .ok()
            .flatten()
        else {
            return false;
        };
        let view_projection = camera.view_projection;
        let painter = context
            .layer_painter(LayerId::new(
                Order::Background,
                egui::Id::unique(PART_LAYER),
            ))
            .with_clip_rect(rect);
        let mut handles = Vec::new();
        let bindings = self.textures.bindings();
        let anchor = |sprite: &SpriteComponent| {
            sprite
                .reference()
                .map(|reference| bindings.sprite_anchor(&reference).offset())
                .unwrap_or_default()
        };
        let placed = placed_parts(
            &self.world,
            self.scene.components(),
            self.selection.all(),
            anchor,
        );
        for (entity, placed) in placed {
            let project = |point: [f32; 2]| {
                project_point(
                    rect,
                    view_projection,
                    placed
                        .space
                        .transform_point3(Vec3::new(point[0], point[1], 0.0)),
                )
            };
            let stroke = Stroke::new(1.5, placed.color);
            for (index, start) in placed.outline.iter().enumerate() {
                let end = placed.outline[(index + 1) % placed.outline.len()];
                let [a, b] =
                    [*start, end].map(|[x, y]| placed.space.transform_point3(Vec3::new(x, y, 0.0)));
                if let Some(segment) = project_segment(rect, view_projection, a, b) {
                    painter.line_segment(segment, stroke);
                }
            }
            for (field, point, grip) in std::iter::once(placed.moves).chain(placed.sizes) {
                if let Some(at) = project(point) {
                    handles.push(Handle {
                        held: Held {
                            entity,
                            key: placed.key.clone(),
                            field,
                        },
                        grip,
                        at,
                        space: placed.space,
                        color: placed.color,
                    });
                }
            }
        }
        for handle in &handles {
            painter.rect(
                Rect::from_center_size(handle.at, egui::vec2(HANDLE_SIZE, HANDLE_SIZE)),
                1.0,
                handle.color,
                Stroke::new(1.0, Color32::BLACK),
                egui::StrokeKind::Middle,
            );
        }
        self.drag_part_handle(context, response, blocked, &handles, view_projection)
    }

    fn drag_part_handle(
        &mut self,
        context: &egui::Context,
        response: &Response,
        blocked: bool,
        handles: &[Handle],
        view_projection: Mat4,
    ) -> bool {
        let id = egui::Id::unique(PART_DRAG);
        let pointer = response.interact_pointer_pos().or(response.hover_pos());
        let pressed = context
            .input(|input| input.pointer.press_origin())
            .or(pointer);
        let near = pressed.and_then(|pointer| {
            handles
                .iter()
                .find(|handle| handle.at.distance(pointer) <= HANDLE_REACH)
        });
        let mut drag = context.data(|data| data.get_temp::<Held>(id));
        if drag.is_none()
            && !blocked
            && response.drag_started_by(egui::PointerButton::Primary)
            && let Some(handle) = near
        {
            drag = Some(handle.held.clone());
        }
        let Some(active) = drag else {
            return near.is_some() && !blocked;
        };
        let Some(handle) = handles.iter().find(|handle| handle.held == active) else {
            context.data_mut(|data| data.remove::<Held>(id));
            return false;
        };
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(pointer) = pointer
            && let Some(world) = unproject_to_plane(
                response.rect,
                view_projection,
                pointer,
                handle.space.w_axis.z,
            )
        {
            let local = handle.space.inverse().transform_point3(world);
            self.write_part(&active, dragged(handle.grip, [local.x, local.y]));
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            context.data_mut(|data| data.remove::<Held>(id));
        } else {
            context.data_mut(|data| data.insert_temp(id, active));
        }
        true
    }

    /// Writes `value` into the field `held` names, keeping every other field
    /// as authored, merged into one undo step for the whole drag.
    fn write_part(&mut self, held: &Held, value: Value) {
        let Some(mut payload) = self
            .world
            .get(held.entity)
            .and_then(|data| data.components.get(&held.key))
            .cloned()
        else {
            return;
        };
        let Some(object) = payload.as_object_mut() else {
            return;
        };
        object.insert(held.field.to_owned(), value);
        let mut buffer = CommandBuffer::new();
        buffer.push(WorldCommand::SetComponent {
            entity: held.entity,
            type_name: held.key.clone(),
            payload,
        });
        let transaction = buffer.into_transaction(label(held)).merging(format!(
            "part-handle:{}:{}:{}",
            held.entity.index(),
            held.key,
            held.field
        ));
        if let Err(error) = self.apply_edit(transaction) {
            self.report(error.to_string());
        }
    }
}

/// Every placeable part of the `selected` objects, with the space its
/// numbers are in. `anchor` is where a sprite's sheet puts its picture in its
/// quad.
fn placed_parts(
    world: &World,
    components: &ComponentSchemaRegistry,
    selected: &[EntityId],
    anchor: impl Fn(&SpriteComponent) -> [f32; 2],
) -> Vec<(EntityId, Placed)> {
    let mut placed = Vec::new();
    for &entity in selected {
        let transform = world.world_transform(entity).unwrap_or_default();
        let object = Mat4::from_scale_rotation_translation(
            Vec3::from_array(transform.scale),
            glam::Quat::from_array(transform.rotation),
            Vec3::from_array(transform.position),
        );
        // Emitters and shadows sit in world units from the object's
        // position, unturned and unscaled.
        let around = Mat4::from_translation(object.w_axis.truncate());
        if let Ok(Some(sprite)) = components.get::<SpriteComponent>(world, entity)
            && sprite.placement_is_finite()
        {
            placed.push((entity, art(&sprite, anchor(&sprite), object)));
        }
        for (part, particles) in components
            .get_parts::<ParticlesComponent>(world, entity)
            .unwrap_or_default()
        {
            placed.push((entity, emitter(part.as_deref(), &particles, around)));
        }
        if let Ok(Some(shadow)) = components.get::<Shadow2dComponent>(world, entity)
            && shadow.contact_size.iter().all(|side| *side > 0.0)
        {
            placed.push((entity, contact_shadow(&shadow, around)));
        }
    }
    placed
}

/// A sprite's frame, where it is drawn: its sheet's `anchor` moves the
/// picture within its own quad, so it moves with the size.
fn art(sprite: &SpriteComponent, anchor: [f32; 2], space: Mat4) -> Placed {
    let lift = [anchor[0] * sprite.size[0], anchor[1] * sprite.size[1]];
    let center = [sprite.offset[0] + lift[0], sprite.offset[1] + lift[1]];
    Placed {
        key: SpriteComponent::TYPE_NAME.to_owned(),
        color: ART_TEAL,
        space,
        outline: box_points(center, sprite.size),
        moves: ("offset", center, Grip::Move { lift }),
        sizes: Some((
            "size",
            corner(center, sprite.size),
            Grip::Corner {
                center,
                was: sprite.size,
            },
        )),
    }
}

/// An emitter part's shape: a box with a corner handle, a circle or ring with
/// one on its edge, or a point that only moves.
fn emitter(part: Option<&str>, particles: &ParticlesComponent, space: Mat4) -> Placed {
    let center = particles.offset;
    let size = particles.shape_size;
    let (outline, sizes) = match particles.shape {
        EmissionShape::Box => (
            box_points(center, size),
            Some((
                "shape_size",
                corner(center, size),
                Grip::Corner { center, was: size },
            )),
        ),
        EmissionShape::Circle | EmissionShape::Ring => (
            round_points(center, [size[0], size[0]]),
            Some((
                "shape_size",
                [center[0] + size[0].abs() / 2.0, center[1]],
                Grip::Across { center, was: size },
            )),
        ),
        EmissionShape::Point => (round_points(center, [0.2, 0.2]), None),
    };
    Placed {
        key: part_key(ParticlesComponent::TYPE_NAME, part),
        color: EMITTER_PINK,
        space,
        outline,
        moves: ("offset", center, Grip::Move { lift: [0.0, 0.0] }),
        sizes,
    }
}

/// The contact shadow's ellipse.
fn contact_shadow(shadow: &Shadow2dComponent, space: Mat4) -> Placed {
    let center = shadow.contact_offset;
    let size = shadow.contact_size;
    Placed {
        key: Shadow2dComponent::TYPE_NAME.to_owned(),
        color: SHADOW_GREY,
        space,
        outline: round_points(center, size),
        moves: ("contact_offset", center, Grip::Move { lift: [0.0, 0.0] }),
        sizes: Some((
            "contact_size",
            corner(center, size),
            Grip::Corner { center, was: size },
        )),
    }
}

/// What a drag on `held` is called in the undo history.
fn label(held: &Held) -> &'static str {
    let moving = matches!(held.field, "offset" | "contact_offset");
    match (sindri_core::split_part(&held.key).0, moving) {
        (SpriteComponent::TYPE_NAME, true) => "Move sprite",
        (SpriteComponent::TYPE_NAME, false) => "Resize sprite",
        (ParticlesComponent::TYPE_NAME, true) => "Move emitter",
        (ParticlesComponent::TYPE_NAME, false) => "Resize emitter",
        (_, true) => "Move shadow",
        (_, false) => "Resize shadow",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_corner_handle_sizes_evenly_either_side_and_keeps_a_mirror() {
        let grip = Grip::Corner {
            center: [1.0, 0.0],
            was: [-2.0, 3.0],
        };
        assert_eq!(dragged(grip, [3.0, -2.0]), json!([-4.0, 4.0]));
        let through = dragged(grip, [1.0, 0.0]);
        assert!(through[1].as_f64().unwrap() >= f64::from(SMALLEST) - 1.0e-6);
    }

    #[test]
    fn moving_art_moves_its_offset_not_its_anchored_center() {
        let grip = Grip::Move { lift: [0.0, 0.5] };
        assert_eq!(dragged(grip, [2.0, 3.0]), json!([2.0, 2.5]));
    }

    #[test]
    fn a_round_emitter_is_as_wide_as_twice_the_handle_is_far() {
        let grip = Grip::Across {
            center: [0.0, 0.0],
            was: [1.0, 7.0],
        };
        assert_eq!(dragged(grip, [3.0, 4.0]), json!([10.0, 7.0]));
    }

    #[test]
    fn a_corner_sits_at_the_top_right_of_a_mirrored_size_too() {
        let [x, y] = corner([1.0, 1.0], [-2.0, 4.0]);
        assert!((x - 2.0).abs() < 1.0e-6 && (y - 3.0).abs() < 1.0e-6);
    }

    #[test]
    fn a_selected_gloom_shows_its_art_both_emitters_and_its_shadow() {
        let components = crate::native::scene_io::scene_extractor()
            .components()
            .clone();
        let mut world = World::default();
        let gloom = world.spawn(sindri_core::EntityData {
            transform_3d: Some(sindri_core::Transform3D {
                position: [4.0, 1.0, 0.0],
                scale: [2.0, 2.0, 1.0],
                ..sindri_core::Transform3D::default()
            }),
            components: [
                ("sindri.sprite".to_owned(), json!({ "texture": "g.png", "size": [3.0, 2.0] })),
                ("sindri.particles".to_owned(), json!({ "texture": "s.png", "shape": "circle", "shape_size": [1.0, 1.0] })),
                ("sindri.particles#surge".to_owned(), json!({ "texture": "s.png", "shape": "box", "shape_size": [2.0, 0.5], "offset": [0.0, 1.0] })),
                ("sindri.shadow2d".to_owned(), json!({ "contact_size": [1.0, 0.4] })),
            ]
            .into_iter()
            .collect(),
            ..sindri_core::EntityData::default()
        });
        let placed = placed_parts(&world, &components, &[gloom], |_| [0.0, 0.5]);
        let keys: Vec<&str> = placed.iter().map(|(_, part)| part.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "sindri.sprite",
                "sindri.particles",
                "sindri.particles#surge",
                "sindri.shadow2d"
            ]
        );
        // The art turns and scales with the object; its anchor lifts it by
        // half its height, so its middle is a whole size unit above the feet.
        let art = &placed[0].1;
        let middle = art
            .space
            .transform_point3(Vec3::new(art.moves.1[0], art.moves.1[1], 0.0));
        assert!(
            (middle - Vec3::new(4.0, 3.0, 0.0)).length() < 1.0e-5,
            "{middle}"
        );
        // The surge's box is in world units from the object, unscaled.
        let surge = &placed[2].1;
        let (field, at, _) = surge.sizes.unwrap();
        assert_eq!(field, "shape_size");
        let at = surge.space.transform_point3(Vec3::new(at[0], at[1], 0.0));
        assert!((at - Vec3::new(5.0, 2.25, 0.0)).length() < 1.0e-5, "{at}");
        // A round emitter is sized across, not by a corner.
        assert!(matches!(
            placed[1].1.sizes,
            Some((_, _, Grip::Across { .. }))
        ));
    }
}
