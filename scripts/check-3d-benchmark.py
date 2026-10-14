#!/usr/bin/env python3
"""Verify physical counters in shadowed/unshadowed standalone benchmark reports."""

import json
import sys
from pathlib import Path


def check(path: str, shadows: bool, require_timing: bool = False) -> None:
    report = json.loads(Path(path).read_text())
    assert not report.get("errors"), report.get("errors")
    frames = report["sections"]["playing"]
    assert len(frames) >= 2, "need consecutive frames to prove snapshot deltas"
    expected = {
        "draw_calls": 502 if shadows else 301,
        "render_passes": 503 if shadows else 301,
        "uniform_writes": 502 if shadows else 301,
        # Color uniforms: 201 * 320 + 100 * 400; shadow matrices: 201 * 64.
        "uniform_bytes": 117184 if shadows else 104320,
        "transient_geometry_uploads": 200 if shadows else 100,
        "model_uploads": 0,
        "cached_geometry_uploads": 0,
    }
    if require_timing:
        assert report["gpu_timing"]["supported"], "software-Vulkan CI requires timestamp support"
    for frame in frames:
        if require_timing:
            assert frame["gpu_execution_us"] > 0, "need a measured GPU interval"
        assert frame["mesh_counters"] == expected, (path, frame["mesh_counters"], expected)
        assert frame["batch_counters"] == dict.fromkeys(
            ["draw_calls", "render_passes", "instances", "buffer_writes", "uniform_bytes", "instance_bytes"], 0
        ), "the fixed 3D workload has no 2D batches"
    print(f"{path}: {len(frames)} frames match physical 3D counters")


if __name__ == "__main__":
    require = len(sys.argv) == 4 and sys.argv[3] == "--require-gpu-timing"
    check(sys.argv[1], True, require)
    check(sys.argv[2], False, require)
