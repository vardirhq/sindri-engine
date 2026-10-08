//! Real Orbital pickup presentation uses managed tween playback.
use orbital_baked::Run;
use sindri_core::SceneComponent;
use sindri_decay::ScriptComponent;

fn step(run: &mut Run, seconds: f32) {
    let notes = run.step(seconds);
    assert!(notes.is_empty(), "{notes:#?}");
}
#[test]
fn pickup_appears_pauses_and_reaches_its_authored_scale() {
    let mut run = Run::open().expect("project opens");
    step(&mut run, 1.0 / 60.0);
    let disable: Vec<_> = run
        .world
        .entities()
        .filter_map(|(id, data)| {
            data.components
                .contains_key(ScriptComponent::TYPE_NAME)
                .then_some(id)
        })
        .collect();
    for entity in disable {
        run.world.get_mut(entity).expect("entity").disabled = true;
    }
    run.set_board("run_state", 1.0);
    let prefab = run
        .session
        .prefabs()
        .get("prefabs/powerup.prefab")
        .expect("pickup")
        .clone();
    let entity = run.world.spawn_prefab(&prefab).expect("spawn").root;
    run.world
        .get_mut(entity)
        .expect("entity")
        .transform_3d
        .as_mut()
        .expect("transform")
        .position = [20.0, 20.0, 0.0];
    let full = run.world.world_transform(entity).expect("transform").scale;
    step(&mut run, 1.0 / 60.0);
    let initial = run.world.world_transform(entity).expect("pickup").scale;
    assert!((initial[0] - full[0] * 0.35).abs() < 0.00001);
    assert!((initial[2] - full[2]).abs() < 0.00001);
    step(&mut run, 1.0 / 60.0);
    let grown = run.world.world_transform(entity).expect("pickup").scale[0];
    assert!(grown > initial[0] && grown < full[0]);
    run.set_board("run_state", 2.0);
    step(&mut run, 1.0 / 60.0); // pause is requested after this tick's advancement
    let paused = run.world.world_transform(entity).expect("pickup").scale[0];
    for _ in 0..20 {
        step(&mut run, 1.0 / 60.0);
    }
    assert!((run.world.world_transform(entity).expect("pickup").scale[0] - paused).abs() < 0.00001);
    run.set_board("run_state", 1.0);
    for _ in 0..20 {
        step(&mut run, 1.0 / 60.0);
    }
    let final_scale = run.world.world_transform(entity).expect("pickup").scale;
    for (actual, expected) in final_scale.into_iter().zip(full) {
        assert!((actual - expected).abs() < 0.00001);
    }

    // The glow waited for the appearance, then breathes between its two ends
    // for as long as the pickup lasts.
    let alpha = |run: &Run| {
        run.world.get(entity).expect("pickup").components["sindri.shape"]["fill"][3]
            .as_f64()
            .expect("an alpha")
    };
    let mut seen = Vec::new();
    for _ in 0..90 {
        step(&mut run, 1.0 / 60.0);
        seen.push(alpha(&run));
    }
    let low = seen.iter().copied().fold(f64::INFINITY, f64::min);
    let high = seen.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        low >= 0.16 - 1.0e-4 && high <= 0.32 + 1.0e-4,
        "{low}..{high}"
    );
    assert!(high - low > 0.12, "it breathes: {low}..{high}");
    let peak = seen
        .iter()
        .position(|a| (*a - high).abs() < 1.0e-9)
        .expect("a peak");
    assert!(
        seen[peak..].iter().any(|a| *a < high - 0.05),
        "and comes back down"
    );
}
