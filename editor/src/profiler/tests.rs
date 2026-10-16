use std::time::{Duration, Instant};

use sindri_decay::ScriptTiming;

use super::{KEPT, Phase, Profiler};

fn timing(script: &str, millis: u64) -> ScriptTiming {
    ScriptTiming {
        source: "game.decay".to_owned(),
        script: script.to_owned(),
        runs: 1,
        time: Duration::from_millis(millis),
    }
}

const fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// Runs one frame: `work` is measured inside a `ui` that takes 10 ms, eframe
/// reports 12 ms for the frame when the next begins, and the next begins
/// 20 ms after this one did.
fn frame(
    profiler: &mut Profiler,
    at: &mut Instant,
    playing: bool,
    work: impl FnOnce(&mut Profiler),
) {
    profiler.begin(*at, Some(ms(12)), playing);
    work(profiler);
    profiler.end(ms(10));
    *at += ms(20);
}

#[test]
fn a_frame_adds_up_its_steps_and_a_frame_that_did_nothing_is_not_kept() {
    let mut profiler = Profiler::default();
    let mut at = Instant::now();
    frame(&mut profiler, &mut at, true, |profiler| {
        profiler.add(Phase::Physics, ms(1));
        profiler.step(vec![timing("Boss", 3)]);
        profiler.add(Phase::Physics, ms(2));
        profiler.step(vec![timing("Boss", 4), timing("Wisp", 1)]);
    });
    // Editing, with nothing stepped: not kept.
    frame(&mut profiler, &mut at, false, |_| {});
    profiler.begin(at, Some(ms(12)), false);
    assert_eq!(profiler.frames().len(), 1, "the editing frame is dropped");
    let frame = &profiler.frames()[0];
    assert_eq!(frame.steps, 2);
    assert_eq!(frame.phase(Phase::Physics), ms(3));
    assert_eq!(frame.scripts[0].runs, 2);
    assert_eq!(frame.scripts[0].time, ms(7));
}

#[test]
fn the_phases_a_frame_cannot_time_are_worked_out_when_it_closes() {
    let mut profiler = Profiler::default();
    let mut at = Instant::now();
    frame(&mut profiler, &mut at, true, |profiler| {
        profiler.add(Phase::Scripts, ms(3));
        profiler.add(Phase::Extraction, ms(2));
    });
    profiler.begin(at, Some(ms(12)), true);
    let frame = &profiler.frames()[0];
    // `ui` took 10, of which 5 was measured inside it.
    assert_eq!(frame.phase(Phase::Panels), ms(5));
    // eframe said 12, so painting took the other 2.
    assert_eq!(frame.phase(Phase::Paint), ms(2));
    // The interval was 20, of which 12 was spent.
    assert_eq!(frame.phase(Phase::Waiting), ms(8));
    assert_eq!(frame.total(), ms(20), "the phases add up to the interval");
    assert_eq!(frame.work(), ms(12));
}

#[test]
fn a_frame_eframe_did_not_report_is_all_the_editors_own() {
    let mut profiler = Profiler::default();
    let at = Instant::now();
    profiler.begin(at, None, true);
    profiler.add(Phase::Scripts, ms(4));
    profiler.end(ms(6));
    profiler.begin(at + ms(16), None, true);
    let frame = &profiler.frames()[0];
    assert_eq!(frame.phase(Phase::Paint), Duration::ZERO);
    assert_eq!(frame.phase(Phase::Panels), ms(2));
    assert_eq!(frame.phase(Phase::Waiting), ms(10));
}

#[test]
fn frames_while_editing_are_kept_only_when_asked_for() {
    let mut profiler = Profiler::default();
    let mut at = Instant::now();
    profiler.set_while_editing(true);
    frame(&mut profiler, &mut at, false, |profiler| {
        profiler.add(Phase::Upkeep, ms(1));
    });
    profiler.begin(at, Some(ms(12)), false);
    assert_eq!(profiler.frames().len(), 1);
    assert_eq!(profiler.frames()[0].steps, 0);
}

#[test]
fn a_capture_keeps_every_frame_past_the_limit() {
    let mut profiler = Profiler::default();
    let mut at = Instant::now();
    profiler.capture();
    for _ in 0..KEPT + 5 {
        frame(&mut profiler, &mut at, true, |profiler| {
            profiler.add(Phase::Effects, Duration::from_micros(1));
        });
    }
    profiler.begin(at, Some(ms(12)), true);
    assert_eq!(profiler.frames().len(), KEPT);
    assert_eq!(profiler.captured_len(), KEPT + 5);
    assert_eq!(profiler.take_captured().len(), KEPT + 5);
    assert_eq!(profiler.captured_len(), 0, "taking ends the capture");
}

