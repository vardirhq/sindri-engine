//! A 2D joint, drawn where physics holds it: from one anchor to the other,
//! with what it allows.
//!
//! Anchors are body-local and turn with their body; the transform's scale is
//! not applied to them, as physics does not apply it. A joint with an endpoint
//! missing or inactive is suspended, and draws nothing.

use glam::Vec3;
use sindri_core::{ComponentSchemaRegistry, EntityId, World};
use sindri_physics::PhysicsPose2d;

use super::{GizmoKind, GizmoStroke, ShapeGizmo, arc, circle};
use crate::physics::RigidBody2dComponent;
use crate::physics_sync::pose_of;
use crate::{
    DistanceJoint2dComponent, HingeJoint2dComponent, SliderJoint2dComponent, SpringJoint2dComponent,
};

/// How far out a hinge's allowed swing is drawn, in world units.
const SWING: f32 = 0.5;

/// How far either way a slider with no limits is drawn along its axis.
const FREE_TRAVEL: f32 = 2.0;

/// Half the length of the tick that marks a limit or a rest length.
const TICK: f32 = 0.12;

/// One body as a joint sees it.
#[derive(Clone, Copy)]
struct Body {
    pose: PhysicsPose2d,
    depth: f32,
}

impl Body {
    fn of(world: &World, components: &ComponentSchemaRegistry, entity: EntityId) -> Self {
        let body = components
            .get::<RigidBody2dComponent>(world, entity)
            .ok()
            .flatten()
            .map(|authored| authored.0);
        Self {
            pose: pose_of(world, entity, body),
            depth: world
                .world_transform(entity)
                .map_or(0.0, |transform| transform.position[2]),
        }
    }

    /// Where a point in the body's own space is in the world.
    fn place(self, local: [f32; 2]) -> Vec3 {
        let turned = turn(local, self.pose.rotation);
        Vec3::new(
            self.pose.position[0] + turned[0],
            self.pose.position[1] + turned[1],
            self.depth,
        )
    }

    /// A direction in the body's own space, in the world.
    fn direction(self, local: [f32; 2]) -> Vec3 {
        let [x, y] = turn(local, self.pose.rotation);
        Vec3::new(x, y, 0.0).normalize_or_zero()
    }
}

fn turn([x, y]: [f32; 2], angle: f32) -> [f32; 2] {
    let (sin, cos) = angle.sin_cos();
    [x * cos - y * sin, x * sin + y * cos]
}

/// The two bodies a joint on `owner` connects, while both are there.
fn endpoints(
    world: &World,
    components: &ComponentSchemaRegistry,
    owner: EntityId,
    first: &str,
    second: &str,
) -> Option<(Body, Body)> {
    let resolve = |target: &str| {
        world
            .resolve_entity_reference(owner, target)
            .filter(|&entity| world.is_active(entity))
    };
    let (first, second) = resolve(first).zip(resolve(second))?;
    Some((
        Body::of(world, components, first),
        Body::of(world, components, second),
    ))
}

/// A short line across `direction` at `at`.
fn tick(at: Vec3, direction: Vec3) -> GizmoStroke {
    let across = Vec3::new(-direction.y, direction.x, 0.0) * TICK;
    GizmoStroke::open(vec![(at - across).to_array(), (at + across).to_array()])
}

fn gizmo(owner: EntityId, a: Vec3, b: Vec3, mut strokes: Vec<GizmoStroke>) -> ShapeGizmo {
    strokes.insert(0, GizmoStroke::open(vec![a.to_array(), b.to_array()]));
    ShapeGizmo {
        entity: owner,
        kind: GizmoKind::Joint,
        strokes,
        marks: vec![a.to_array(), b.to_array()],
    }
}

pub(super) fn joints(world: &World, components: &ComponentSchemaRegistry) -> Vec<ShapeGizmo> {
    let mut gizmos = Vec::new();
    for (owner, joint) in components
        .query::<DistanceJoint2dComponent>(world)
        .unwrap_or_default()
    {
        let Some((first, second)) =
            endpoints(world, components, owner, &joint.first, &joint.second)
        else {
            continue;
        };
        // A rope between the centres: the second may be anywhere inside it.
        let (a, b) = (first.place([0.0; 2]), second.place([0.0; 2]));
        let reach = circle(a, Vec3::X, Vec3::Y, joint.max_distance);
        gizmos.push(gizmo(owner, a, b, vec![reach]));
    }
    for (owner, joint) in components
        .query::<HingeJoint2dComponent>(world)
        .unwrap_or_default()
    {
        let Some((first, second)) =
            endpoints(world, components, owner, &joint.first, &joint.second)
        else {
            continue;
        };
        let settings = joint.settings;
        let a = first.place(settings.first_anchor);
        let b = second.place(settings.second_anchor);
        let mut strokes = Vec::new();
        if settings.limits_enabled {
            // The swing the second body is allowed, measured from the first
            // body's own heading.
            let from = first.pose.rotation + settings.lower_angle;
            let to = first.pose.rotation + settings.upper_angle;
            let mut swing = vec![a.to_array()];
            swing.extend(arc(a, Vec3::X, Vec3::Y, SWING, from, to));
            swing.push(a.to_array());
            strokes.push(GizmoStroke::open(swing));
        }
        gizmos.push(gizmo(owner, a, b, strokes));
    }
    for (owner, joint) in components
        .query::<SliderJoint2dComponent>(world)
        .unwrap_or_default()
    {
        let Some((first, second)) =
            endpoints(world, components, owner, &joint.first, &joint.second)
        else {
            continue;
        };
        let settings = joint.settings;
        let a = first.place(settings.first_anchor);
        let b = second.place(settings.second_anchor);
        let axis = first.direction(settings.first_axis);
        let (lower, upper) = if settings.limits_enabled {
            (settings.lower_distance, settings.upper_distance)
        } else {
            (-FREE_TRAVEL, FREE_TRAVEL)
        };
        let mut strokes = vec![GizmoStroke::open(vec![
            (a + axis * lower).to_array(),
            (a + axis * upper).to_array(),
        ])];
        if settings.limits_enabled {
            strokes.push(tick(a + axis * lower, axis));
            strokes.push(tick(a + axis * upper, axis));
        }
        gizmos.push(gizmo(owner, a, b, strokes));
    }
    for (owner, joint) in components
        .query::<SpringJoint2dComponent>(world)
        .unwrap_or_default()
    {
        let Some((first, second)) =
            endpoints(world, components, owner, &joint.first, &joint.second)
        else {
            continue;
        };
        let settings = joint.settings;
        let a = first.place(settings.first_anchor);
        let b = second.place(settings.second_anchor);
        // Where the spring would rest, along the line it pulls on.
        let along = (b - a).normalize_or(Vec3::X);
        let rest = tick(a + along * settings.rest_length, along);
        gizmos.push(gizmo(owner, a, b, vec![rest]));
    }
    gizmos
}
