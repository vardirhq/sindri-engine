//! A named part (`sindri.light2d#halo`) is shown and edited on the phone as
//! its type is, under its own key.

use std::path::Path;

use serde_json::{Value, json};
use sindri_core::{EntityData, Transform3D};
use sindri_mobile::{Editing, OpenedScene};

#[test]
fn a_named_light_part_is_shown_with_its_defaults_and_edited_under_its_own_key() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter");
    let mut opened = OpenedScene::open(&project, "world.scene").expect("the scene opens");
    let lantern = opened.world.spawn(EntityData {
        name: Some("Lantern".to_owned()),
        transform_3d: Some(Transform3D::default()),
        components: [
            ("sindri.light2d".to_owned(), json!({ "radius": 10.0 })),
            ("sindri.light2d#halo".to_owned(), json!({ "radius": 4.6 })),
        ]
        .into(),
        ..EntityData::default()
    });
    let mut editing = Editing::for_scene(&opened).expect("editing starts");
    editing.selected = Some(lantern);
    let shown = editing.inspector(&opened.world, &opened.extractor, &opened.assets.sheets);
    let halo = shown["components"]
        .as_array()
        .expect("components")
        .iter()
        .find(|component| component["type"] == "sindri.light2d#halo")
        .expect("the halo is shown");
    assert_eq!(halo["values"]["radius"], json!(4.6));
    // Its type's defaults fill in what the scene does not say.
    assert_eq!(halo["values"]["enabled"], json!(true));
    assert_eq!(halo["meanings"]["color"]["kind"], json!("color"));

    editing
        .set_field(
            &mut opened.world,
            &opened.extractor,
            "sindri.light2d#halo",
            "radius",
            json!(3.0),
            false,
        )
        .expect("the halo's reach is set");
    let components = &opened.world.get(lantern).expect("the lantern").components;
    assert_eq!(components["sindri.light2d#halo"]["radius"], json!(3.0));
    assert_eq!(
        components["sindri.light2d"]["radius"],
        json!(10.0),
        "the beam is untouched"
    );
    // A value its type refuses is refused on the part too.
    assert!(
        editing
            .set_field(
                &mut opened.world,
                &opened.extractor,
                "sindri.light2d#halo",
                "radius",
                json!("far"),
                false
            )
            .is_err()
    );
    editing.undo(&mut opened.world).expect("the edit undoes");
    let halo: &Value =
        &opened.world.get(lantern).expect("the lantern").components["sindri.light2d#halo"];
    assert_eq!(halo["radius"], json!(4.6));
}
