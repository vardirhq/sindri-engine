#!/usr/bin/env python3
"""Copy Causeway and attach a read-only browser voxel-collision observer."""
import json
from pathlib import Path
import shutil
import sys

repository = Path(__file__).resolve().parents[2]
project = repository / "game"
destination = Path(sys.argv[1])
destination.mkdir(parents=True, exist_ok=True)
shutil.copytree(project / "assets", destination / "assets", dirs_exist_ok=True)
shutil.copyfile(project / "sindri.toml", destination / "sindri.toml")
shutil.copyfile(
    project / "tests/browser_voxel_landing.decay",
    destination / "assets/scripts/browser_voxel_landing.decay",
)
scene_path = destination / "assets/causeway.scene"
scene = json.loads(scene_path.read_text())
scene["entities"].append({
    "id": "browser-voxel-landing",
    "name": "Browser voxel landing",
    "components": {
        "sindri.script": {
            "source": "scripts/browser_voxel_landing.decay",
            "script": "BrowserVoxelLanding",
            "properties": {"block": "prefabs/loose-block.prefab"},
        }
    },
})
scene_path.write_text(json.dumps(scene, indent=2) + "\n")
