#!/usr/bin/env python3
"""Times a project in the editor and in the standalone host, and compares them.

    scripts/frame-benchmark.py run games/platformer [--profile P] [--frames N]
    scripts/frame-benchmark.py summarize target/bench/*.json

`run` builds both benchmarks, plays the project in the editor (under
`xvfb-run` when there is no display) and offscreen in the standalone host,
writes each report to `target/bench/`, and prints the comparison. `summarize`
prints the same table for reports that already exist.

The editor's report is `sindri-editor --benchmark` (`editor/src/benchmark.rs`)
and the standalone one is `project-benchmark` (`game/src/bin/`). Both hold raw
frames in microseconds; everything below is worked out from them, so the
statistics can change without measuring again.

What is compared is the CPU **work** of a frame: every phase except the
editor's `waiting` (presenting, vsync, idle) and the standalone host's `gpu`
(waiting for the GPU to finish). The standalone host runs one fixed step a
frame by construction; the editor runs as many as its clock owes, so its
steps per frame are printed beside it — more than one means it is not keeping
up, and each extra step is gameplay paid for twice in one frame.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import statistics
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BENCH = ROOT / "target" / "bench"
# Phases that are not the host working.
IDLE = {"waiting", "gpu"}
# The editor's phases that are one fixed step of gameplay, which the
# standalone host times as one `step`.
GAMEPLAY = ("effects", "physics", "screen_ui", "scripts", "animation", "cameras", "placement")
# The editor's phases that draw the game, which the standalone host times as
# presentation, extraction and encoding.
VIEWS = ("upkeep", "presentation", "extraction", "encoding")


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    if not ordered:
        return 0.0
    index = min(len(ordered) - 1, round(fraction * (len(ordered) - 1)))
    return ordered[index]


def work_ms(frame: dict) -> float:
    return sum(v for k, v in frame["phases"].items() if k not in IDLE) / 1000.0


def interval_ms(frame: dict) -> float:
    return sum(frame["phases"].values()) / 1000.0


def section_stats(frames: list[dict]) -> dict:
    work = [work_ms(f) for f in frames]
    phases: dict[str, float] = {}
    for frame in frames:
        for key, value in frame["phases"].items():
            phases[key] = phases.get(key, 0.0) + value / 1000.0
    count = max(len(frames), 1)
    return {
        "frames": len(frames),
        "work_mean": statistics.fmean(work) if work else 0.0,
        "work_p50": percentile(work, 0.50),
        "work_p95": percentile(work, 0.95),
        "work_max": max(work, default=0.0),
        "interval_mean": statistics.fmean(interval_ms(f) for f in frames) if frames else 0.0,
        "steps": statistics.fmean(f["steps"] for f in frames) if frames else 0.0,
        "phases": {key: total / count for key, total in phases.items()},
    }


def describe(path: Path) -> list[str]:
    report = json.loads(path.read_text())
    build = "optimised" if report.get("optimized") else "debug"
    lines = [f"{path.name}: {report['host']}, {build}, {report['opened']}"]
    for name, frames in report["sections"].items():
        stats = section_stats(frames)
        lines.append(
            f"  {name:8} {stats['frames']:4} frames  work mean {stats['work_mean']:6.2f} ms"
            f"  p50 {stats['work_p50']:6.2f}  p95 {stats['work_p95']:6.2f}"
            f"  max {stats['work_max']:6.2f}  steps/frame {stats['steps']:.2f}"
        )
        ranked = sorted(stats["phases"].items(), key=lambda item: -item[1])
        lines.append(
            "           "
            + "  ".join(f"{key} {value:.2f}" for key, value in ranked if value >= 0.005)
        )
    return lines


def compare(editor: Path, standalone: Path) -> list[str]:
    ours = section_stats(json.loads(editor.read_text())["sections"]["playing"])
    theirs = section_stats(json.loads(standalone.read_text())["sections"]["playing"])
    if not theirs["work_mean"]:
        return ["  (the standalone report has no frames to compare against)"]
    ratio = ours["work_mean"] / theirs["work_mean"]
    gameplay = sum(ours["phases"].get(key, 0.0) for key in GAMEPLAY)
    per_step = gameplay / ours["steps"] if ours["steps"] else 0.0
    one_step = one_step_frame(ours)
    return [
        f"  Play: editor {ours['work_mean']:.2f} ms/frame "
        f"({ours['steps']:.2f} steps), standalone {theirs['work_mean']:.2f} ms/frame: "
        f"{ratio:.2f}x",
        f"  Gameplay per step: editor {per_step:.2f} ms, "
        f"standalone {theirs['phases'].get('step', 0.0):.2f} ms",
        f"  One-step game frame: editor {one_step:.2f} ms, "
        f"standalone {theirs['work_mean']:.2f} ms: {one_step / theirs['work_mean']:.2f}x; "
        f"editor's own UI (panels + paint) {editor_ui(ours):.2f} ms",
    ]


def one_step_frame(stats: dict) -> float:
    """What the editor spends on the game in a frame that ran one step: its
    views and upkeep, and one step's gameplay. This is what the standalone
    host's frame is the equivalent of, without the catch-up steps a slow
    frame owes, which make a slow editor slower still."""
    phases = stats["phases"]
    gameplay = sum(phases.get(key, 0.0) for key in GAMEPLAY)
    per_step = gameplay / stats["steps"] if stats["steps"] else 0.0
    return sum(phases.get(key, 0.0) for key in VIEWS) + per_step


def editor_ui(stats: dict) -> float:
    return sum(stats["phases"].get(key, 0.0) for key in ("panels", "paint"))


def target_dir(profile: str) -> Path:
    """Where cargo puts a profile's binaries: `dev` builds into `debug`."""
    return ROOT / "target" / ("debug" if profile == "dev" else profile)


