"""Make the shadow-disabled control project for a same-host render comparison."""
import json
from pathlib import Path
import shutil
import sys

root = Path(__file__).resolve().parents[2]
out = Path(sys.argv[1])
shutil.copytree(root / "examples/benchmark3d", out, dirs_exist_ok=True)
path = out / "assets/benchmark.scene"
scene = json.loads(path.read_text())
for entity in scene["entities"]:
    environment = entity["components"].get("sindri.environment")
    if environment is not None:
        environment["shadows"]["enabled"] = False
path.write_text(json.dumps(scene, indent=2) + "\n")
