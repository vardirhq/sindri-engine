"""Aim ranged spotlights toward/away from the checked 3D swatches."""
import json
from pathlib import Path
import subprocess
import sys

root = Path(__file__).resolve().parents[2]
out = Path(sys.argv[1])
subprocess.run([sys.executable, str(root / "scripts/browser/prepare-point-lights.py"), str(out)], check=True)
path = out / "assets/benchmark.scene"
scene = json.loads(path.read_text())
for entity in scene["entities"]:
    light = entity.get("components", {}).get("sindri.light", {})
    if light.get("kind") == "point":
        light.update(kind="spot", inner_angle=0.35, outer_angle=0.75)
        # The parent is turned/scaled. Down is invariant under that yaw;
        # reversing the local pitch points into the empty sky instead.
        entity["transform_3d"]["rotation"] = [0.70710677 if "--away" in sys.argv else -0.70710677, 0, 0, 0.70710677]
        entity["name"] = entity["name"].replace("point", "spot")
path.write_text(json.dumps(scene, indent=2) + "\n")
