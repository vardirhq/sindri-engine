use serde_json::json;
use sindri_core::{EntityData, EntityId, Presses, Transform3D, World};
use sindri_scene::{SceneExtractor, ScreenExtent, ScreenUi, UiHierarchy, UiInput};

fn widget(world: &mut World, component: &str, payload: serde_json::Value, y: f32) -> EntityId {
    world.spawn(EntityData {
        transform_3d: Some(Transform3D { position: [0.0,y,0.0], scale:[1.0,0.3,1.0], ..Transform3D::default() }),
        components: [(component.to_owned(),payload)].into_iter().collect(),
        ..EntityData::default()
    })
}

#[test]
fn keyboard_navigation_toggles_and_commits_unicode_once() {
    let extractor = SceneExtractor::new().unwrap();
    let mut world = World::default();
    let toggle = widget(&mut world,"sindri.ui.toggle",json!({"checked":false}),0.4);
    let text = widget(&mut world,"sindri.ui.text_input",json!({"value":"", "max_length":3}),0.0);
    let disabled = widget(&mut world,"sindri.ui.toggle",json!({"checked":false,"disabled":true}),-0.4);
    let mut ui = ScreenUi::new();
    ui.update(&mut world,extractor.components(),ScreenExtent::new(800.0,600.0),&Presses::default()).unwrap();
    ui.read_controls(&mut world,&UiInput { next:true,..UiInput::default() });
    assert_eq!(ui.focused(),Some(toggle));
    ui.read_controls(&mut world,&UiInput { activate:true,..UiInput::default() });
    assert_eq!(world.get(toggle).unwrap().components["sindri.ui.toggle"]["checked"],true);
    assert!(ui.changed(toggle));
    ui.read_controls(&mut world,&UiInput { next:true,text:"å猫🙂extra".to_owned(),..UiInput::default() });
    assert_eq!(ui.focused(),Some(text));
    assert_eq!(world.get(text).unwrap().components["sindri.ui.text_input"]["value"],"å猫🙂");
    ui.read_controls(&mut world,&UiInput { backspace:true,submit:true,..UiInput::default() });
    assert_eq!(world.get(text).unwrap().components["sindri.ui.text_input"]["value"],"å猫");
    assert!(ui.submitted(text));
    // No new input produces no change or submit event.
    ui.update(&mut world,extractor.components(),ScreenExtent::new(800.0,600.0),&Presses::default()).unwrap();
    ui.read_controls(&mut world,&UiInput::default());
    assert!(!ui.changed(text));
    assert!(!ui.submitted(text));
    ui.read_controls(&mut world,&UiInput { next:true,..UiInput::default() });
    assert_eq!(ui.focused(),Some(toggle));
    assert!(!ui.changed(disabled));
    world.despawn_recursive(toggle).unwrap();
    ui.read_controls(&mut world,&UiInput::default());
    assert_eq!(ui.focused(),None);
}

#[test]
fn nested_scroll_moves_content_and_clips_hidden_buttons() {
    let extractor = SceneExtractor::new().unwrap();
    let mut world = World::default();
    let scroll = widget(&mut world,"sindri.ui.scroll",json!({"content_height":2.0,"offset":0.5}),0.0);
    let child = widget(&mut world,"sindri.ui.button",json!({"label":"hidden"}),0.0);
    world.set_parent(child,Some(scroll)).unwrap();
    let hierarchy = UiHierarchy::of(&world,extractor.components()).unwrap();
    assert!((hierarchy.placement(child).unwrap().offset.y-0.5).abs()<1.0e-6);
    let clip = hierarchy.clip_pixels(&world,child,[800,600]).unwrap();
    assert_eq!(clip,[250,255,300,90]);
    let mut ui = ScreenUi::new();
    ui.update(&mut world,extractor.components(),ScreenExtent::new(800.0,600.0),&Presses::default()).unwrap();
    // A hidden child has a placement but is never a pointer target.
    assert_eq!(ui.element_at([0.0,0.5]),None);
}