def run(args: argparse.Namespace) -> int:
    project = args.project.rstrip("/")
    name = Path(project).name
    build = args.profile
    BENCH.mkdir(parents=True, exist_ok=True)
    profile = ["--profile", args.profile]
    subprocess.run(["cargo", "build", "-p", "sindri-editor", *profile], cwd=ROOT, check=True)
    subprocess.run(
        ["cargo", "build", "-p", "sindri-causeway", "--bin", "project-benchmark", *profile],
        cwd=ROOT,
        check=True,
    )
    editor_report = BENCH / f"{name}-editor-{build}.json"
    command = [
        str(target_dir(args.profile) / "sindri-editor"),
        project,
        "--benchmark",
        str(editor_report),
        "--frames",
        str(args.frames),
        "--settle",
        str(args.settle),
    ]
    if not os.environ.get("DISPLAY") and not os.environ.get("WAYLAND_DISPLAY"):
        if not shutil.which("xvfb-run"):
            print("no display and no xvfb-run to make one", file=sys.stderr)
            return 1
        command = [
            "xvfb-run",
            "--auto-servernum",
            "--server-args=-screen 0 1600x1200x24",
            *command,
        ]
    subprocess.run(command, cwd=ROOT, check=True, timeout=args.timeout)
    standalone_report = BENCH / f"{name}-standalone-{build}.json"
    standalone = target_dir(args.profile) / "project-benchmark"
    subprocess.run(
        [str(standalone), project, str(standalone_report), str(args.frames), str(args.settle)],
        cwd=ROOT,
        check=True,
        timeout=args.timeout,
    )
    for line in describe(editor_report) + describe(standalone_report):
        print(line)
    for line in compare(editor_report, standalone_report):
        print(line)
    return 0


def summarize(args: argparse.Namespace) -> int:
    paths = [Path(path) for path in args.reports]
    for path in paths:
        for line in describe(path):
            print(line)
    editors = {p.name.replace("-editor-", "|"): p for p in paths if "-editor-" in p.name}
    for key, editor in editors.items():
        standalone = editor.with_name(key.replace("|", "-standalone-"))
        if standalone.exists():
            print(f"{editor.name} against {standalone.name}")
            for line in compare(editor, standalone):
                print(line)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    running = commands.add_parser("run", help="measure a project in both hosts")
    running.add_argument("project")
    running.add_argument(
        "--profile",
        default="dev",
        help="the cargo profile to build and measure: dev, editor or release",
    )
    running.add_argument("--frames", type=int, default=600)
    running.add_argument("--settle", type=int, default=60)
    running.add_argument("--timeout", type=int, default=900, help="seconds per host")
    running.set_defaults(handler=run)
    summarizing = commands.add_parser("summarize", help="compare existing reports")
    summarizing.add_argument("reports", nargs="+")
    summarizing.set_defaults(handler=summarize)
    args = parser.parse_args()
    return args.handler(args)


if __name__ == "__main__":
    sys.exit(main())
