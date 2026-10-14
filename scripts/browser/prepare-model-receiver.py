"""Make a small 3D Benchmark control with a green imported shadow receiver.

This uses the existing checked benchmark script/goal and Model Collision Lab's
unchanged GLB. Only the blue textured cube casts a shadow; the green ground is
imported geometry, so green-pixel comparisons prove model reception specifically.
"""
import json
import math
from pathlib import Path
import shutil
import sys

root = Path(__file__).resolve().parents[2]
out = Path(sys.argv[1])
shutil.copytree(root / "examples/benchmark3d", out, dirs_exist_ok=True)
shutil.copyfile(root / "examples/model-collision/assets/models/level.glb",
                out / "assets/models/prop.glb")
path = out / "assets/benchmark.scene"
scene = json.loads(path.read_text())
scene["entities"] = scene["entities"][:4]
camera, sun, world, floor = scene["entities"]
camera["transform_3d"]["position"] = [8, 9, 12]
# Aim shadows toward the camera so reception is visible on the open ground.
sy, cy = math.sin(math.radians(-60)), math.cos(math.radians(-60))
sp, cp = math.sin(math.radians(-20)), math.cos(math.radians(-20))
sun["transform_3d"]["rotation"] = [sp * cy, cp * sy, -sp * sy, cp * cy]
world["components"]["sindri.environment"]["shadows"].update(
    enabled="--no-shadows" not in sys.argv, distance=16, bias=0.001)
world["components"]["sindri.environment"]["ambient_intensity"] = 0.25
floor["transform_3d"] = {"position": [0, 0, 0], "scale": [1, 1, 1]}
floor["components"] = {"sindri.model": {"asset": "models/prop.glb"}}
scene["entities"].append({"id": "Caster", "name": "Caster",
    "transform_3d": {"position": [-2, 1, 0], "scale": [1, 2, 1]},
    "components": {"sindri.mesh": {"primitive": "cube", "texture": "textures/cube.png"}}})
path.write_text(json.dumps(scene, indent=2) + "\n")
