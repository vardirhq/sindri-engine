//! The built-in component registry, checked.

use std::collections::BTreeSet;

use super::builtin_components;

/// Every built-in says what it consists of.
///
/// The check that stops the next component being registered with
/// `register` by habit: without a field template its panel shows whatever
/// fields its payload happens to carry, which is how one added by a button
/// and one authored by hand became different-looking components.
#[test]
fn every_built_in_component_names_its_fields() {
    let components = builtin_components().expect("the built-in schemas register");
    for metadata in components.registered_components() {
        let fields = components
            .fields(&metadata.type_name)
            .unwrap_or_else(|| panic!("{} has no field template", metadata.type_name));
        assert!(
            fields.as_object().is_some_and(|fields| !fields.is_empty()),
            "{} has an empty field template",
            metadata.type_name
        );
    }
}

/// Which components have no honest blank, listed so that adding a sixth is
/// a decision rather than an omission.
///
/// Each of these names something only a project can supply — a font, a
/// sheet, another entity, or a clip. They inspect like any other component;
/// they simply cannot be created without a host that knows what is lying
/// beside the scene.
#[test]
fn only_the_components_that_name_a_project_asset_lack_a_default() {
    let components = builtin_components().expect("the built-in schemas register");
    let uncreatable: BTreeSet<&str> = components
        .registered_components()
        .filter(|metadata| components.default_payload(&metadata.type_name).is_none())
        .map(|metadata| metadata.type_name.as_str())
        .collect();
    assert_eq!(
        uncreatable,
        BTreeSet::from([
            "sindri.animation.sprite",
            "sindri.audio.source",
            "sindri.grid.occupant",
            "sindri.grid.placement",
            "sindri.model",
            "sindri.ui.text",
        ])
    );
}

#[test]
fn a_fresh_tile_volume_is_an_empty_addable_component() {
    let components = builtin_components().expect("the built-in schemas register");
    let payload = components
        .default_payload("sindri.tile_volume")
        .expect("tile volume is addable");
    assert_eq!(payload["tileset"], "");
    assert_eq!(payload["cells"], serde_json::json!([]));
    components
        .validate_payload("sindri.tile_volume", payload)
        .expect("the blank tile volume is valid");
}
