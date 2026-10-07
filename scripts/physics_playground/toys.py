"""The contraptions, one function each. Coordinates are the room's: x from
-30 to 30 and y from -14 (the floor) to 14 (the ceiling)."""

from __future__ import annotations

from scene import Scene


def loose_pile(scene: Scene) -> None:
    """Something to throw around while the contraptions are being built."""
    colors = ["#ff8a3d", "#ffd166", "#2ec4b6", "#ff5d8f", "#9b5de5"]
    for i in range(10):
        scene.crate(f"pile-crate-{i}", -4 + (i % 5) * 1.0, -13.5 + (i // 5) * 1.0, 0.9, 0.9,
                    0, colors[i % 5], "#ffffff")
    for i in range(12):
        scene.ball(f"pile-ball-{i}", 4 + (i % 6) * 0.8, -10 + (i // 6) * 0.8, 0.35, 0,
                   colors[(i + 2) % 5], "#ffffff")


def build(scene: Scene) -> None:
    loose_pile(scene)
