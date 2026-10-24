#!/usr/bin/env python3
"""Verify physical batch accounting while Lamplighter's transient sprites vary."""

import json
import sys
from pathlib import Path

# The settled level has six non-sprite batches: the four it had, and the pause
# button's box and its "II". Their payload is 2,912 bytes smaller than storing
# every instance as a sprite.
# Sprite counts vary with particle lifetime: standalone advances one fixed step
# per draw, whereas the editor advances however many steps its clock owes.
NON_SPRITE_BATCHES = 6
NON_SPRITE_BYTE_SAVING = 2912
SPRITE_BYTES = 176

for path in sys.argv[1:]:
    report = json.loads(Path(path).read_text())
    assert not report.get("errors"), report.get("errors")
    frames = report["sections"]["playing"]
    assert len(frames) >= 2, "need consecutive frames to prove snapshot deltas"
    for frame in frames:
        actual = frame["batch_counters"]
        draws = actual["draw_calls"]
        instances = actual["instances"]
        assert draws > NON_SPRITE_BATCHES and instances > 0, (path, actual)
        expected = {
            "draw_calls": draws,
            "render_passes": draws,
            "instances": instances,
            "buffer_writes": 2 * draws,
            "uniform_bytes": 160 * (draws - NON_SPRITE_BATCHES) + 64 * NON_SPRITE_BATCHES,
            "instance_bytes": SPRITE_BYTES * instances - NON_SPRITE_BYTE_SAVING,
        }
        assert actual == expected, (path, actual, expected)
    print(f"{path}: {len(frames)} frames match physical 2D batch accounting")
