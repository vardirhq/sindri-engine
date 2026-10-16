//! The real project spawns a GLB hull, queues controls and replays materialization.
use sindri_runtime::{ProjectRun, STEP};
use std::path::Path;

#[test]
fn convex_prefab_controls_resolve_on_the_next_fixed_step_and_checkpoint_replays() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/model-collision");
    let mut run = ProjectRun::open(&project, [960.0, 540.0]).unwrap();
    let report = run.step(STEP).unwrap();
    assert!(report.notes().is_empty(), "{report:?}");
    assert!(
        report
            .scripts
            .printed
            .iter()
            .any(|line| line.message == "Model spawn controls queued")
    );
    let actor = run
        .world
        .entities()
        .find(|(_, data)| data.name.as_deref() == Some("Prop"))
        .unwrap()
        .0;
    assert!(!run.session.physics3d().world().contains(actor));
    let checkpoint = run.session.checkpoint();
    let snapshot = run.world.clone();
    let authored = run.world.get(actor).unwrap().components.clone();
    assert!(run.step(STEP).unwrap().notes().is_empty());
    let velocity = run
        .session
        .physics3d()
        .world()
        .linear_velocity(actor)
        .unwrap();
    assert!((velocity[0] - 1.0).abs() < 0.001);
    assert!(velocity[1] < -0.1);
    assert!(
        run.session
            .physics3d()
            .world()
            .angular_velocity(actor)
            .unwrap()
            .iter()
            .all(|axis| axis.abs() < 0.001)
    );
    assert_eq!(run.world.get(actor).unwrap().components, authored);
    let first_pose = run.world.world_transform(actor).unwrap();
    run.session.restore(&checkpoint);
    run.world = snapshot;
    assert!(run.step(STEP).unwrap().notes().is_empty());
    assert_eq!(run.world.world_transform(actor), Some(first_pose));
    let mut said = Vec::new();
    for _ in 0..180 {
        let report = run.step(STEP).unwrap();
        assert!(report.notes().is_empty(), "{report:?}");
        said.extend(report.scripts.printed.into_iter().map(|line| line.message));
    }
    assert!(
        said.iter()
            .any(|line| line == "Model prop landed and pushed"),
        "{said:?}"
    );
    assert!(
        said.iter().any(|line| line == "Model prop goal reached"),
        "{said:?}"
    );
}
