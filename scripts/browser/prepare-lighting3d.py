"""The authored lighting example, with only its scripted final-lighting flag off."""
import json
from pathlib import Path
import shutil
import sys

root = Path(__file__).resolve().parents[2]
out = Path(sys.argv[1])
shutil.copytree(root / "examples/lighting3d", out, dirs_exist_ok=True)
path = out / "assets/lighting.scene"
scene = json.loads(path.read_text())
camera = next(e for e in scene["entities"] if e["id"] == "Camera")
camera["components"]["sindri.script"]["properties"]["lit"] = False
path.write_text(json.dumps(scene, indent=2) + "\n")
