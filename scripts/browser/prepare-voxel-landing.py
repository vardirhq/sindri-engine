#!/usr/bin/env python3
"""Copy Voxel Lab, give its world collision, and attach a read-only observer
that drops a block onto it: the browser proof that voxel collision works."""
import json
from pathlib import Path
import shutil
import sys

repository = Path(__file__).resolve().parents[2]
project = repository / "games" / "voxel-lab"
fixture = Path(__file__).resolve().parent / "voxel-landing"
destination = Path(sys.argv[1])
destination.mkdir(parents=True, exist_ok=True)
shutil.copytree(project / "assets", destination / "assets", dirs_exist_ok=True)
shutil.copyfile(project / "sindri.toml", destination / "sindri.toml")
(destination / "assets/scripts").mkdir(exist_ok=True)
(destination / "assets/prefabs").mkdir(exist_ok=True)
shutil.copyfile(fixture / "observer.decay", destination / "assets/scripts/voxel-landing.decay")
shutil.copyfile(fixture / "block.prefab", destination / "assets/prefabs/falling-block.prefab")
scene_path = destination / "assets/voxel-lab.scene"
scene = json.loads(scene_path.read_text())
for entity in scene["entities"]:
    if entity["id"] == "voxel-world":
        entity["components"]["sindri.physics3d.voxel_collider"] = {
            "friction": 0.8,
            "layers": {"filter": 4294967295, "memberships": 4294967295},
            "margin": 2.0,
            "restitution": 0.0,
        }
scene["entities"].append({
    "id": "physics",
    "name": "Physics",
    "components": {"sindri.physics3d.world": {"gravity": [0.0, -9.81, 0.0], "layers": ["solid"]}},
})
scene["entities"].append({
    "id": "browser-voxel-landing",
    "name": "Browser voxel landing",
    "components": {
        "sindri.script": {
            "source": "scripts/voxel-landing.decay",
            "script": "BrowserVoxelLanding",
            "properties": {"block": "prefabs/falling-block.prefab"},
        }
    },
})
scene_path.write_text(json.dumps(scene, indent=2) + "\n")
