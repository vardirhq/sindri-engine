use std::time::Duration;

use serde_json::json;
use sindri_core::{EntityData, EntityId, SceneComponent, SceneEntityId, Transform3D, World};
use sindri_physics::{Collider2d, Collider3d, ColliderShape3d, PhysicsPose3d};

use super::reach::effect_reach;
use super::solids::wireframe;
use super::{GizmoKind, shape_gizmos};
use crate::{
    Character2dComponent, Collider2dComponent, EffectBurstComponent, Effects2d,
    HingeJoint2dComponent, SceneExtractor, SliderJoint2dComponent,
};

fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1.0e-4)
}

fn registry() -> sindri_core::ComponentSchemaRegistry {
    SceneExtractor::new().unwrap().components().clone()
}

fn spawn(
    world: &mut World,
    id: &str,
    position: [f32; 3],
    turned: f32,
    components: Vec<(&str, serde_json::Value)>,
) -> EntityId {
    world.spawn(EntityData {
        source_id: SceneEntityId::new(id).ok(),
        transform_3d: Some(Transform3D {
            position,
            rotation: glam::Quat::from_rotation_z(turned).to_array(),
            ..Transform3D::default()
        }),
        components: components
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
        ..EntityData::default()
    })
}

#[test]
fn a_hinge_is_drawn_between_its_anchors_as_the_bodies_turn_them() {
    let registry = registry();
    let mut world = World::default();
    spawn(&mut world, "door", [0.0, 0.0, 0.0], 0.0, vec![]);
    // Turned a quarter, so its local X anchor is the world's Y.
    spawn(
        &mut world,
        "frame",
        [4.0, 0.0, 0.0],
        std::f32::consts::FRAC_PI_2,
        vec![],
    );
    let mut hinge = registry
        .default_payload(HingeJoint2dComponent::TYPE_NAME)
        .unwrap()
        .clone();
    hinge["first"] = json!("door");
    hinge["second"] = json!("frame");
    hinge["first_anchor"] = json!([1.0, 0.0]);
    hinge["second_anchor"] = json!([1.0, 0.0]);
    hinge["limits_enabled"] = json!(true);
    hinge["lower_angle"] = json!(-0.5);
    hinge["upper_angle"] = json!(0.5);
    let owner = spawn(
        &mut world,
        "hinge",
        [0.0; 3],
        0.0,
        vec![(HingeJoint2dComponent::TYPE_NAME, hinge)],
    );
    let gizmos = shape_gizmos(&world, &registry);
    let joint = gizmos
        .iter()
        .find(|gizmo| gizmo.entity == owner)
        .expect("drawn");
    assert_eq!(joint.kind, GizmoKind::Joint);
    assert!(near(joint.marks[0], [1.0, 0.0, 0.0]));
    assert!(near(joint.marks[1], [4.0, 1.0, 0.0]));
    assert_eq!(joint.strokes.len(), 2, "the line, and the allowed swing");
}

#[test]
fn a_joint_missing_an_end_draws_nothing() {
    let registry = registry();
    let mut world = World::default();
    spawn(&mut world, "rail", [0.0; 3], 0.0, vec![]);
    let mut slider = registry
        .default_payload(SliderJoint2dComponent::TYPE_NAME)
        .unwrap()
        .clone();
    slider["first"] = json!("rail");
    slider["second"] = json!("nobody");
    spawn(
        &mut world,
        "slider",
        [0.0; 3],
        0.0,
        vec![(SliderJoint2dComponent::TYPE_NAME, slider)],
    );
    assert!(shape_gizmos(&world, &registry).is_empty());
}

#[test]
fn a_box_has_twelve_edges_where_its_body_and_offset_put_it() {
    let piece = Collider3d {
        shape: ColliderShape3d::Box {
            half_extents: [1.0, 2.0, 3.0],
        },
        offset: [10.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        sensor: false,
        layers: sindri_physics::CollisionLayers::default(),
        friction: 0.5,
        restitution: 0.0,
    };
    let pose = PhysicsPose3d {
        position: [0.0, 5.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
    };
    let strokes = wireframe(pose, &piece);
    let edges: usize = strokes.iter().map(|stroke| stroke.segments().count()).sum();
    assert_eq!(edges, 12);
    let corners: Vec<[f32; 3]> = strokes.iter().flat_map(|s| s.points.clone()).collect();
    assert!(corners.iter().any(|&c| near(c, [11.0, 7.0, 3.0])));
    assert!(corners.iter().any(|&c| near(c, [9.0, 3.0, -3.0])));
}

#[test]
fn a_character_shows_its_footing_from_the_bottom_of_its_collider() {
    let registry = registry();
    let mut world = World::default();
    let hero = spawn(
        &mut world,
        "hero",
        [2.0, 3.0, 0.0],
        0.0,
        vec![
            (
                Collider2dComponent::TYPE_NAME,
                json!({"pieces": [Collider2d::rectangle([0.5, 1.0])]}),
            ),
            (
                Character2dComponent::TYPE_NAME,
                json!({"step_height": 0.25, "snap_distance": 0.5}),
            ),
        ],
    );
    let gizmos = shape_gizmos(&world, &registry);
    let character = gizmos
        .iter()
        .find(|gizmo| gizmo.entity == hero && gizmo.kind == GizmoKind::Character)
        .expect("drawn");
    assert!(near(character.marks[0], [2.0, 2.0, 0.0]), "the foot");
    // Two slope lines, the step and the snap.
    assert_eq!(character.strokes.len(), 4);
    assert!(near(character.strokes[2].points[0], [1.5, 2.25, 0.0]));
    assert!(near(character.strokes[3].points[0], [1.5, 1.5, 0.0]));
}

#[test]
fn the_reach_drawn_for_a_burst_is_how_far_its_flecks_fly() {
    let burst = EffectBurstComponent {
        texture: "sindri:white".to_owned(),
        count: 256,
        speed: 6.0,
        spread: 0.5,
        lifetime: 1.5,
        size: 0.1,
        tint: [1.0; 4],
        fade: true,
        drag: 2.0,
        layer: 0,
    };
    let mut effects = Effects2d::default();
    effects.burst(&burst, [0.0, 0.0]);
    let mut farthest = 0.0_f32;
    while effects.live() > 0 {
        effects.advance(Duration::from_secs_f32(1.0 / 60.0));
        for fleck in effects.flecks() {
            farthest = farthest.max(fleck.position[0].hypot(fleck.position[1]));
        }
    }
    let reach = effect_reach(burst.speed * 1.5, burst.lifetime, burst.drag);
    assert!(farthest <= reach * 1.02, "{farthest} well past {reach}");
    assert!(farthest > reach * 0.9, "{farthest} well short of {reach}");
}
