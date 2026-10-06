//! A real picker click becomes a checked edit, and undo restores the constraint.
use eframe::egui::{self, pos2};
use serde_json::json;
use sindri_core::{CommandHistory, EntityData, SceneComponent, SceneEntityId, World};
use sindri_scene::{
    DistanceJoint2dComponent, HingeJoint2dComponent, SceneExtractor, ScenePhysics2d,
    SliderJoint2dComponent, SpringJoint2dComponent,
};
use std::time::Duration;

use super::super::draft::component_commands;
use crate::inspector::entities::EntityReferences;

#[test]
fn choosing_and_clearing_an_endpoint_apply_and_undo_through_commands() {
    for kind in [
        DistanceJoint2dComponent::TYPE_NAME,
        HingeJoint2dComponent::TYPE_NAME,
        SliderJoint2dComponent::TYPE_NAME,
        SpringJoint2dComponent::TYPE_NAME,
    ] {
        exercise_kind(kind);
    }
}

fn exercise_kind(kind: &str) {
    let extractor = SceneExtractor::new().unwrap();
    let mut world = World::default();
    for name in ["anchor", "body", "replacement"] {
        let mut collider = extractor
            .components()
            .default_payload("sindri.physics2d.collider")
            .unwrap()
            .clone();
        collider["pieces"][0]["layers"] = json!({"memberships":0,"filter":0});
        let mut components =
            std::collections::BTreeMap::from([("sindri.physics2d.collider".into(), collider)]);
        if name != "anchor" {
            components.insert(
                "sindri.physics2d.rigid_body".into(),
                extractor
                    .components()
                    .default_payload("sindri.physics2d.rigid_body")
                    .unwrap()
                    .clone(),
            );
        }
        world.spawn(EntityData {
            source_id: SceneEntityId::new(name).ok(),
            components,
            ..EntityData::default()
        });
    }
    let owner = world.spawn(EntityData {
        source_id: SceneEntityId::new("joint").ok(),
        components: [(
            kind.into(),
            json!({"first":"anchor", "second":"body", "max_distance":2.0, "extra":17}),
        )]
        .into(),
        ..EntityData::default()
    });
    let mut physics = ScenePhysics2d::top_down().unwrap();
    let step = Duration::from_millis(16);
    physics
        .step(&mut world, extractor.components(), step)
        .unwrap();
    assert_eq!(physics.world().joint_count(), 1);
    let mut history = CommandHistory::default();
    for (label, expected, count) in [("replacement", "replacement", 1), ("None", "", 0)] {
        let original = world.get(owner).unwrap().components.clone();
        let references = EntityReferences::new(&world, owner, &original);
        let mut draft = original.clone();
        let value = &mut draft.get_mut(kind).unwrap()["second"];
        choose(value, &references, label);
        assert_eq!(value.as_str(), Some(expected));
        let (commands, refused) =
            component_commands(owner, &original, &draft, extractor.components());
        assert!(refused.is_empty());
        history
            .apply(
                commands.into_transaction("Choose joint endpoint"),
                &mut world,
            )
            .unwrap();
        physics
            .step(&mut world, extractor.components(), step)
            .unwrap();
        assert_eq!(physics.world().joint_count(), count);
        assert_eq!(world.get(owner).unwrap().components[kind]["extra"], 17);
        history.undo(&mut world).unwrap();
        assert_eq!(world.get(owner).unwrap().components, original);
        physics
            .step(&mut world, extractor.components(), step)
            .unwrap();
        assert_eq!(physics.world().joint_count(), 1);
    }
}

fn choose(value: &mut serde_json::Value, references: &EntityReferences, label: &str) {
    let context = egui::Context::default();
    let picker = std::cell::Cell::new(egui::Rect::NOTHING);
    let mut draw = |events| {
        context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    pos2(0.0, 0.0),
                    egui::vec2(600.0, 400.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                picker.set(super::entity_row(ui, "second", "Second", value, references, 0.0).rect);
            },
        )
    };
    draw(Vec::new()).drop_without_applying_deltas();
    let click = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    // The picker sits at the far end of the property row.
    let pos = picker.get().center();
    draw(vec![egui::Event::PointerMoved(pos), click(pos, true)]).drop_without_applying_deltas();
    draw(vec![click(pos, false)]).drop_without_applying_deltas();
    let output = draw(Vec::new());
    let target = output
        .shapes
        .iter()
        .find_map(|shape| {
            if let egui::epaint::Shape::Text(text) = &shape.shape {
                (text.galley.job.text == label)
                    .then_some(text.pos + text.galley.rect.center().to_vec2())
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("picker option {label} was not drawn"));
    output.drop_without_applying_deltas();
    draw(vec![egui::Event::PointerMoved(target), click(target, true)])
        .drop_without_applying_deltas();
    draw(vec![click(target, false)]).drop_without_applying_deltas();
}
