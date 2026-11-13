//! What a host puts in scope, and how strictly it is then checked.

use crate::{Environment, FunctionType, HostType, Type, analyze, analyze_with_environment};

#[test]
fn host_globals_are_injected_without_engine_dependencies() {
    let mut environment = Environment::new();
    environment.add_function(
        "delta",
        FunctionType {
            params: vec![],
            return_type: Type::Number,
        },
    );
    environment.add_value("Input", Type::Named("Input".to_owned()));

    let analysis = analyze_with_environment(
        r#"
        script Player {
            fn update() {
                let dt: f32 = delta();
                Input.axis("left", "right");
            }
        }
        "#,
        &environment,
    );

    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
}

/// A host that has described a type gets its members checked, which is the
/// whole point: a misspelled component field is a compile error rather than
/// a runtime failure at frame one.
#[test]
fn a_described_type_checks_its_members() {
    let mut environment = Environment::new();
    environment.add_type(
        "Vec3",
        HostType::new()
            .with_value("x", Type::Number)
            .with_value("y", Type::Number)
            .with_value("z", Type::Number),
    );
    environment.add_type(
        "Transform",
        HostType::new().with_value("position", Type::Named("Vec3".to_owned())),
    );
    environment.add_this_value("transform", Type::Named("Transform".to_owned()));

    let good = analyze_with_environment(
        r"script Player { fn update(dt: f32) { this.transform.position.x += dt; } }",
        &environment,
    );
    assert!(good.diagnostics.is_empty(), "{:?}", good.diagnostics);

    let typo = analyze_with_environment(
        r"script Player { fn update(dt: f32) { this.transfrom.position.x += dt; } }",
        &environment,
    );
    assert!(
        typo.diagnostics
            .iter()
            .any(|d| d.message.contains("has no member `transfrom`")),
        "{:?}",
        typo.diagnostics
    );

    let deep_typo = analyze_with_environment(
        r"script Player { fn update(dt: f32) { this.transform.position.w += dt; } }",
        &environment,
    );
    assert!(
        deep_typo
            .diagnostics
            .iter()
            .any(|d| d.message.contains("`Vec3` has no member `w`")),
        "{:?}",
        deep_typo.diagnostics
    );
}

/// A member's type is a real type, so what is done with it is checked too.
#[test]
fn a_members_type_is_enforced_like_any_other() {
    let mut environment = Environment::new();
    environment.add_type("Sprite", HostType::new().with_value("visible", Type::Bool));
    environment.add_this_value("sprite", Type::Named("Sprite".to_owned()));

    let analysis = analyze_with_environment(
        r"script Player { fn update(dt: f32) { this.sprite.visible = dt; } }",
        &environment,
    );
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|d| d.message.contains("cannot assign `number` to `bool`")),
        "{:?}",
        analysis.diagnostics
    );
}

/// Describing the host is gradual. A named type nobody has described keeps
/// behaving exactly as everything did before types existed, so a host
/// part-way through describing itself does not reject working scripts.
#[test]
fn an_undescribed_type_stays_permissive() {
    let mut environment = Environment::new();
    environment.add_this_value("rigidbody", Type::Named("RigidBody".to_owned()));

    let analysis = analyze_with_environment(
        r"script Player { fn update(dt: f32) { this.rigidbody.anything.at.all = dt; } }",
        &environment,
    );
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
}

/// A container's own field wins over anything the host attached, so the
/// engine growing a name cannot shadow a script's state.
#[test]
fn a_container_field_wins_over_a_host_member() {
    let mut environment = Environment::new();
    environment.add_this_value("speed", Type::Named("Opaque".to_owned()));

    let analysis = analyze_with_environment(
        r"script Player { var speed: f32 = 1.0; fn update(dt: f32) { this.speed += dt; } }",
        &environment,
    );
    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
}

/// Methods on host types are checked against a real signature rather than
/// waved through because the callee was unknown.
#[test]
fn a_host_method_checks_its_arguments() {
    let mut environment = Environment::new();
    environment.add_type(
        "RigidBody",
        HostType::new().with_function(
            "add_impulse",
            FunctionType {
                params: vec![Type::Number, Type::Number],
                return_type: Type::Unit,
            },
        ),
    );
    environment.add_this_value("rigidbody", Type::Named("RigidBody".to_owned()));

    let good = analyze_with_environment(
        r"script Player { fn update() { this.rigidbody.add_impulse(0.0, 1.0); } }",
        &environment,
    );
    assert!(good.diagnostics.is_empty(), "{:?}", good.diagnostics);

    let wrong = analyze_with_environment(
        r"script Player { fn update() { this.rigidbody.add_impulse(0.0); } }",
        &environment,
    );
    assert!(
        wrong
            .diagnostics
            .iter()
            .any(|d| d.message.contains("expected 2 argument(s), found 1")),
        "{:?}",
        wrong.diagnostics
    );
}

/// Decay has no methods. `this.helper()` lowered to a host path call that no
/// host implements, so it failed at runtime looking like the engine's fault;
/// now it says what to write instead.
#[test]
fn reaching_for_a_method_says_what_to_write_instead() {
    let analysis = analyze(
        r"script Player { fn helper() -> f32 { return 1.0; } fn update() { this.helper(); } }",
    );
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|d| d.message.contains("call it as `helper(...)`")),
        "{:?}",
        analysis.diagnostics
    );

    let bare =
        analyze(r"script Player { fn helper() -> f32 { return 1.0; } fn update() { helper(); } }");
    assert!(bare.diagnostics.is_empty(), "{:?}", bare.diagnostics);
}

