//! Toggles, text fields and scroll regions, read and set from a script.

use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptReport, ScriptSources, Scripts};
use sindri_platform::InputState;
use sindri_scene::{
    SceneExtractor, ScreenExtent, ScreenUi, UiScrollComponent, UiTextInputComponent,
    UiToggleComponent,
};

fn registry() -> ComponentSchemaRegistry {
    let mut registry = SceneExtractor::new()
        .expect("the builtin components register")
        .components()
        .clone();
    registry
        .register::<ScriptComponent>("Script")
        .expect("sindri.script registers");
    registry
}

/// One element carrying all three widgets, so one script reaches each.
fn form(world: &mut World) -> EntityId {
    world.spawn(EntityData {
        transform_3d: Some(Transform3D {
            scale: [1.0, 0.3, 1.0],
            ..Transform3D::default()
        }),
        components: [
            (
                ScriptComponent::TYPE_NAME.to_owned(),
                json!({ "source": "form.decay", "script": "Form" }),
            ),
            (
                UiToggleComponent::TYPE_NAME.to_owned(),
                json!({ "checked": false }),
            ),
            (
                UiTextInputComponent::TYPE_NAME.to_owned(),
                json!({ "value": "", "max_length": 4 }),
            ),
            (
                UiScrollComponent::TYPE_NAME.to_owned(),
                json!({ "content_height": 1.0 }),
            ),
        ]
        .into_iter()
        .collect(),
        ..EntityData::default()
    })
}

fn run(world: &mut World, source: &str) -> ScriptReport {
    let components = registry();
    let mut screen = ScreenUi::new();
    let input = InputState::default();
    screen
        .update(
            world,
            &components,
            ScreenExtent::new(800.0, 600.0),
            input.presses(),
        )
        .expect("registered");
    let mut sources = ScriptSources::new();
    sources.insert("form.decay", source);
    Scripts::new().advance(
        world,
        &components,
        ScriptFrame::new(&sources, &input, 1.0 / 60.0).with_screen_ui(&screen),
    )
}

#[test]
fn a_script_sets_widgets_within_what_they_hold_and_nothing_reports_a_change() {
    let source = r#"
script Form {
    fn update(dt: f32) {
        Ui.set_checked(this.entity, true);
        Ui.set_input_text(this.entity, "callsign");
        Ui.set_scroll_offset(this.entity, 99.0);
        this.transform.position.x = Ui.scroll_offset(this.entity);
        if Ui.is_checked(this.entity) {
            this.transform.position.y = 1.0;
        }
        if Ui.input_text(this.entity) == "call" {
            this.transform.position.z = 1.0;
        }
        if Ui.changed(this.entity) || Ui.submitted(this.entity) || Ui.is_focused(this.entity) {
            this.transform.position.z = 5.0;
        }
    }
}
"#;
    let mut world = World::default();
    let entity = form(&mut world);
    let report = run(&mut world, source);
    assert!(report.failures.is_empty(), "{:?}", report.failures);

    let data = world.get(entity).expect("there");
    assert_eq!(
        data.components[UiToggleComponent::TYPE_NAME]["checked"],
        true
    );
    assert_eq!(
        data.components[UiTextInputComponent::TYPE_NAME]["value"],
        "call",
        "cut to its max_length"
    );
    let position = data.transform_3d.expect("a transform").position;
    assert!(
        (position[0] - 0.7).abs() < 1.0e-5,
        "clamped to the 0.7 the region can scroll: {position:?}"
    );
    assert!((position[1] - 1.0).abs() < 1.0e-5, "read back checked");
    assert!(
        (position[2] - 1.0).abs() < 1.0e-5,
        "a script's own write is not a change the person made: {position:?}"
    );
}

#[test]
fn a_widget_call_on_the_wrong_element_is_an_error_not_a_default() {
    let source = r"
script Form {
    fn update(dt: f32) {
        Ui.set_checked(this.entity, true);
    }
}
";
    let mut world = World::default();
    let entity = form(&mut world);
    world
        .get_mut(entity)
        .expect("there")
        .components
        .remove(UiToggleComponent::TYPE_NAME);
    let report = run(&mut world, source);
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.to_string().contains(UiToggleComponent::TYPE_NAME)),
        "{:?}",
        report.failures
    );
}
