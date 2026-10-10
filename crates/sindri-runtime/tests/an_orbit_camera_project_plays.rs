//! Camera behavior is proven through the same Session used by every host.

use sindri_runtime::{ProjectRun, STEP};
use std::path::Path;

#[test]
fn resizing_an_orbit_barrier_changes_collision_on_the_next_session_step() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/orbit");
    let mut run = ProjectRun::open(&path, [960.0, 540.0]).unwrap();
    let named = |name: &str, run: &ProjectRun| {
        run.world
            .entities()
            .find(|(_, data)| data.name.as_deref() == Some(name))
            .map(|(id, _)| id)
            .unwrap()
    };
    let barrier = named("Barrier", &run);
    let camera = named("Camera", &run);
    let report = run.step(STEP).unwrap();
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    let before = run.world.world_transform(camera).unwrap().position;
    assert!(before[2] < 0.8);
    run.world
        .get_mut(barrier)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .scale = [0.02, 4.0, 0.4];
    assert!(
        run.world
            .world_transform(camera)
            .unwrap()
            .position
            .iter()
            .zip(before)
            .all(|(a, b)| (a - b).abs() < f32::EPSILON)
    );
    for _ in 0..30 {
        let report = run.step(STEP).unwrap();
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert!(
            report.scripts.failures.is_empty(),
            "{:?}",
            report.scripts.failures
        );
    }
    assert!(
        run.world.world_transform(camera).unwrap().position[2] > 1.5,
        "camera recovers after the wall narrows"
    );
    run.world
        .get_mut(barrier)
        .unwrap()
        .transform_3d
        .as_mut()
        .unwrap()
        .scale = [10.0, 4.0, 0.4];
    let report = run.step(STEP).unwrap();
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    assert!(
        run.world.world_transform(camera).unwrap().position[2] < 0.8,
        "restored wall immediately pulls the camera in"
    );
}

#[test]
fn an_orbit_camera_pulls_in_recovers_and_follows_the_walker_to_its_goal() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/orbit");
    let mut run = ProjectRun::open(&path, [960.0, 540.0]).expect("orbit project opens");
    let find = |name: &str, run: &ProjectRun| {
        run.world
            .entities()
            .find(|(_, data)| data.name.as_deref() == Some(name))
            .map(|(id, _)| id)
            .unwrap()
    };
    let camera = find("Camera", &run);
    let walker = find("Walker", &run);
    let barrier = find("Barrier", &run);
    let report = run.step(STEP).expect("session steps");
    assert!(
        report.scripts.failures.is_empty(),
        "{:?}",
        report.scripts.failures
    );
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    let pose = run.world.world_transform(camera).unwrap();
    assert!(
        pose.position[2] < 0.8,
        "pulled in before the barrier: {pose:?}"
    );
    let mut arrived = false;
    for _ in 0..400 {
        let report = run.step(STEP).expect("session steps");
        assert!(
            report.scripts.failures.is_empty(),
            "{:?}",
            report.scripts.failures
        );
        assert!(report.problems.is_empty(), "{:?}", report.problems);
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
    assert!(arrived, "the feature example reaches its goal");
    assert!(!run.world.is_active(barrier));
    let pose = run.world.world_transform(camera).unwrap();
    let target = run.world.world_transform(walker).unwrap().position;
    let focus = [target[0], target[1] + 0.5, target[2]];
    let delta: [f32; 3] = std::array::from_fn(|i| focus[i] - pose.position[i]);
    let distance = delta.into_iter().map(|v| v * v).sum::<f32>().sqrt();
    assert!(
        (distance - 6.0).abs() < 0.03,
        "camera recovered: {distance}"
    );
    assert!(
        pose.forward()
            .into_iter()
            .zip(delta)
            .all(|(a, b)| (a - b / distance).abs() < 1.0e-4)
    );
    let payload = &run.world.get(camera).unwrap().components["sindri.camera"];
    assert!((payload["vertical_fov_degrees"].as_f64().unwrap() - 65.0).abs() < 1.0e-6);
    assert!((payload["near"].as_f64().unwrap() - 0.1).abs() < 1.0e-6);
}

