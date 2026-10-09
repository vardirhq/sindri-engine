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
