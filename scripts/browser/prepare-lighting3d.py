"""Lighting Lab controls: final lights off, or its back-facing panel single-sided."""
import argparse
import json
from pathlib import Path
import shutil

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("out", type=Path)
parser.add_argument("--single-sided", action="store_true")
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
out = args.out
shutil.copytree(root / "examples/lighting3d", out, dirs_exist_ok=True)
path = out / "assets/lighting.scene"
scene = json.loads(path.read_text())
if args.single_sided:
    panel = next(e for e in scene["entities"] if e["id"] == "Backface-double")
    panel["components"]["sindri.material"]["sidedness"] = "single"
else:
    camera = next(e for e in scene["entities"] if e["id"] == "Camera")
    camera["components"]["sindri.script"]["properties"]["lit"] = False
path.write_text(json.dumps(scene, indent=2) + "\n")
