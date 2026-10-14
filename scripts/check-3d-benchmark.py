#!/usr/bin/env python3
"""Verify physical counters in shadowed/unshadowed standalone benchmark reports."""

import json
import sys
from pathlib import Path


def check(path: str, shadows: bool) -> None:
    report = json.loads(Path(path).read_text())
    frames = report["sections"]["playing"]
    assert len(frames) >= 2, "need consecutive frames to prove snapshot deltas"
    expected = {
        "draw_calls": 502 if shadows else 301,
        "render_passes": 503 if shadows else 301,
        "uniform_writes": 502 if shadows else 301,
        # Color uniforms: 201 * 320 + 100 * 288; shadow matrices: 201 * 64.
        "uniform_bytes": 105984 if shadows else 93120,
        "transient_geometry_uploads": 200 if shadows else 100,
        "model_uploads": 0,
        "cached_geometry_uploads": 0,
    }
    for frame in frames:
        assert frame["mesh_counters"] == expected, (path, frame["mesh_counters"], expected)
    print(f"{path}: {len(frames)} frames match physical 3D counters")


if __name__ == "__main__":
    check(sys.argv[1], True)
    check(sys.argv[2], False)
