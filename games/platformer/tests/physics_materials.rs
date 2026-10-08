//! Shared wood actually controls the game's solved impacts.

use platformer::Run;
use sindri_core::ProfileDocument;

#[test]
fn crate_and_planks_share_wood_with_an_explicit_plank_override() {
    let mut run = Run::open().unwrap();
    assert!(run.step(1.0 / 60.0).is_empty());
    for id in ["wind-crate", "one-way-planks"] {
        for material in run
            .physics()
            .world()
            .materials(run.entity(id).unwrap())
            .unwrap()
        {
            assert!((material.friction - 0.3).abs() < f32::EPSILON);
            let restitution = if id == "wind-crate" { 0.1 } else { 0.0 };
            assert!((material.restitution - restitution).abs() < f32::EPSILON);
        }
    }
}

fn rebound(restitution: f32) -> f32 {
    let mut run = Run::open().unwrap();
    let profile = ProfileDocument::from_json(&serde_json::json!({"format_version": 1, "type": "physics_material", "values": {"friction": 0.3, "restitution": restitution}}).to_string()).unwrap();
    // The wood itself changes, as an author editing the profile would: the
    // session reads its materials from the profiles every step.
    let mut profiles = run.session.profiles().clone();
    profiles.insert("materials/wood.profile", profile);
    run.session.set_profiles(profiles);
    let entity = run.entity("wind-crate").unwrap();
    let mut up = 0.0_f32;
    for _ in 0..100 {
        let notes = run.step(1.0 / 60.0);
        assert!(notes.is_empty(), "{notes:?}");
        up = up.max(run.physics().world().linear_velocity(entity).unwrap()[1]);
    }
    up
}

#[test]
fn changing_the_shared_asset_changes_the_crates_rebound() {
    assert!(rebound(1.0) > rebound(0.0) + 0.5);
}