/// A host may say a named type is also another. One way: a `Bolt` is an
/// `Entity`, and an `Entity` is not a `Bolt`.
#[test]
fn a_subtype_goes_where_its_supertype_does_and_not_back() {
    let mut environment = Environment::new();
    environment.add_type("Entity", HostType::new().with_value("alive", Type::Bool));
    environment.add_type("Bolt", HostType::new().with_value("damage", Type::Number));
    environment.add_supertype("Bolt", "Entity");
    let said = |body: &str| {
        analyze_with_environment(
            &format!("script T {{ fn f(b: Bolt, e: Entity) {{ {body} }} }}"),
            &environment,
        )
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect::<Vec<_>>()
    };
    assert!(said("let x: Entity = b; let same = b == e; let also = e == b;").is_empty());
    assert!(
        said("let x: Bolt = e;")
            .join("")
            .contains("cannot assign `Entity` to `Bolt`")
    );
}

/// A reference to another instance of this very script is answered by the
/// host's description of it, so its functions are callable through it, while
/// `this.helper()` is still refused.
#[test]
fn a_held_reference_to_this_script_is_not_this() {
    let mut environment = Environment::new();
    environment.add_type(
        "Echo",
        HostType::new()
            .with_value("loud", Type::Number)
            .with_function(
                "ping",
                FunctionType {
                    params: vec![Type::Number],
                    return_type: Type::Unit,
                },
            ),
    );
    let said = |body: &str| {
        analyze_with_environment(
            &format!(
                "script Echo {{ var loud: f32 = 1.0; var other: Echo = null; \
                 fn ping(x: f32) {{}} fn f() {{ {body} }} }}"
            ),
            &environment,
        )
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect::<Vec<_>>()
    };
    assert!(said("other.ping(1.0); let l = other.loud;").is_empty());
    assert!(!said("this.ping(1.0);").is_empty());
}

#[test]
fn computed_host_values_reject_whole_and_component_writes_but_allow_copies() {
    let mut environment = Environment::new();
    environment.add_type(
        "Pose",
        HostType::new().with_read_only_value("direction", Type::Vec3),
    );
    environment.add_this_value("pose", Type::Named("Pose".into()));
    for (target, value) in [
        ("this.pose.direction", "Vec3(1.0, 0.0, 0.0)"),
        ("this.pose.direction.x", "1.0"),
    ] {
        let source = format!("script Test {{ fn update() {{ {target} = {value}; }} }}");
        let checked = analyze_with_environment(&source, &environment);
        assert_eq!(checked.diagnostics.len(), 1, "{:?}", checked.diagnostics);
        assert_eq!(checked.diagnostics[0].code, "immutable");
    }
    let copied = analyze_with_environment(
        "script Test { fn update() { var direction = this.pose.direction; direction.x += 1.0; } }",
        &environment,
    );
    assert!(copied.diagnostics.is_empty(), "{:?}", copied.diagnostics);
}

/// A member with named parts: the member alone is the thing's own, and a call
/// with a name answers another of the same type, or `null`.
#[test]
fn a_member_with_parts_is_called_with_a_name() {
    let mut environment = Environment::new();
    environment.add_type(
        "Light2d",
        HostType::new().with_value("radius", Type::Number),
    );
    environment.add_type(
        "Entity",
        HostType::new()
            .with_value("light2d", Type::Named("Light2d".to_owned()))
            .with_parts("light2d"),
    );
    environment.add_this_value("light2d", Type::Named("Light2d".to_owned()));
    environment.add_this_parts("light2d");
    environment.add_this_value("entity", Type::Named("Entity".to_owned()));

    let good = analyze_with_environment(
        r#"
        script Lantern {
            fn update(dt: f32) {
                this.light2d.radius = 10.0;
                let halo = this.entity.light2d("halo");
                if halo != null {
                    halo.radius = 4.6;
                }
                let glow: Light2d = this.light2d("glow");
                if glow == null { return; }
                glow.radius += dt;
            }
        }
        "#,
        &environment,
    );
    assert!(good.diagnostics.is_empty(), "{:?}", good.diagnostics);

    let wrong_member = analyze_with_environment(
        r#"script Lantern { fn update() { let halo = this.entity.light2d("halo"); if halo != null { halo.radiuss = 1.0; } } }"#,
        &environment,
    );
    assert!(
        wrong_member
            .diagnostics
            .iter()
            .any(|d| d.message.contains("has no member `radiuss`")),
        "the part is checked as its type: {:?}",
        wrong_member.diagnostics
    );

    let wrong_argument = analyze_with_environment(
        r"script Lantern { fn update() { let halo = this.entity.light2d(2.0); } }",
        &environment,
    );
    assert!(
        wrong_argument
            .diagnostics
            .iter()
            .any(|d| d.message.contains("takes the part's name as `String`")),
        "{:?}",
        wrong_argument.diagnostics
    );

    let no_argument = analyze_with_environment(
        r"script Lantern { fn update() { let halo = this.entity.light2d(); } }",
        &environment,
    );
    assert!(
        no_argument
            .diagnostics
            .iter()
            .any(|d| d.message.contains("takes one argument")),
        "{:?}",
        no_argument.diagnostics
    );
}

/// A value member without parts is still not callable.
#[test]
fn a_member_without_parts_is_not_called() {
    let mut environment = Environment::new();
    environment.add_type("Transform", HostType::new().with_value("x", Type::Number));
    environment.add_this_value("transform", Type::Named("Transform".to_owned()));
    let called = analyze_with_environment(
        r#"script Player { fn update() { let t = this.transform("x"); } }"#,
        &environment,
    );
    assert!(
        called
            .diagnostics
            .iter()
            .any(|d| d.message.contains("is a value, not a function")),
        "{:?}",
        called.diagnostics
    );
}
