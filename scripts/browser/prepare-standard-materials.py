"""Derive authored standard-material swatches from the checked 3D Benchmark."""
import json
import math
from pathlib import Path
import shutil
import sys

root = Path(__file__).resolve().parents[2]
out = Path(sys.argv[1])
shutil.copytree(root / "examples/benchmark3d", out, dirs_exist_ok=True)
for texture in ["cube.png", "surface.png"]:
    shutil.copyfile(root / "examples/audio/assets/textures/white.png", out / "assets/textures" / texture)
path = out / "assets/benchmark.scene"
scene = json.loads(path.read_text())
by_id = {entity["id"]: entity for entity in scene["entities"]}
camera, sun, world, floor = scene["entities"][:4]
camera["transform_3d"]["position"] = [8, 9, 12]
sy, cy = math.sin(math.atan2(8, 12) / 2), math.cos(math.atan2(8, 12) / 2)
sp, cp = math.sin(-math.asin(9 / 17) / 2), math.cos(-math.asin(9 / 17) / 2)
camera["transform_3d"]["rotation"] = [sp * cy, cp * sy, -sp * sy, cp * cy]
# Light opposite the view horizontally: polished top faces show a highlight.
yaw, pitch = math.atan2(-8, -12), -math.asin(9 / 17)
sy, cy = math.sin(yaw / 2), math.cos(yaw / 2)
sp, cp = math.sin(pitch / 2), math.cos(pitch / 2)
sun["transform_3d"]["rotation"] = [sp * cy, cp * sy, -sp * sy, cp * cy]
sun["components"]["sindri.light"]["intensity"] = 3.0
world["components"]["sindri.environment"]["ambient_intensity"] = 0.2
world["components"]["sindri.environment"]["shadows"].update(distance=20, bias=0.001)
floor["transform_3d"]["scale"] = [12, 0.2, 8]
entities = [camera, sun, world, floor]
for index, (kind, metallic, roughness, emissive) in enumerate([
    ("cube", 0.0, 1.0, [0, 0, 0]), ("cube", 1.0, 1.0, [0, 0, 0]),
    ("cube", 1.0, 0.15, [0, 0, 0]), ("surface", 0.0, 0.4, [0, 0, 0]),
    ("model", 1.0, 0.15, [0, 0, 0]), ("model", 0.0, 1.0, [0.05, 0.35, 1.0]),
]):
    entity = json.loads(json.dumps(by_id[f"{kind}-0-0"]))
    entity["id"] = entity["name"] = f"Swatch-{index}"
    entity["transform_3d"] = {"position": [(index % 3 - 1) * 2.5, 0.75, (index // 3 - 0.5) * 3], "scale": [1.3, 1.5, 1.3]}
    if "--legacy" not in sys.argv:
        entity["components"]["sindri.material"] = {"base_color": [0.85, 0.5, 0.08, 1.0],
            "metallic": metallic, "roughness": roughness, "emissive": emissive}
    entities.append(entity)
scene["entities"] = entities
path.write_text(json.dumps(scene, indent=2) + "\n")
