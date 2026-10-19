use super::*;
use serde_json::json;

#[test]
fn profiles_validate_typed_values_and_reject_recursive_or_unknown_fields() {
    let starter = render_material_template();
    assert!(
        render_material_profile("shared.profile", &starter)
            .unwrap()
            .is_some()
    );
    for (key, value) in [
        ("roughness", json!(-1)),
        ("metallic", json!(2)),
        ("base_color", json!([1, 0, 0])),
        ("sidedness", json!("both")),
        ("profile", json!("other.profile")),
        ("override_factors", json!(true)),
        ("texture", json!("x.png")),
        ("emissive", json!([-1, 0, 0])),
    ] {
        let mut profile = starter.clone();
        profile.values.insert(key.into(), value);
        let error = render_material_profile("shared.profile", &profile).unwrap_err();
        assert!(error.to_string().contains("shared.profile"), "{error}");
    }
    let mut other = starter;
    other.profile_type = "game_tuning".into();
    other.values.insert("anything".into(), json!({"game":true}));
    assert!(
        render_material_profile("game.profile", &other)
            .unwrap()
            .is_none()
    );
}

#[test]
fn profile_inheritance_local_overrides_and_missing_sources_are_explicit() {
    let mut profile = render_material_template();
    profile.values.insert("metallic".into(), json!(0.75));
    profile.values.insert("sidedness".into(), json!("double"));
    let sources = RenderMaterialSources::from_profiles([("gold.profile", &profile)]).unwrap();
    let mut component: MaterialComponent = serde_json::from_value(json!({
        "profile":"gold.profile", "metallic":0.2, "sidedness":"single"
    }))
    .unwrap();
    let authored = sources.resolve(&component).unwrap();
    assert!((authored.factors.metallic - 0.75).abs() < f32::EPSILON);
    assert_eq!(authored.sidedness, MaterialSidedness::Double);
    component.override_factors = true;
    assert_eq!(
        sources.resolve(&component).unwrap().factors,
        component.factors
    );
    assert_eq!(
        sources.resolve(&component).unwrap().sidedness,
        MaterialSidedness::Double
    );
    component.override_sidedness = true;
    assert_eq!(
        sources.resolve(&component).unwrap().sidedness,
        MaterialSidedness::Single
    );
    assert!(matches!(
        RenderMaterialSources::default().resolve(&component),
        Err(RenderMaterialError::Missing(_))
    ));
    component.profile.clear();
    assert_eq!(
        sources.resolve(&component).unwrap().factors,
        component.factors
    );
    for reference in ["../gold.profile", "/gold.profile", "gold.png"] {
        assert!(serde_json::from_value::<MaterialComponent>(json!({"profile":reference})).is_err());
    }
}
