//! What a scene's invisible settings reach, as lines a view can draw.
//!
//! A joint, a 3D collider, a character's footing and an effect's spray all
//! act somewhere in the world without drawing anything there, so an author
//! tunes them by guessing numbers. These are the shapes the editor draws for
//! them, worked out from the same values the simulation is given: a joint's
//! anchors from the bodies' poses as physics places them, a 3D collider's
//! pieces as the 3D physics world inserts them, a character's slope, step and
//! snap from its collider's lowest point, and an effect's reach from how its
//! flecks move. Audio has nothing to draw: a source plays at one volume
//! wherever it is, with no falloff to show.
//!
//! A component that does not parse draws nothing rather than a stale shape;
//! the inspector names the broken field.

mod joints;
mod reach;
mod solids;

#[cfg(test)]
mod tests;

use sindri_core::{ComponentSchemaRegistry, EntityId, World};

/// What a gizmo shows, which decides how a view draws it and when.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GizmoKind {
    /// A 2D joint: the line between its anchors, and its limits.
    Joint,
    /// A 3D collider's pieces.
    Collider3d,
    /// A 2D character's footing: the steepest ground it walks, how high it
    /// steps and how far down it snaps.
    Character,
    /// How far an effect burst's flecks can fly.
    EffectReach,
}

/// One drawn line: a run of points in world space, joined in order.
#[derive(Clone, Debug, PartialEq)]
pub struct GizmoStroke {
    pub points: Vec<[f32; 3]>,
    /// Whether the last point joins back to the first.
    pub closed: bool,
}

impl GizmoStroke {
    fn open(points: Vec<[f32; 3]>) -> Self {
        Self {
            points,
            closed: false,
        }
    }

    fn closed(points: Vec<[f32; 3]>) -> Self {
        Self {
            points,
            closed: true,
        }
    }

    /// Each segment of the line, as its two ends.
    pub fn segments(&self) -> impl Iterator<Item = ([f32; 3], [f32; 3])> + '_ {
        let count = self.points.len();
        let joins = if self.closed && count > 2 {
            count
        } else {
            count.saturating_sub(1)
        };
        (0..joins).map(move |index| (self.points[index], self.points[(index + 1) % count]))
    }
}

/// One entity's gizmo.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeGizmo {
    pub entity: EntityId,
    pub kind: GizmoKind,
    pub strokes: Vec<GizmoStroke>,
    /// Points worth a dot: a joint's anchors.
    pub marks: Vec<[f32; 3]>,
}

/// Every gizmo in `world`, joints first, then 3D colliders, characters and
/// effects, each in entity order.
#[must_use]
pub fn shape_gizmos(world: &World, components: &ComponentSchemaRegistry) -> Vec<ShapeGizmo> {
    let mut gizmos = joints::joints(world, components);
    gizmos.extend(solids::colliders(world, components));
    gizmos.extend(reach::characters(world, components));
    gizmos.extend(reach::effects(world, components));
    gizmos
}

/// How many straight edges a full circle is drawn with.
const ROUND: usize = 32;

/// Points around a circle in the plane of `a` and `b` (unit vectors at right
/// angles), from angle `from` to `to`, about `centre`.
fn arc(
    centre: glam::Vec3,
    a: glam::Vec3,
    b: glam::Vec3,
    radius: f32,
    from: f32,
    to: f32,
) -> Vec<[f32; 3]> {
    let span = (to - from).abs();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let steps = ((span / std::f32::consts::TAU) * ROUND as f32)
        .ceil()
        .max(2.0) as usize;
    (0..=steps)
        .map(|step| {
            #[allow(clippy::cast_precision_loss)]
            let angle = from + (to - from) * step as f32 / steps as f32;
            (centre + (a * angle.cos() + b * angle.sin()) * radius).to_array()
        })
        .collect()
}

/// A whole circle, as a closed stroke.
fn circle(centre: glam::Vec3, a: glam::Vec3, b: glam::Vec3, radius: f32) -> GizmoStroke {
    let mut points = arc(centre, a, b, radius, 0.0, std::f32::consts::TAU);
    points.pop();
    GizmoStroke::closed(points)
}
