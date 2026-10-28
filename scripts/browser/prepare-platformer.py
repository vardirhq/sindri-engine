#!/usr/bin/env python3
"""Copy the shipped platformer so it opens on its first level.

    prepare-platformer.py <destination> [--without-observer]

The shipped game opens on its title screen; the browser goal run and the batch
benchmark both want the level, so the copy's manifest names it as the main
scene. By default a read-only observer is attached for the browser test too;
`--without-observer` leaves the level exactly as shipped, for the benchmark."""
import json
from pathlib import Path
import shutil
import sys

repository = Path(__file__).resolve().parents[2]
project = repository / "games/platformer"
arguments = [argument for argument in sys.argv[1:] if not argument.startswith("--")]
observe = "--without-observer" not in sys.argv[1:]
destination = Path(arguments[0])
destination.mkdir(parents=True, exist_ok=True)
shutil.copytree(project / "assets", destination / "assets", dirs_exist_ok=True)
# It stays listed among the scenes besides.
manifest = (project / "sindri.toml").read_text()
assert 'main_scene = "assets/title.scene"' in manifest, "the shipped game opens on its title"
(destination / "sindri.toml").write_text(
    manifest.replace('main_scene = "assets/title.scene"', 'main_scene = "assets/level-1.scene"')
)
if observe:
    shutil.copyfile(project / "tests/browser_goal.decay", destination / "assets/scripts/browser_goal.decay")
    scene_path = destination / "assets/level-1.scene"
    scene = json.loads(scene_path.read_text())
    scene["entities"].append({
        "id": "browser-goal-observer",
        "name": "Browser goal observer",
        "components": {
            "sindri.script": {
                "source": "scripts/browser_goal.decay",
                "script": "BrowserGoalObserver",
                "properties": {},
            }
        },
    })
    scene_path.write_text(json.dumps(scene, indent=2) + "\n")
