//! A real project opens and plays through the same Session every host uses.

use sindri_runtime::{ProjectRun, STEP};
use std::path::Path;

#[test]
fn the_parented_walker_reaches_a_raised_goal_while_the_camera_orbits() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/transform");
    let mut run = ProjectRun::open(&project, [960.0, 540.0]).expect("transform example opens");
    let find = |name: &str, run: &ProjectRun| {
        run.world
            .entities()
            .find(|(_, data)| data.name.as_deref() == Some(name))
            .map(|(id, _)| id)
            .unwrap()
    };
    let walker = find("Walker", &run);
    let goal = find("Goal", &run);
    let camera = find("Camera", &run);
    let initial_camera = run.world.world_transform(camera).unwrap();
    let mut arrived = false;
    for _ in 0..400 {
        let report = run.step(STEP).expect("session steps");
        assert!(
            report.scripts.failures.is_empty(),
            "{:?}",
            report.scripts.failures
        );
        if report
            .scripts
            .printed
            .iter()
            .any(|line| line.message == "Arrived")
        {
            arrived = true;
            break;
        }
    }
    assert!(arrived, "the feature example runs to its goal");
    let at = run.world.world_transform(walker).unwrap().position;
    let target = run.world.world_transform(goal).unwrap().position;
    assert!(
        at.into_iter()
            .zip(target)
            .all(|(a, b)| (a - b).abs() < 0.01)
    );
    let final_camera = run.world.world_transform(camera).unwrap();
    assert!((final_camera.position[0] - initial_camera.position[0]).abs() > 1.0);
    let direction = final_camera.forward();
    let delta = [
        -final_camera.position[0],
        1.0 - final_camera.position[1],
        -final_camera.position[2],
    ];
    let length = delta.into_iter().map(|v| v * v).sum::<f32>().sqrt();
    assert!(
        direction
            .into_iter()
            .zip(delta)
            .all(|(a, b)| (a - b / length).abs() < 1.0e-4)
    );
}
