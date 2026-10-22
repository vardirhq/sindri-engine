//! Live environment defaults, atomic color writes, reference ownership and errors.
use serde_json::json;
use sindri_core::{ComponentSchemaRegistry, EntityData, EntityId, Transform3D, World};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts, check_source};
use sindri_platform::InputState;

fn fixture(payload: serde_json::Value) -> (World, EntityId, ComponentSchemaRegistry) {
    let mut registry = ComponentSchemaRegistry::default();
    registry.register::<ScriptComponent>("Script").unwrap();
    let mut world = World::default();
    let entity = world.spawn(EntityData {
        name: Some("World".into()),
        transform_3d: Some(Transform3D::default()),
        components: [
            (
                "sindri.script".into(),
                json!({"source":"environment.decay", "script":"S"}),
            ),
            ("sindri.environment".into(), payload),
        ]
        .into(),
        ..Default::default()
    });
    (world, entity, registry)
}
fn run(world: &mut World, registry: &ComponentSchemaRegistry, body: &str) -> String {
    let mut sources = ScriptSources::new();
    sources.insert(
        "environment.decay",
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
fn defaults_and_fractional_writes_share_self_and_reference_access() {
    let payload = json!({"ambient_intensity":1,"shadows":{"enabled":true}});
    let (mut world, entity, registry) = fixture(payload);
    let result = run(
        &mut world,
        &registry,
        r#"
        let world = World.find("World");
        if this.environment.ambient_color.a == 1.0 && this.environment.ambient_color.r == 1.0 {
            this.environment.ambient_intensity = 0.125;
            world.environment.ambient_color = Color(0.2, 0.3, 0.4);
            this.entity.environment.ambient_color.b = 0.75;
            this.environment.background = Color(0.125, 0.25, 0.5, 0.75);
            let copied = world.environment.background;
            this.environment.background = copied;
            world.environment.background.a = 0.5;
            if world.environment.background.a == 0.5 { this.transform.position.x = 1.0; }
        }
    "#,
    );
    assert!(result.is_empty(), "{result}");
    let data = world.get(entity).unwrap();
    assert!((data.transform_3d.unwrap().position[0] - 1.0).abs() < f32::EPSILON);
    let payload = &data.components["sindri.environment"];
    assert_eq!(payload["ambient_color"], json!([0.2, 0.3, 0.75]));
    assert_eq!(payload["background"], json!([0.125, 0.25, 0.5, 0.5]));
    assert_eq!(payload["ambient_intensity"], json!(0.125));
    assert_eq!(payload["shadows"], json!({"enabled":true}));
    assert!(payload.get("fog").is_none());
}

#[test]
fn omitted_channels_use_defaults_and_authored_color_ranges_are_retained() {
    let (mut world, entity, registry) = fixture(json!({}));
    let result = run(
        &mut world,
        &registry,
        "this.environment.background.a = 0.25; this.environment.ambient_color.r = -0.5;",
    );
    assert!(result.is_empty(), "{result}");
    let payload = &world.get(entity).unwrap().components["sindri.environment"];
    let environment: sindri_scene::EnvironmentComponent =
        serde_json::from_value(payload.clone()).unwrap();
    assert!((environment.background[0] - 0.035).abs() < 1e-6);
    assert!((environment.background[3] - 0.25).abs() < 1e-6);
    assert_eq!(payload["ambient_color"], json!([-0.5, 1.0, 1.0]));
    assert!(payload.get("ambient_intensity").is_none());
}

#[test]
fn refused_writes_keep_the_entire_payload() {
    for body in [
        "this.environment.ambient_intensity = -0.1;",
        "this.environment.ambient_intensity = 0.0 / 0.0;",
        "this.environment.background = Color(0.1, 0.2, 0.0 / 0.0);",
        "this.environment.background.r = 1.0 / 0.0;",
        "this.environment.ambient_color = Color(0.1, 0.2, 0.3, 0.5);",
        "this.environment.ambient_color.a = 0.5;",
        "this.environment.ambient_color.g = 100000000000000000000000000000000000000000000000000000.0;",
    ] {
        let payload = json!({"ambient_intensity":1,"ambient_color":[1,0.2,0.3]});
        let (mut world, entity, registry) = fixture(payload.clone());
        let result = run(&mut world, &registry, body);
        assert!(!result.is_empty(), "{body}");
        assert_eq!(
            world.get(entity).unwrap().components["sindri.environment"],
            payload,
            "{body}: {result}"
        );
    }
}

#[test]
fn absent_stale_and_invalid_components_report_errors_and_types_are_checked() {
    let (mut world, entity, registry) = fixture(json!({"shadows":{"map_size":4096}}));
    let result = run(
        &mut world,
        &registry,
        "this.environment.ambient_intensity = 0.5;",
    );
    assert!(result.contains("shadows.map_size"), "{result}");
    world
        .get_mut(entity)
        .unwrap()
        .components
        .remove("sindri.environment");
    let result = run(
        &mut world,
        &registry,
        "let value = this.environment.background;",
    );
    assert!(result.contains("sindri.environment"), "{result}");
    let target = world.spawn(EntityData {
        name: Some("Target".into()),
        ..Default::default()
    });
    let result = run(
        &mut world,
        &registry,
        r#"let target = World.find("Target"); World.despawn(target); target.environment.background = Color(0.1, 0.2, 0.3);"#,
    );
    assert!(
        !result.is_empty() && world.get(target).is_none(),
        "{result}"
    );
    for body in [
        "this.environment.ambient_intensity = false;",
        "this.environment.background = Vec3(1.0, 0.0, 0.0);",
    ] {
        let source = format!("script S {{ fn update(dt: f32) {{ {body} }} }}");
        assert!(!check_source(&source).diagnostics.is_empty());
    }
}

/// Fog, the grade and the bloom are as live as the ambient light: a game can
/// roll a mist in, warm the grade at dusk or let the lamps glow harder, and
/// the next frame shows it. A section the scene never authored is filled from
/// its defaults, so one write keeps every other field as it was.
#[test]
fn fog_grade_and_bloom_are_live_and_keep_their_other_fields() {
    let payload = json!({"bloom":{"enabled":true,"passes":3}});
    let (mut world, entity, registry) = fixture(payload);
    let result = run(
        &mut world,
        &registry,
        r#"
        let world = World.find("World");
        if !world.environment.fog.enabled && world.environment.bloom.enabled {
            world.environment.fog.enabled = true;
            world.environment.fog.color = Color(0.5, 0.6, 0.7);
            this.environment.fog.color.b = 0.75;
            world.environment.fog.density = 0.25;
            world.environment.fog.start = 4.0;
            world.environment.post_process.exposure = -0.5;
            world.environment.post_process.saturation = 0.75;
            world.environment.post_process.vignette = 0.5;
            world.environment.bloom.intensity = 1.5;
            world.environment.bloom.threshold = 0.5;
            if world.environment.fog.density == 0.25 && world.environment.bloom.intensity == 1.5 {
                this.transform.position.x = 1.0;
            }
        }
    "#,
    );
    assert!(result.is_empty(), "{result}");
    let data = world.get(entity).unwrap();
    assert!(
        (data.transform_3d.unwrap().position[0] - 1.0).abs() < f32::EPSILON,
        "the writes read back"
    );
    let payload = &data.components["sindri.environment"];
    assert_eq!(payload["fog"]["enabled"], json!(true));
    assert_eq!(payload["fog"]["color"], json!([0.5, 0.6, 0.75]));
    assert_eq!(payload["fog"]["density"], json!(0.25));
    assert_eq!(payload["fog"]["start"], json!(4.0));
    assert!(
        payload["fog"]["distance"].as_f64().is_some(),
        "untouched fields keep their default"
    );
    assert_eq!(payload["post_process"]["exposure"], json!(-0.5));
    assert_eq!(payload["post_process"]["saturation"], json!(0.75));
    assert_eq!(payload["post_process"]["vignette"], json!(0.5));
    assert_eq!(payload["bloom"]["intensity"], json!(1.5));
    assert_eq!(payload["bloom"]["threshold"], json!(0.5));
    assert_eq!(
        payload["bloom"]["passes"],
        json!(3),
        "an authored field is kept"
    );
}

#[test]
fn out_of_range_fog_and_grade_are_refused_and_leave_the_payload() {
    let (mut world, entity, registry) = fixture(json!({}));
    for (body, field) in [
        ("this.environment.fog.density = 2.0;", "fog.density"),
        (
            "this.environment.post_process.exposure = 12.0;",
            "post_process.exposure",
        ),
        (
            "this.environment.fog.color = Color(0.5, 0.5, 0.5, 0.5);",
            "alpha one",
        ),
    ] {
        let result = run(&mut world, &registry, body);
        assert!(result.contains(field), "{body}: {result}");
        assert_eq!(
            world.get(entity).unwrap().components["sindri.environment"],
            json!({}),
            "{body} left nothing behind"
        );
    }
}
