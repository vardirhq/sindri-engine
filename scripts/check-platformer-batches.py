#!/usr/bin/env python3
"""Keep Platformer's settled native editor/standalone batch counts comparable."""

import json
import sys
from pathlib import Path

EXPECTED = {
    "draw_calls": 9,
    "render_passes": 9,
    "instances": 295,
    "buffer_writes": 18,
    "uniform_bytes": 576,
    "instance_bytes": 41968,
}

for path in sys.argv[1:]:
    report = json.loads(Path(path).read_text())
    assert not report.get("errors"), report.get("errors")
    frames = report["sections"]["playing"]
    assert len(frames) >= 2, "need consecutive frames to prove snapshot deltas"
    for frame in frames:
        assert frame["batch_counters"] == EXPECTED, (path, frame["batch_counters"])
    print(f"{path}: {len(frames)} frames match physical 2D batch counters")
