#!/usr/bin/env python3
"""Copy the shipped platformer and attach a read-only browser-test observer."""
import json
from pathlib import Path
import shutil
import sys

repository = Path(__file__).resolve().parents[2]
project = repository / "games/platformer"
destination = Path(sys.argv[1])
destination.mkdir(parents=True, exist_ok=True)
shutil.copytree(project / "assets", destination / "assets", dirs_exist_ok=True)
shutil.copyfile(project / "sindri.toml", destination / "sindri.toml")
shutil.copyfile(project / "tests/browser_goal.decay", destination / "assets/scripts/browser_goal.decay")
scene_path = destination / "assets/platformer.scene"
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
