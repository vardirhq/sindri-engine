//! The canvas shows the screen UI as it will be played: styled by the
//! project's stylesheets, and hidden while it is switched off.

use std::path::PathBuf;

use sindri_mobile::{Editing, OpenedScene};

fn lamplighter() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../games/lamplighter")
}

#[test]
fn the_wilds_hud_is_drawn_styled_and_can_be_hidden() {
    let opened = OpenedScene::open(&lamplighter(), "wilds.scene").unwrap();
    assert!(opened.styles.is_some(), "Lamplighter has stylesheets");
    let mut editing = Editing::for_scene(&opened).unwrap();
    let pack = opened
        .world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("pack-panel"))
        .map(|(entity, _)| entity)
        .expect("the pack's panel");
    let size = [390.0, 844.0];
    let authored = opened.world.get(pack).unwrap().clone();

    let shown = editing
        .presented(&opened.world, size, 1.5, opened.styles.as_ref())
        .unwrap();
    let styled = shown.get(pack).unwrap();
    assert_ne!(
        styled.components["sindri.ui.shape"]["fill"],
        authored.components["sindri.ui.shape"]["fill"],
        "the panel takes the stylesheet's colour"
    );
    assert_eq!(
        opened.world.get(pack).unwrap(),
        &authored,
        "and what is edited is untouched"
    );

    editing.screen_ui = false;
    let hidden = editing
        .presented(&opened.world, size, 1.5, opened.styles.as_ref())
        .unwrap();
    assert!(
        hidden
            .get(pack)
            .unwrap()
            .components
            .keys()
            .all(|name| !name.starts_with("sindri.ui.")),
        "switched off, it is not drawn"
    );
}