#[test]
fn mouse_drag_changes_orbit_angles_once_and_clamps_pitch() {
    use sindri_platform::{InputEvent, MouseButton};
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/orbit");
    let mut run = ProjectRun::open(&path, [960.0, 540.0]).unwrap();
    let camera = run.entity("camera").expect("authored camera");
    run.input
        .apply(InputEvent::PointerMoved { x: 100.0, y: 100.0 });
    run.step(STEP).unwrap();
    run.input
        .apply(InputEvent::PointerMoved { x: 120.0, y: 110.0 });
    run.step(STEP).unwrap();
    let orbit = &run.world.get(camera).unwrap().components["sindri.camera.orbit"];
    assert!((orbit["pitch"].as_f64().unwrap() - 0.35).abs() < 1.0e-6);
    run.input
        .apply(InputEvent::ButtonPressed(MouseButton::Right));
    run.input
        .apply(InputEvent::PointerMoved { x: 180.0, y: 150.0 });
    let report = run.step(STEP).unwrap();
    assert!(report.scripts.is_quiet(), "{:?}", report.scripts);
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    let orbit = &run.world.get(camera).unwrap().components["sindri.camera.orbit"];
    let yaw = orbit["yaw"].as_f64().unwrap();
    assert!((yaw - (f64::from(STEP) * 0.6 - 0.3)).abs() < 1.0e-6);
    assert!((orbit["pitch"].as_f64().unwrap() - 0.55).abs() < 1.0e-6);
    run.step(STEP).unwrap();
    let orbit = &run.world.get(camera).unwrap().components["sindri.camera.orbit"];
    assert!((orbit["yaw"].as_f64().unwrap() - yaw - f64::from(STEP) * 0.2).abs() < 1.0e-6);
    run.input.apply(InputEvent::PointerMoved {
        x: 180.0,
        y: 10000.0,
    });
    let report = run.step(STEP).unwrap();
    assert!(report.problems.is_empty(), "{:?}", report.problems);
    let orbit = &run.world.get(camera).unwrap().components["sindri.camera.orbit"];
    assert!((orbit["pitch"].as_f64().unwrap() - 1.2).abs() < 1.0e-6);
}

#[test]
fn capture_intent_is_drained_once_and_relative_motion_drives_the_camera() {
    use sindri_platform::{InputEvent, Key, MouseButton};
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/orbit");
    let mut run = ProjectRun::open(&path, [960.0, 540.0]).unwrap();
    run.step(STEP).unwrap();
    run.input
        .apply(InputEvent::PointerMoved { x: 100.0, y: 100.0 });
    run.input
        .apply(InputEvent::ButtonPressed(MouseButton::Left));
    let report = run.step(STEP).unwrap();
    assert_eq!(report.scripts.pointer_lock_request, Some(true));
    assert_eq!(run.session.take_pointer_lock_request(), Some(true));
    assert_eq!(run.session.take_pointer_lock_request(), None);
    assert!(!run.input.pointer_locked());
    run.input.apply(InputEvent::PointerLockChanged(true));
    run.input
        .apply(InputEvent::PointerMotion { x: 60.0, y: 40.0 });
    let report = run.step(STEP).unwrap();
    assert!(report.notes().is_empty(), "{report:?}");
    let camera = run.entity("camera").unwrap();
    let orbit = &run.world.get(camera).unwrap().components["sindri.camera.orbit"];
    assert!((orbit["pitch"].as_f64().unwrap() - 0.55).abs() < 1.0e-6);
    run.input.apply(InputEvent::KeyPressed(Key::U));
    run.step(STEP).unwrap();
    let checkpoint = run.session.checkpoint();
    assert_eq!(run.session.take_pointer_lock_request(), Some(false));
    run.session.restore(&checkpoint);
    assert_eq!(
        run.session.take_pointer_lock_request(),
        None,
        "restoring does not replay window commands"
    );
}
