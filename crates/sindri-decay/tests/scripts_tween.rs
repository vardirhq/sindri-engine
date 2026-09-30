//! Managed playback through the real typed Decay host.
use serde_json::json;
use sindri_core::{
    ComponentSchemaRegistry, EntityData, EntityId, SceneComponent, Transform3D, World,
};
use sindri_decay::{ScriptComponent, ScriptFrame, ScriptSources, Scripts, check_source};
use sindri_platform::InputState;

fn fixture(
    source: &str,
) -> (
    World,
    ComponentSchemaRegistry,
    ScriptSources,
    Scripts,
    EntityId,
) {
    let mut world = World::default();
    let mut registry = ComponentSchemaRegistry::default();
    registry
        .register::<ScriptComponent>("Script")
        .expect("register");
    let entity = world.spawn(EntityData {
        transform_3d: Some(Transform3D::default()),
        components: [(
            ScriptComponent::TYPE_NAME.to_owned(),
            json!({"source": "test.decay", "script": "Test"}),
        )]
        .into_iter()
        .collect(),
        ..EntityData::default()
    });
    let mut sources = ScriptSources::new();
    sources.insert("test.decay", source);
    (world, registry, sources, Scripts::new(), entity)
}
fn step(
    at: &mut (
        World,
        ComponentSchemaRegistry,
        ScriptSources,
        Scripts,
        EntityId,
    ),
    delta: f32,
) -> Vec<String> {
    let report = at.3.advance(
        &mut at.0,
        &at.1,
        ScriptFrame::new(&at.2, &InputState::default(), delta),
    );
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    report
        .printed
        .into_iter()
        .map(|line| line.message)
        .collect()
}
#[test]
fn playback_pause_resume_cancel_restart_and_aliases() {
    let mut at = fixture(
        r#"
        script Test {
            var t: NumberTween = null;
            var frame: f32 = 0.0;
            fn start() { this.t = Tween.number(2.0, 10.0, 1.0, "linear"); }
            fn update(dt: f32) {
                print(Tween.number_value(this.t));
                if this.frame == 1.0 { let copy = this.t; Tween.pause(copy); }
                if this.frame == 2.0 { print(Tween.is_paused(this.t)); Tween.resume(this.t); }
                if this.frame == 3.0 { Tween.cancel(this.t); }
                if this.frame == 4.0 {
                    print(Tween.is_cancelled(this.t)); print(Tween.is_done(this.t));
                    Tween.resume(this.t);
                }
                if this.frame == 5.0 { Tween.restart(this.t); }
                this.frame += 1.0;
            }
        }
    "#,
    );
    assert_eq!(step(&mut at, 0.25), ["2"]);
    assert_eq!(step(&mut at, 0.25), ["4"]);
    assert_eq!(step(&mut at, 0.25), ["4", "true"]);
    assert_eq!(step(&mut at, 0.25), ["6"]);
    assert_eq!(step(&mut at, 0.25), ["6", "true", "false"]);
    assert_eq!(step(&mut at, 0.25), ["6"]);
    assert_eq!(step(&mut at, 0.25), ["4"]);
    assert_eq!(step(&mut at, 2.0), ["10"]);
}
#[test]
fn vectors_colours_zero_duration_and_exact_completion() {
    let mut at = fixture(
        r#"
        script Test {
            var v: Vec3Tween = null;
            var c: ColorTween = null;
            fn start() {
                this.v = Tween.vec3(Vec3(0.0, 2.0, 4.0), Vec3(2.0, 4.0, 6.0), 1.0, "linear");
                this.c = Tween.color(Color(0.0, 0.0, 0.0, 0.0), Color(1.0, 1.0, 1.0, 1.0), 1.0, "linear");
                let z = Tween.number(1.0, 7.0, 0.0, "ease");
                print(Tween.number_value(z)); print(Tween.is_done(z));
                let v2 = Tween.vec2(Vec2(1.0, 2.0), Vec2(3.0, 4.0), 0.0, "linear");
                let result = Tween.vec2_value(v2); print(result.y);
            }
            fn update(dt: f32) {
                this.transform.position = Tween.vec3_value(this.v);
                let tint = Tween.color_value(this.c);
                print(this.transform.position.x); print(tint.a); print(Tween.is_done(this.v));
            }
        }
    "#,
    );
    assert_eq!(step(&mut at, 0.5), ["7", "true", "4", "0", "0", "false"]);
    assert_eq!(step(&mut at, 0.5), ["1", "0.5", "false"]);
    assert_eq!(step(&mut at, 0.5), ["2", "1", "true"]);
    assert_eq!(step(&mut at, 10.0), ["2", "1", "true"]);
}
#[test]
fn signatures_reject_wrong_endpoints_and_wrong_handles() {
    for call in [
        r#"Tween.number(Vec2(0.0, 0.0), 1.0, 1.0, "linear")"#,
        r#"Tween.vec3(Vec3(0.0, 0.0, 0.0), Vec2(1.0, 1.0), 1.0, "linear")"#,
        r"Tween.pause(this.entity)",
        r#"Tween.number_value(Tween.color(Color(1.0, 1.0, 1.0), Color(0.0, 0.0, 0.0), 1.0, "linear"))"#,
    ] {
        let source = format!("script Test {{ fn start() {{ {call}; }} }}");
        assert!(
            !check_source(&source).diagnostics.is_empty(),
            "accepted {call}"
        );
    }
}
#[test]
fn invalid_inputs_and_disposed_handles_report_useful_errors() {
    for (call, expected) in [
        (r#"Tween.number(0.0, 1.0, -1.0, "linear");"#, "non-negative"),
        (r#"Tween.number(0.0, 1.0, 1.0 / 0.0, "linear");"#, "finite"),
        (r#"Tween.number(0.0, 1.0, 0.0 / 0.0, "linear");"#, "finite"),
        (
            r#"Tween.number(0.0 / 0.0, 1.0, 1.0, "linear");"#,
            "finite endpoints",
        ),
        (r#"Tween.number(0.0, 1.0, 1.0, "typo");"#, "unknown easing"),
        (
            r#"let t = Tween.number(0.0, 1.0, 1.0, "linear"); Tween.dispose(t); Tween.number_value(t);"#,
            "disposed",
        ),
    ] {
        let mut at = fixture(&format!("script Test {{ fn start() {{ {call} }} }}"));
        let report = at.3.advance(
            &mut at.0,
            &at.1,
            ScriptFrame::new(&at.2, &InputState::default(), 0.1),
        );
        assert_eq!(report.failures.len(), 1, "{:?}", report.failures);
        assert!(
            report.failures[0].to_string().contains(expected),
            "{:?}",
            report.failures
        );
    }
}

#[test]
fn replacement_starts_at_displayed_value_and_owner_removal_invalidates_aliases() {
    let mut at = fixture(
        r#"
        script Test {
            var t: NumberTween = null;
            var old: NumberTween = null;
            var frame: f32 = 0.0;
            fn start() { this.t = Tween.number(0.0, 10.0, 1.0, "linear"); }
            fn update(dt: f32) {
                print(Tween.number_value(this.t));
                if this.frame == 1.0 {
                    let current = Tween.number_value(this.t);
                    this.old = this.t;
                    Tween.cancel(this.old);
                    this.t = Tween.number(current, 0.0, 1.0, "linear");
                    print(Tween.number_value(this.t));
                    Tween.dispose(this.old);
                }
                this.frame += 1.0;
            }
        }
    "#,
    );
    assert_eq!(step(&mut at, 0.5), ["0"]);
    assert_eq!(step(&mut at, 0.5), ["5", "5"]);
    assert_eq!(step(&mut at, 0.5), ["2.5"]);
    at.0.despawn_recursive(at.4).expect("despawn");
    assert!(step(&mut at, 0.5).is_empty());
    at.3.clear();
}
