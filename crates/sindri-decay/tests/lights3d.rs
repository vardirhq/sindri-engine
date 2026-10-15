//! Typed light defaults, checked atomic writes, self/reference addressing and errors.
use serde_json::json;
use sindri_core::{ComponentSchemaRegistry, EntityData, EntityId, Transform3D, World};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts, check_source};
use sindri_platform::InputState;

fn fixture(payload: serde_json::Value) -> (World, EntityId, ComponentSchemaRegistry) {
    let mut registry = ComponentSchemaRegistry::default();
    registry.register::<ScriptComponent>("Script").unwrap();
    let mut world = World::default();
    let entity = world.spawn(EntityData {
        name: Some("Lamp".into()),
        transform_3d: Some(Transform3D::default()),
        components: [
            (
                "sindri.script".into(),
                json!({"source":"light.decay", "script":"S"}),
            ),
            ("sindri.light".into(), payload),
        ]
        .into(),
        ..Default::default()
    });
    (world, entity, registry)
}
fn run(world: &mut World, registry: &ComponentSchemaRegistry, body: &str) -> String {
    let mut sources = ScriptSources::new();
    sources.insert(
        "light.decay",
        format!("script S {{ fn update(dt: f32) {{ {body} }} }}"),
    );
    let report = Scripts::new().advance(
        world,
        registry,
        ScriptFrame::new(&sources, &InputState::default(), 0.1),
    );
    if report.is_quiet() {
        String::new()
    } else {
        format!("{report:?}")
    }
}

#[test]
fn defaults_self_and_reference_writes_are_typed_and_do_not_disable_the_script() {
    let (mut world, entity, registry) = fixture(json!({"kind":"spot", "intensity":1}));
    let result = run(
        &mut world,
        &registry,
        r##"
        let lamp = World.find("Lamp");
        if this.light.enabled && this.light.range == 8.0 && this.light.color.a == 1.0 {
            this.light.color = Color("#ff8000");
            lamp.light.color.b = 0.5;
            this.entity.light.intensity = 0.5;
            lamp.light.outer_angle = 0.9;
            this.light.inner_angle = 0.5;
            this.light.enabled = false;
            if !lamp.light.enabled { this.transform.position.x = 1.0; }
            this.light.enabled = true;
        }
    "##,
    );
    assert!(result.is_empty(), "{result}");
    let data = world.get(entity).unwrap();
    assert!(world.is_active(entity));
    assert!((data.transform_3d.unwrap().position[0] - 1.0).abs() < 1e-6);
    let light = &data.components["sindri.light"];
    assert_eq!(light["enabled"], true);
    assert!((light["intensity"].as_f64().unwrap() - 0.5).abs() < 1e-6);
    assert!((light["color"][2].as_f64().unwrap() - 0.5).abs() < 1e-6);
    assert!(
        light.get("range").is_none(),
        "unrelated omitted fields stay omitted"
    );
}

#[test]
fn invalid_values_and_atomic_colors_keep_the_entire_previous_payload() {
    for kind in ["directional", "point", "spot"] {
        for body in [
            "this.light.intensity = -1.0;",
            "this.light.range = 0.0;",
            "this.light.range = 1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000.0;",
            "this.light.intensity = 0.0 / 0.0;",
            "this.light.inner_angle = 2.0;",
            "this.light.outer_angle = 0.1;",
            "this.light.color = Color(0.1, -1.0, 0.9);",
            "this.light.color = Color(0.1, 0.2, 0.3, 0.5);",
            "this.light.color.g = -0.1;",
            "this.light.color.a = 0.5;",
        ] {
            let payload = json!({"kind":kind,"color":[1,0.2,0.3]});
            let (mut world, entity, registry) = fixture(payload.clone());
            let result = run(&mut world, &registry, body);
            assert!(!result.is_empty(), "{kind}: {body}");
            assert_eq!(
                world.get(entity).unwrap().components["sindri.light"],
                payload,
                "{body}: {result}"
            );
        }
    }
}

#[test]
fn absent_lights_and_stale_references_report_errors_and_wrong_types_do_not_compile() {
    let (mut world, entity, registry) = fixture(json!({}));
    world
        .get_mut(entity)
        .unwrap()
        .components
        .remove("sindri.light");
    assert!(
        run(&mut world, &registry, "this.light.intensity = 2.0;").contains("needs a sindri.light")
    );
    let (mut world, _, registry) = fixture(json!({}));
    assert!(
        run(
            &mut world,
            &registry,
            "let lamp = World.find(\"Lamp\"); World.despawn(lamp); lamp.light.enabled = false;"
        )
        .contains("no longer exists")
    );
    for body in [
        "this.light.enabled = 1.0;",
        "this.light.intensity = true;",
        "this.light.color = Vec3(1.0,0.0,0.0);",
    ] {
        let source = format!("script S {{ fn start() {{ {body} }} }}");
        assert!(!check_source(&source).diagnostics.is_empty(), "{body}");
    }
}

#[test]
fn a_disabled_light_keeps_its_own_script_ticking_and_can_switch_back_on() {
    let (mut world, entity, registry) = fixture(json!({"kind":"point"}));
    let mut sources = ScriptSources::new();
    sources.insert("light.decay", "script S { var ticks: f32 = 0.0; fn update(dt: f32) { ticks += 1.0; this.light.enabled = ticks > 1.0; this.transform.position.x = ticks; } }");
    let mut scripts = Scripts::new();
    for tick in 1..=2 {
        let report = scripts.advance(
            &mut world,
            &registry,
            ScriptFrame::new(&sources, &InputState::default(), 0.1),
        );
        assert!(report.is_quiet(), "{report:?}");
        assert!(world.is_active(entity));
        let data = world.get(entity).unwrap();
        assert_eq!(data.components["sindri.light"]["enabled"], tick == 2);
        assert!(
            (data.transform_3d.unwrap().position[0] - f32::from(u8::try_from(tick).unwrap())).abs()
                < 1e-6
        );
    }
}
