"""Copy Day/Night Lab and author isolated hour, shadow and night-light controls."""
import argparse
import json
from pathlib import Path
import shutil

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
parser.add_argument("--hour", type=float, default=-1)
parser.add_argument("--stationary", action="store_true")
parser.add_argument("--no-shadows", action="store_true")
parser.add_argument("--no-night", action="store_true")
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
shutil.copytree(root / "examples/daynight", args.output, dirs_exist_ok=True)
path = args.output / "assets/daynight.scene"
scene = json.loads(path.read_text())
camera = next(e for e in scene["entities"] if e["id"] == "Camera")
camera["components"]["sindri.script"]["properties"].update(
    preview_hour=args.hour, move_carrier=not args.stationary, night_lights=not args.no_night)
world = next(e for e in scene["entities"] if e["id"] == "World")
world["components"]["sindri.environment"]["shadows"]["enabled"] = not args.no_shadows
path.write_text(json.dumps(scene, indent=2) + "\n")