#[test]
fn the_summary_averages_and_ranks_the_slowest_script_first() {
    let mut profiler = Profiler::default();
    let mut at = Instant::now();
    for (boss, wisp) in [(1, 3), (3, 3)] {
        frame(&mut profiler, &mut at, true, |profiler| {
            profiler.add(Phase::Scripts, ms(boss + wisp));
            profiler.step(vec![timing("Boss", boss), timing("Wisp", wisp)]);
        });
    }
    profiler.begin(at, Some(ms(12)), true);
    let summary = profiler.summary();
    assert_eq!(summary.frames, 2);
    // Each frame worked 12 of its 20: eframe's figure covers the scripts.
    assert_eq!(summary.average, ms(12));
    assert_eq!(summary.interval, ms(20));
    assert_eq!(summary.phase(Phase::Scripts), ms(5));
    let ranked: Vec<&str> = summary.scripts.iter().map(|t| t.script.as_str()).collect();
    assert_eq!(ranked, ["Wisp", "Boss"]);
    assert_eq!(summary.scripts[1].time, ms(2));
    assert_eq!(summary.scripts[1].runs, 1, "runs a frame, not in all");
}

#[test]
fn clearing_forgets_the_open_frame_too() {
    let mut profiler = Profiler::default();
    let mut at = Instant::now();
    frame(&mut profiler, &mut at, true, |profiler| {
        profiler.add(Phase::Scripts, ms(1));
    });
    profiler.clear();
    profiler.begin(at, Some(ms(12)), true);
    assert!(profiler.frames().is_empty());
}

#[test]
fn view_counters_follow_frame_capture_without_mixing_scene_and_game() {
    let mut profiler = Profiler::default();
    let mut at = Instant::now();
    profiler.capture();
    frame(&mut profiler, &mut at, true, |profiler| {
        for (beside, draws, uploads) in [(false, 2, 1), (true, 9, 3), (false, 4, 0)] {
            profiler.set_scene_beside_run(beside);
            profiler.add_gpu_execution(ms(draws));
            profiler.add_batch_counters(sindri_render::BatchDrawStats {
                draw_calls: draws,
                instances: draws * 2,
                ..Default::default()
            });
            profiler.add_mesh_counters(super::MeshCounters {
                meshes: sindri_render::MeshDrawStats {
                    draw_calls: draws,
                    render_passes: draws + 1,
                    uniform_writes: draws,
                    uniform_bytes: draws * 64,
                    transient_geometry_uploads: draws,
                },
                model_uploads: uploads,
                cached_geometry_uploads: 1,
            });
        }
        profiler.set_scene_beside_run(false);
    });
    profiler.begin(at, None, true);
    let captured = profiler.take_captured();
    assert_eq!(captured[0].gpu_execution, Some(ms(6)));
    assert_eq!(captured[0].scene_gpu_execution, Some(ms(9)));
    assert_eq!(
        captured[0].work(),
        ms(10),
        "GPU intervals stay outside CPU phases"
    );
    assert_eq!(captured[0].batch_counters.unwrap().draw_calls, 6);
    assert_eq!(captured[0].batch_counters.unwrap().instances, 12);
    assert_eq!(captured[0].scene_batch_counters.unwrap().draw_calls, 9);
    let primary = captured[0].mesh_counters.unwrap();
    assert_eq!(primary.meshes.draw_calls, 6);
    assert_eq!(primary.meshes.render_passes, 8);
    assert_eq!(primary.meshes.uniform_writes, 6);
    assert_eq!(primary.meshes.uniform_bytes, 384);
    assert_eq!(primary.meshes.transient_geometry_uploads, 6);
    assert_eq!(primary.model_uploads, 1);
    assert_eq!(primary.cached_geometry_uploads, 2);
    assert_eq!(
        captured[0].scene_mesh_counters.unwrap().meshes.draw_calls,
        9
    );
    assert_eq!(captured[0].scene_mesh_counters.unwrap().model_uploads, 3);
    assert_eq!(profiler.frames()[0], captured[0]);
    profiler.end(ms(1));
    profiler.begin(at + ms(20), None, true);
    assert!(
        profiler.frames()[1].mesh_counters.is_none(),
        "undrawn is not measured zero"
    );
    assert!(profiler.frames()[1].scene_mesh_counters.is_none());
    assert!(profiler.frames()[1].gpu_execution.is_none());
    assert!(profiler.frames()[1].scene_gpu_execution.is_none());
    assert!(profiler.frames()[1].batch_counters.is_none());
    assert!(profiler.frames()[1].scene_batch_counters.is_none());
    profiler.add_mesh_counters(super::MeshCounters::default());
    profiler.add_gpu_execution(ms(4));
    profiler.add_batch_counters(sindri_render::BatchDrawStats::default());
    profiler.clear();
    profiler.begin(at + ms(40), None, true);
    profiler.end(ms(1));
    profiler.begin(at + ms(60), None, true);
    assert!(
        profiler.frames()[0].mesh_counters.is_none(),
        "Play reset clears counters"
    );
    assert!(profiler.frames()[0].gpu_execution.is_none());
    assert!(profiler.frames()[0].batch_counters.is_none());
}
