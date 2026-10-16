//! Project profile delivery and checkpoint restore use the latest host assets.
use sindri_core::{EntityId, World};
use sindri_runtime::{ProjectRun, STEP};
use std::path::Path;

fn named(world: &World, name: &str) -> EntityId {
    world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some(name))
        .unwrap()
        .0
}
fn coefficients(run: &ProjectRun, entity: EntityId, friction: f32) {
    let values = run.session.physics3d().world().materials(entity).unwrap();
    assert_eq!(values.len(), 1);
    assert!((values[0].friction - friction).abs() < 0.001);
    assert!(values[0].restitution.abs() < 0.001);
}

#[test]
fn loaded_shared_profile_reload_and_restore_preserve_spawned_model_motion() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/model-collision");
    let mut run = ProjectRun::open(&path, [960.0, 540.0]).unwrap();
    for _ in 0..2 {
        assert!(run.step(STEP).unwrap().notes().is_empty());
    }
    let prop = named(&run.world, "Prop");
    let floor = named(&run.world, "Level");
    coefficients(&run, prop, 0.3);
    coefficients(&run, floor, 0.3);
    let checkpoint = run.session.checkpoint();
    let snapshot = run.world.clone();
    let mut profiles = run.session.profiles().clone();
    let profile = profiles.get_mut("materials/slide.profile").unwrap();
    profile
        .values
        .insert("friction".into(), serde_json::json!(0.2));
    profile
        .values
        .insert("restitution".into(), serde_json::json!(0.9));
    run.session.set_profiles(profiles.clone());
    assert!(run.step(STEP).unwrap().notes().is_empty());
    coefficients(&run, prop, 0.2);
    coefficients(&run, floor, 0.2);
    assert!(
        (run.session
            .physics3d()
            .world()
            .linear_velocity(prop)
            .unwrap()[0]
            - 1.0)
            .abs()
            < 0.001
    );
    let advanced = run.world.world_transform(prop);
    run.session.restore(&checkpoint);
    run.world = snapshot;
    assert!(run.step(STEP).unwrap().notes().is_empty());
    assert_eq!(run.world.world_transform(prop), advanced);
    coefficients(&run, prop, 0.2);
    coefficients(&run, floor, 0.2);
    profiles
        .get_mut("materials/slide.profile")
        .unwrap()
        .values
        .insert("friction".into(), serde_json::json!(-1.0));
    run.session.set_profiles(profiles);
    let error = run.step(STEP).unwrap_err();
    assert!(error.contains("materials/slide.profile"), "{error}");
    assert_eq!(run.world.world_transform(prop), advanced);
    coefficients(&run, prop, 0.2);
}
