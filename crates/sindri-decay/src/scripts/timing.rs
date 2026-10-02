//! Timing a pass of scripts, for a host that asks.

use crate::{ScriptComponent, ScriptReport};

/// Runs one entity's tick, adding how long it took to its script's timing
/// when there is a report to time into.
pub(super) fn timed<T>(
    report: Option<&mut ScriptReport>,
    component: &ScriptComponent,
    run: impl FnOnce() -> T,
) -> T {
    let Some(report) = report else {
        return run();
    };
    let began = web_time::Instant::now();
    let outcome = run();
    let time = began.elapsed();
    match report
        .timings
        .iter_mut()
        .find(|timing| timing.source == component.source && timing.script == component.script)
    {
        Some(timing) => {
            timing.runs += 1;
            timing.time += time;
        }
        None => report.timings.push(crate::ScriptTiming {
            source: component.source.clone(),
            script: component.script.clone(),
            runs: 1,
            time,
        }),
    }
    outcome
}
